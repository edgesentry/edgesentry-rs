//! TLS configuration for the HTTP ingest transport.
//!
//! Builds a `rustls` [`ServerConfig`] from PEM-encoded certificate and private
//! key material.  The resulting config enforces TLS 1.2 as the minimum version
//! (TLS 1.3 is preferred), satisfying CLS-05 / §5.5 "Communicate securely" and
//! ETSI EN 303 645 §5.5 channel confidentiality requirements.
//!
//! # Minimum TLS version
//!
//! | Version | Support |
//! |---------|---------|
//! | TLS 1.3 | Preferred (negotiated first) |
//! | TLS 1.2 | Accepted (CLS-05 minimum) |
//! | TLS 1.1 and below | Rejected |
//!
//! # Example
//!
//! ```no_run
//! use edgesentry_rs::transport::tls::TlsConfig;
//! use std::path::Path;
//!
//! let tls = TlsConfig::from_pem_files(
//!     Path::new("cert.pem"),
//!     Path::new("key.pem"),
//! ).unwrap();
//! ```

use std::io;
use std::path::Path;
use std::sync::Arc;

use rustls::ServerConfig;
use thiserror::Error;

/// Errors produced when loading or building a [`TlsConfig`].
#[derive(Debug, Error)]
pub enum TlsConfigError {
    #[error("I/O error reading TLS material: {0}")]
    Io(#[from] io::Error),
    #[error("no certificate found in PEM data")]
    NoCertificate,
    #[error("no private key found in PEM data")]
    NoPrivateKey,
    #[error("rustls configuration error: {0}")]
    Rustls(#[from] rustls::Error),
}

/// PEM-based TLS configuration for [`serve_tls`](super::http::serve_tls).
///
/// Enforces TLS 1.2 as the minimum accepted version; TLS 1.3 is preferred.
pub struct TlsConfig {
    pub(crate) server_config: Arc<ServerConfig>,
}

impl TlsConfig {
    /// Load certificate chain and private key from PEM files on disk.
    pub fn from_pem_files(cert_path: &Path, key_path: &Path) -> Result<Self, TlsConfigError> {
        let cert_pem = std::fs::read(cert_path)?;
        let key_pem = std::fs::read(key_path)?;
        Self::from_pem_bytes(&cert_pem, &key_pem)
    }

    /// Build a TLS config from raw PEM bytes.
    pub fn from_pem_bytes(cert_pem: &[u8], key_pem: &[u8]) -> Result<Self, TlsConfigError> {
        use rustls::pki_types::{CertificateDer, PrivateKeyDer};
        use rustls_pemfile::{certs, private_key};

        let certs: Vec<CertificateDer<'static>> = certs(&mut io::BufReader::new(cert_pem))
            .collect::<Result<Vec<_>, _>>()?;
        if certs.is_empty() {
            return Err(TlsConfigError::NoCertificate);
        }

        let key: PrivateKeyDer<'static> =
            private_key(&mut io::BufReader::new(key_pem))?
                .ok_or(TlsConfigError::NoPrivateKey)?;

        // Use the ring crypto provider explicitly to avoid ambiguity when
        // multiple rustls providers (ring, aws-lc-rs) are present in the tree.
        // TLS 1.2 minimum, TLS 1.3 preferred.
        let provider = std::sync::Arc::new(rustls::crypto::ring::default_provider());
        let server_config = ServerConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls::version::TLS13, &rustls::version::TLS12])?
            .with_no_client_auth()
            .with_single_cert(certs, key)?;

        Ok(Self {
            server_config: Arc::new(server_config),
        })
    }
}
