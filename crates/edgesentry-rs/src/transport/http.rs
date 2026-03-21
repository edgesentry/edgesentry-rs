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
