//! Axum-based HTTP ingest transport layer.
//!
//! Exposes a single endpoint:
//!
//! ```text
//! POST /api/v1/ingest
//! Content-Type: application/json
//!
//! {
//!   "record": { … AuditRecord fields … },
//!   "raw_payload_hex": "deadbeef…"
//! }
//! ```
//!
//! Responses:
//! - `202 Accepted` — record was verified and stored
//! - `400 Bad Request` — hex decode error or malformed body
//! - `403 Forbidden` — source IP is not in the network allowlist
//! - `422 Unprocessable Entity` — record failed integrity / ingest verification

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{ConnectInfo, Json, State};
use axum::http::StatusCode;
use axum::routing::post;
use axum::Router;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

#[cfg(feature = "transport-tls")]
use {
    hyper::body::Incoming,
    hyper_util::rt::{TokioExecutor, TokioIo},
    hyper_util::server::conn::auto::Builder as HyperBuilder,
    hyper_util::service::TowerToHyperService,
    tokio_rustls::TlsAcceptor,
    tower::ServiceExt,
};

use crate::ingest::{
    AsyncAuditLedger, AsyncIngestService, AsyncOperationLogStore, AsyncRawDataStore, NetworkPolicy,
};
use crate::record::AuditRecord;

#[derive(Debug, Deserialize)]
pub struct IngestRequest {
    pub record: AuditRecord,
    pub raw_payload_hex: String,
}

#[derive(Debug, Serialize)]
pub struct IngestResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl IngestResponse {
    fn accepted() -> Self {
        Self { status: "accepted".into(), error: None }
    }

    fn rejected(reason: impl Into<String>) -> Self {
        Self { status: "rejected".into(), error: Some(reason.into()) }
    }
}

struct AppState<R, L, O>
where
    R: AsyncRawDataStore + 'static,
    L: AsyncAuditLedger + 'static,
    O: AsyncOperationLogStore + 'static,
{
    service: Arc<AsyncIngestService<R, L, O>>,
    network_policy: Arc<NetworkPolicy>,
}

// Arc<T> is always Clone regardless of T, so we can implement Clone manually.
impl<R, L, O> Clone for AppState<R, L, O>
where
    R: AsyncRawDataStore + 'static,
    L: AsyncAuditLedger + 'static,
    O: AsyncOperationLogStore + 'static,
{
    fn clone(&self) -> Self {
        Self {
            service: Arc::clone(&self.service),
            network_policy: Arc::clone(&self.network_policy),
        }
    }
}

async fn ingest_handler<R, L, O>(
    State(state): State<AppState<R, L, O>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(req): Json<IngestRequest>,
) -> (StatusCode, Json<IngestResponse>)
where
    R: AsyncRawDataStore + Send + Sync + 'static,
    L: AsyncAuditLedger + Send + Sync + 'static,
    O: AsyncOperationLogStore + Send + Sync + 'static,
{
    let src_ip = addr.ip();

    if let Err(e) = state.network_policy.check(src_ip) {
        warn!(ip = %src_ip, "ingest request denied by network policy: {e}");
        return (
            StatusCode::FORBIDDEN,
            Json(IngestResponse::rejected(e.to_string())),
        );
    }

    let payload = match hex::decode(&req.raw_payload_hex) {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(IngestResponse::rejected(format!("invalid hex payload: {e}"))),
            );
        }
    };

    match state.service.ingest(req.record, &payload, None).await {
        Ok(()) => {
            info!(ip = %src_ip, "ingest accepted");
            (StatusCode::ACCEPTED, Json(IngestResponse::accepted()))
        }
        Err(e) => {
            warn!(ip = %src_ip, "ingest rejected: {e}");
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(IngestResponse::rejected(e.to_string())),
            )
        }
    }
}

/// Start the HTTP ingest server and block until the listener closes.
///
/// # Arguments
///
/// * `service` — an [`AsyncIngestService`] with registered devices
/// * `network_policy` — IP allowlist; connections from unlisted sources receive 403
/// * `addr` — local socket address to bind (e.g. `"0.0.0.0:8080".parse()`)
pub async fn serve<R, L, O>(
    service: AsyncIngestService<R, L, O>,
    network_policy: NetworkPolicy,
    addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    R: AsyncRawDataStore + Send + Sync + 'static,
    L: AsyncAuditLedger + Send + Sync + 'static,
    O: AsyncOperationLogStore + Send + Sync + 'static,
{
    let state: AppState<R, L, O> = AppState {
        service: Arc::new(service),
        network_policy: Arc::new(network_policy),
    };

    let app = Router::new()
        .route("/api/v1/ingest", post(ingest_handler::<R, L, O>))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(addr = %addr, "HTTP ingest server listening");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

/// Start the HTTPS ingest server with TLS termination and block until the listener closes.
///
/// Requires the `transport-tls` feature.  TLS 1.2 is the minimum accepted version;
/// TLS 1.3 is preferred, satisfying CLS-05 / ETSI EN 303 645 §5.5 channel confidentiality.
///
/// # Arguments
///
/// * `service` — an [`AsyncIngestService`] with registered devices
/// * `network_policy` — IP allowlist; connections from unlisted sources receive 403
/// * `addr` — local socket address to bind (e.g. `"0.0.0.0:8443".parse()`)
/// * `tls_config` — certificate chain and private key loaded via [`TlsConfig`]
///
/// [`TlsConfig`]: super::tls::TlsConfig
#[cfg(feature = "transport-tls")]
pub async fn serve_tls<R, L, O>(
    service: AsyncIngestService<R, L, O>,
    network_policy: NetworkPolicy,
    addr: SocketAddr,
    tls_config: crate::transport::tls::TlsConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    R: AsyncRawDataStore + Send + Sync + 'static,
    L: AsyncAuditLedger + Send + Sync + 'static,
    O: AsyncOperationLogStore + Send + Sync + 'static,
{
    let state: AppState<R, L, O> = AppState {
        service: Arc::new(service),
        network_policy: Arc::new(network_policy),
    };

    let app = Router::new()
        .route("/api/v1/ingest", post(ingest_handler::<R, L, O>))
        .with_state(state);

    let acceptor = TlsAcceptor::from(tls_config.server_config);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(addr = %addr, "HTTPS ingest server listening (TLS)");

    loop {
        let (stream, remote_addr) = listener.accept().await?;
        let acceptor = acceptor.clone();
        let app = app.clone();

        tokio::spawn(async move {
            let tls_stream = match acceptor.accept(stream).await {
                Ok(s) => s,
                Err(e) => {
                    warn!(addr = %remote_addr, "TLS handshake failed: {e}");
                    return;
                }
            };

            let io = TokioIo::new(tls_stream);

            // Inject the remote address as a ConnectInfo extension so the
            // ingest_handler can extract it via axum's ConnectInfo extractor.
            let svc = TowerToHyperService::new(tower::service_fn(
                move |mut req: hyper::Request<Incoming>| {
                    req.extensions_mut().insert(ConnectInfo(remote_addr));
                    let app = app.clone();
                    async move { app.oneshot(req).await }
                },
            ));

            if let Err(e) = HyperBuilder::new(TokioExecutor::new())
                .serve_connection(io, svc)
                .await
            {
                warn!(addr = %remote_addr, "connection error: {e}");
            }
        });
    }
}
