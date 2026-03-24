//! TLS ingest transport layer (`transport-tls` feature).
//!
//! Wraps the same `POST /api/v1/ingest` JSON protocol used by the plain HTTP
//! transport in a rustls TLS 1.2/1.3 channel.
//!
//! # Network policy
//!
//! The IP allowlist from [`NetworkPolicy`] is enforced at the TCP accept step,
//! *before* the TLS handshake, so that blocked sources never consume CPU for
//! cryptographic negotiation.
//!
//! # Usage
//!
//! ```no_run
//! use edgesentry_audit::{
//!     AsyncIngestService, AsyncInMemoryRawDataStore, AsyncInMemoryAuditLedger,
//!     AsyncInMemoryOperationLog, IntegrityPolicyGate, NetworkPolicy,
//! };
//! use edgesentry_audit::transport::tls::{TlsConfig, serve_tls};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//! let service = AsyncIngestService::new(
//!     IntegrityPolicyGate::new(),
//!     AsyncInMemoryRawDataStore::default(),
//!     AsyncInMemoryAuditLedger::default(),
//!     AsyncInMemoryOperationLog::default(),
//! );
//!
//! let mut network_policy = NetworkPolicy::new();
//! network_policy.allow_cidr("10.0.0.0/8").unwrap();
//!
//! let tls = TlsConfig::from_pem_files("/etc/edgesentry/cert.pem", "/etc/edgesentry/key.pem");
//! serve_tls(service, network_policy, "0.0.0.0:8443".parse()?, tls).await?;
//! # Ok(())
//! # }
//! ```

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Json, State};
use axum::http::StatusCode;
use axum::routing::post;
use axum::Router;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as ConnBuilder;
use hyper_util::service::TowerToHyperService;
use rustls_pemfile::{certs, private_key};
use tokio_rustls::TlsAcceptor;
use tracing::{info, warn};

use crate::ingest::{
    AsyncAuditLedger, AsyncIngestService, AsyncOperationLogStore, AsyncRawDataStore, NetworkPolicy,
};
use crate::transport::http::{IngestRequest, IngestResponse};

// ── per-connection state ──────────────────────────────────────────────────────

struct TlsState<R, L, O>
where
    R: AsyncRawDataStore + 'static,
    L: AsyncAuditLedger + 'static,
    O: AsyncOperationLogStore + 'static,
{
    service: Arc<AsyncIngestService<R, L, O>>,
}

impl<R, L, O> Clone for TlsState<R, L, O>
where
    R: AsyncRawDataStore + 'static,
    L: AsyncAuditLedger + 'static,
    O: AsyncOperationLogStore + 'static,
{
    fn clone(&self) -> Self {
        Self { service: Arc::clone(&self.service) }
    }
}

// ── ingest handler ────────────────────────────────────────────────────────────

// Network policy is enforced at TCP accept time so the handler does not need
// to check the source IP.
async fn tls_ingest_handler<R, L, O>(
    State(state): State<TlsState<R, L, O>>,
    Json(req): Json<IngestRequest>,
) -> (StatusCode, Json<IngestResponse>)
where
    R: AsyncRawDataStore + Send + Sync + 'static,
    L: AsyncAuditLedger + Send + Sync + 'static,
    O: AsyncOperationLogStore + Send + Sync + 'static,
{
    let payload = match hex::decode(&req.raw_payload_hex) {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(IngestResponse {
                    status: "rejected".into(),
                    error: Some(format!("invalid hex payload: {e}")),
                }),
            );
        }
    };

    match state.service.ingest(req.record, &payload, None).await {
        Ok(()) => {
            info!("TLS ingest accepted");
            (
                StatusCode::ACCEPTED,
                Json(IngestResponse { status: "accepted".into(), error: None }),
            )
        }
        Err(e) => {
            warn!("TLS ingest rejected: {e}");
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(IngestResponse {
                    status: "rejected".into(),
                    error: Some(e.to_string()),
                }),
            )
        }
    }
}

// ── public API ────────────────────────────────────────────────────────────────

/// TLS certificate and key configuration for [`serve_tls`].
pub struct TlsConfig {
    /// Path to the PEM-encoded certificate chain file (leaf first, then
    /// intermediates).
    pub cert_path: PathBuf,
    /// Path to the PEM-encoded private key file (PKCS #8 or PKCS #1 RSA).
    pub key_path: PathBuf,
}

impl TlsConfig {
    /// Create a [`TlsConfig`] from PEM file paths.
    ///
    /// Files are read lazily when [`serve_tls`] is called, not when this
    /// constructor runs.
    pub fn from_pem_files(cert: impl Into<PathBuf>, key: impl Into<PathBuf>) -> Self {
        Self { cert_path: cert.into(), key_path: key.into() }
    }
}

/// Start the HTTPS ingest server using rustls TLS 1.2/1.3.
///
/// Listens on `addr`, performs the rustls TLS handshake with the certificate
/// and private key loaded from `tls`, then routes each HTTPS request through
/// the same `POST /api/v1/ingest` JSON handler used by the plain HTTP
/// transport.
///
/// # Network policy
///
/// The IP allowlist from `network_policy` is enforced at the TCP accept step,
/// before the TLS handshake.  Blocked connections are dropped immediately.
///
/// # TLS parameters
///
/// - rustls 0.23, TLS 1.2 minimum, TLS 1.3 preferred.
/// - No mutual TLS (client certificates not required).
/// - Supply the leaf certificate followed by any intermediate certificates in
///   the PEM chain file.
pub async fn serve_tls<R, L, O>(
    service: AsyncIngestService<R, L, O>,
    network_policy: NetworkPolicy,
    addr: SocketAddr,
    tls: TlsConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    R: AsyncRawDataStore + Send + Sync + 'static,
    L: AsyncAuditLedger + Send + Sync + 'static,
    O: AsyncOperationLogStore + Send + Sync + 'static,
{
    // Install ring as the default rustls crypto provider.  This is a no-op if
    // another provider was already installed (e.g. aws-lc-rs in the caller).
    let _ = tokio_rustls::rustls::crypto::ring::default_provider().install_default();

    // Load certificate chain (leaf first, then intermediates).
    let cert_chain = {
        let mut f = std::io::BufReader::new(std::fs::File::open(&tls.cert_path)?);
        certs(&mut f).collect::<Result<Vec<_>, _>>()?
    };

    // Load private key (PKCS #8 or PKCS #1).
    let private_key = {
        let mut f = std::io::BufReader::new(std::fs::File::open(&tls.key_path)?);
        private_key(&mut f)?.ok_or("no private key found in key file")?
    };

    let rustls_cfg = tokio_rustls::rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, private_key)?;
    let acceptor = TlsAcceptor::from(Arc::new(rustls_cfg));

    let state = TlsState { service: Arc::new(service) };
    let network_policy = Arc::new(network_policy);

    let app = Router::new()
        .route("/api/v1/ingest", post(tls_ingest_handler::<R, L, O>))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(addr = %addr, "HTTPS ingest server listening (TLS 1.2/1.3)");

    loop {
        let (tcp, peer_addr) = listener.accept().await?;

        // Enforce network policy before TLS handshake.
        if let Err(e) = network_policy.check(peer_addr.ip()) {
            warn!(ip = %peer_addr.ip(), "TLS ingest blocked by network policy: {e}");
            continue; // drop `tcp` — connection closes immediately
        }

        let acceptor = acceptor.clone();
        let app = app.clone();

        tokio::spawn(async move {
            let tls_stream = match acceptor.accept(tcp).await {
                Ok(s) => s,
                Err(e) => {
                    warn!(ip = %peer_addr.ip(), "TLS handshake failed: {e}");
                    return;
                }
            };

            let _ = ConnBuilder::new(TokioExecutor::new())
                .serve_connection(TokioIo::new(tls_stream), TowerToHyperService::new(app))
                .await;
        });
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls_config_paths_are_stored() {
        let cfg = TlsConfig::from_pem_files("/etc/ssl/cert.pem", "/etc/ssl/key.pem");
        assert_eq!(cfg.cert_path, PathBuf::from("/etc/ssl/cert.pem"));
        assert_eq!(cfg.key_path, PathBuf::from("/etc/ssl/key.pem"));
    }
}
