//! MQTT ingest transport layer.
//!
//! Connects to an MQTT broker, subscribes to a configurable topic, and routes
//! incoming messages through [`AsyncIngestService`].  The message format is the
//! same JSON envelope used by the HTTP transport:
//!
//! ```json
//! {
//!   "record": { "device_id": "...", "sequence": 1, ... },
//!   "raw_payload_hex": "deadbeef..."
//! }
//! ```
//!
//! Accept / reject outcomes are published on a response topic
//! `<ingest_topic>/response` as:
//!
//! ```json
//! { "device_id": "...", "sequence": 1, "status": "accepted" }
//! { "device_id": "...", "sequence": 1, "status": "rejected", "error": "..." }
//! ```
//!
//! # Usage
//!
//! ```rust,no_run
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use edgesentry_rs::transport::mqtt::{MqttIngestConfig, serve_mqtt};
//! use edgesentry_rs::{
//!     AsyncInMemoryAuditLedger, AsyncInMemoryOperationLog, AsyncInMemoryRawDataStore,
//!     AsyncIngestService, IntegrityPolicyGate,
//! };
//!
//! let service = AsyncIngestService::new(
//!     IntegrityPolicyGate::new(),
//!     AsyncInMemoryRawDataStore::default(),
//!     AsyncInMemoryAuditLedger::default(),
//!     AsyncInMemoryOperationLog::default(),
//! );
//!
//! let config = MqttIngestConfig::new("localhost", "edgesentry/ingest", "eds-server");
//! serve_mqtt(config, service).await?;
//! # Ok(())
//! # }
//! ```

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, error, info, warn};

use crate::ingest::{AsyncAuditLedger, AsyncIngestService, AsyncOperationLogStore, AsyncRawDataStore};
use crate::record::AuditRecord;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Quality-of-service level for MQTT message delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MqttQos {
    /// At most once delivery (fire and forget).
    #[default]
    AtMostOnce,
    /// At least once delivery (acknowledged).
    AtLeastOnce,
    /// Exactly once delivery (four-way handshake).
    ExactlyOnce,
}

impl From<MqttQos> for QoS {
    fn from(qos: MqttQos) -> Self {
        match qos {
            MqttQos::AtMostOnce => QoS::AtMostOnce,
            MqttQos::AtLeastOnce => QoS::AtLeastOnce,
            MqttQos::ExactlyOnce => QoS::ExactlyOnce,
        }
    }
}

/// TLS configuration for MQTT over TLS (MQTTS, typically port 8883).
///
/// Provide a PEM-encoded CA certificate that signed the broker's server
/// certificate so the client can verify the broker's identity.
/// Mutual TLS (client certificate) is not yet supported.
///
/// Only available with the `transport-mqtt-tls` feature.
#[cfg(feature = "transport-mqtt-tls")]
#[derive(Debug, Clone)]
pub struct MqttTlsConfig {
    /// Path to the PEM-encoded CA certificate file used to verify the broker.
    pub ca_cert_path: PathBuf,
}

#[cfg(feature = "transport-mqtt-tls")]
impl MqttTlsConfig {
    /// Create an [`MqttTlsConfig`] from a CA certificate file path.
    pub fn from_ca_cert_file(path: impl Into<PathBuf>) -> Self {
        Self { ca_cert_path: path.into() }
    }
}

/// Configuration for the MQTT ingest transport.
#[derive(Debug, Clone)]
pub struct MqttIngestConfig {
    /// MQTT broker host or IP.
    pub broker_host: String,
    /// MQTT broker port (default 1883; use 8883 for TLS).
    pub broker_port: u16,
    /// Topic to subscribe to for incoming audit records.
    pub topic: String,
    /// MQTT client identifier — must be unique per broker connection.
    pub client_id: String,
    /// Quality-of-service level for inbound messages.
    pub qos: MqttQos,
    /// Keep-alive interval sent to the broker.
    pub keep_alive_secs: u64,
    /// Capacity of the internal MQTT event channel (number of in-flight messages).
    pub channel_capacity: usize,
    /// Optional TLS configuration; when set, connects over MQTTS using rustls.
    ///
    /// Only available with the `transport-mqtt-tls` feature.
    #[cfg(feature = "transport-mqtt-tls")]
    pub tls: Option<MqttTlsConfig>,
}

impl MqttIngestConfig {
    /// Construct a configuration with default port 1883, QoS AtLeastOnce,
    /// 30 s keep-alive, and channel capacity 64.
    pub fn new(
        broker_host: impl Into<String>,
        topic: impl Into<String>,
        client_id: impl Into<String>,
    ) -> Self {
        Self {
            broker_host: broker_host.into(),
            broker_port: 1883,
            topic: topic.into(),
            client_id: client_id.into(),
            qos: MqttQos::AtLeastOnce,
            keep_alive_secs: 30,
            channel_capacity: 64,
            #[cfg(feature = "transport-mqtt-tls")]
            tls: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Wire types
// ---------------------------------------------------------------------------

/// JSON envelope expected on the ingest topic.
#[derive(Debug, Deserialize)]
pub struct MqttIngestRequest {
    pub record: AuditRecord,
    pub raw_payload_hex: String,
}

/// JSON envelope published on the response topic (`<ingest_topic>/response`).
#[derive(Debug, Serialize, Deserialize)]
pub struct MqttIngestResponse {
    pub device_id: String,
    pub sequence: u64,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl MqttIngestResponse {
    fn accepted(record: &AuditRecord) -> Self {
        Self {
            device_id: record.device_id.clone(),
            sequence: record.sequence,
            status: "accepted".into(),
            error: None,
        }
    }

    fn rejected(record: &AuditRecord, reason: impl Into<String>) -> Self {
        Self {
            device_id: record.device_id.clone(),
            sequence: record.sequence,
            status: "rejected".into(),
            error: Some(reason.into()),
        }
    }
}

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum MqttServeError {
    #[error("failed to subscribe to topic '{topic}': {reason}")]
    Subscribe { topic: String, reason: String },
    #[error("MQTT event loop error: {0}")]
    EventLoop(String),
    #[cfg(feature = "transport-mqtt-tls")]
    #[error("TLS configuration error: {0}")]
    TlsConfig(String),
}

// ---------------------------------------------------------------------------
// Parse helper (testable without a broker)
// ---------------------------------------------------------------------------

/// Parse raw MQTT message bytes into an [`MqttIngestRequest`].
///
/// Exposed so callers can unit-test their message serialisation without a broker.
pub fn parse_ingest_message(bytes: &[u8]) -> Result<MqttIngestRequest, serde_json::Error> {
    serde_json::from_slice(bytes)
}

// ---------------------------------------------------------------------------
// serve_mqtt
// ---------------------------------------------------------------------------

/// Connect to the MQTT broker described by `config`, subscribe to the ingest
/// topic, and route every well-formed message through `service`.
///
/// This function runs until the broker connection is lost, at which point it
/// returns [`MqttServeError::EventLoop`].  Callers that want automatic
/// reconnection should wrap the call in a retry loop.
///
/// Responses (accept / reject) are published on `<topic>/response` at QoS
/// [`MqttQos::AtMostOnce`] to avoid back-pressure on the event loop.
pub async fn serve_mqtt<R, L, O>(
    config: MqttIngestConfig,
    service: AsyncIngestService<R, L, O>,
) -> Result<(), MqttServeError>
where
    R: AsyncRawDataStore + Send + Sync + 'static,
    L: AsyncAuditLedger + Send + Sync + 'static,
    O: AsyncOperationLogStore + Send + Sync + 'static,
{
    let mut opts =
        MqttOptions::new(&config.client_id, &config.broker_host, config.broker_port);
    opts.set_keep_alive(Duration::from_secs(config.keep_alive_secs));

    // Configure MQTTS when a TLS config is present.
    #[cfg(feature = "transport-mqtt-tls")]
    if let Some(tls) = &config.tls {
        use std::io::BufReader;

        // Install ring as the default rustls crypto provider (no-op if already set).
        let _ = tokio_rustls::rustls::crypto::ring::default_provider().install_default();

        let ca_pem = std::fs::read(&tls.ca_cert_path)
            .map_err(|e| MqttServeError::TlsConfig(e.to_string()))?;

        let mut root_store = tokio_rustls::rustls::RootCertStore::empty();
        for cert in rustls_pemfile::certs(&mut BufReader::new(ca_pem.as_slice())) {
            let cert = cert.map_err(|e| MqttServeError::TlsConfig(e.to_string()))?;
            root_store.add(cert).map_err(|e| MqttServeError::TlsConfig(e.to_string()))?;
        }

        let rustls_config = tokio_rustls::rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        opts.set_transport(rumqttc::Transport::tls_with_config(
            rumqttc::TlsConfiguration::Rustls(Arc::new(rustls_config)),
        ));

        info!(
            broker = %config.broker_host,
            port   = config.broker_port,
            "MQTT TLS (MQTTS) enabled via rustls"
        );
    }

    let (client, mut eventloop) = AsyncClient::new(opts, config.channel_capacity);

    let qos: QoS = config.qos.into();
    client
        .subscribe(&config.topic, qos)
        .await
        .map_err(|e| MqttServeError::Subscribe {
            topic: config.topic.clone(),
            reason: e.to_string(),
        })?;

    let response_topic = format!("{}/response", config.topic);
    let service = Arc::new(service);

    info!(
        broker = %config.broker_host,
        port   = config.broker_port,
        topic  = %config.topic,
        "MQTT ingest transport started"
    );

    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::Publish(publish))) => {
                debug!(
                    topic = %publish.topic,
                    bytes = publish.payload.len(),
                    "MQTT message received"
                );

                let svc = Arc::clone(&service);
                let client2 = client.clone();
                let resp_topic = response_topic.clone();
                let payload = publish.payload.to_vec();

                tokio::spawn(async move {
                    handle_message(&payload, svc, &client2, &resp_topic).await;
                });
            }
            Ok(_) => {
                // ConnAck, PubAck, SubAck, PingResp, etc. — no action required.
            }
            Err(e) => {
                error!(error = %e, "MQTT event loop error — transport shutting down");
                return Err(MqttServeError::EventLoop(e.to_string()));
            }
        }
    }
}

async fn handle_message<R, L, O>(
    bytes: &[u8],
    service: Arc<AsyncIngestService<R, L, O>>,
    client: &AsyncClient,
    response_topic: &str,
) where
    R: AsyncRawDataStore + Send + Sync + 'static,
    L: AsyncAuditLedger + Send + Sync + 'static,
    O: AsyncOperationLogStore + Send + Sync + 'static,
{
    let req = match parse_ingest_message(bytes) {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "MQTT message is not valid JSON — discarded");
            return;
        }
    };

    let raw_payload = match hex::decode(&req.raw_payload_hex) {
        Ok(b) => b,
        Err(e) => {
            warn!(error = %e, "raw_payload_hex is not valid hex — discarded");
            return;
        }
    };

    let response = match service.ingest(req.record.clone(), &raw_payload, None).await {
        Ok(()) => {
            info!(
                device_id = %req.record.device_id,
                sequence  = req.record.sequence,
                "MQTT record accepted"
            );
            MqttIngestResponse::accepted(&req.record)
        }
        Err(e) => {
            warn!(
                device_id = %req.record.device_id,
                sequence  = req.record.sequence,
                reason    = %e,
                "MQTT record rejected"
            );
            MqttIngestResponse::rejected(&req.record, e.to_string())
        }
    };

    if let Ok(json) = serde_json::to_vec(&response) {
        // Best-effort publish — if it fails, log and continue.
        if let Err(e) = client.publish(response_topic, QoS::AtMostOnce, false, json).await {
            warn!(error = %e, "failed to publish MQTT response");
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests (no broker required)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_signed_record, parse_fixed_hex, record::AuditRecord};
    use ed25519_dalek::SigningKey;

    const PRIV_HEX: &str = "0101010101010101010101010101010101010101010101010101010101010101";

    fn make_record() -> (AuditRecord, Vec<u8>) {
        let key_bytes = parse_fixed_hex::<32>(PRIV_HEX).unwrap();
        let signing_key = SigningKey::from_bytes(&key_bytes);
        let payload = b"scenario=lift-inspection,check=door".to_vec();
        let record = build_signed_record(
            "lift-01".to_string(),
            1,
            1_700_000_000_000,
            &payload,
            AuditRecord::zero_hash(),
            "s3://bucket/lift-01/1.bin".to_string(),
            &signing_key,
        );
        (record, payload)
    }

    #[test]
    fn mqtt_qos_converts_correctly() {
        assert_eq!(QoS::from(MqttQos::AtMostOnce), QoS::AtMostOnce);
        assert_eq!(QoS::from(MqttQos::AtLeastOnce), QoS::AtLeastOnce);
        assert_eq!(QoS::from(MqttQos::ExactlyOnce), QoS::ExactlyOnce);
    }

    #[test]
    fn config_defaults_are_sensible() {
        let cfg = MqttIngestConfig::new("broker.local", "edgesentry/ingest", "eds-1");
        assert_eq!(cfg.broker_port, 1883);
        assert_eq!(cfg.qos, MqttQos::AtLeastOnce);
        assert_eq!(cfg.keep_alive_secs, 30);
        assert_eq!(cfg.channel_capacity, 64);
    }

    #[test]
    fn parse_ingest_message_accepts_valid_json() {
        let (record, payload) = make_record();
        let json = serde_json::to_vec(&serde_json::json!({
            "record": record,
            "raw_payload_hex": hex::encode(&payload),
        }))
        .unwrap();

        let parsed = parse_ingest_message(&json).unwrap();
        assert_eq!(parsed.record.device_id, record.device_id);
        assert_eq!(parsed.record.sequence, record.sequence);
        assert_eq!(parsed.raw_payload_hex, hex::encode(&payload));
    }

    #[test]
    fn parse_ingest_message_rejects_malformed_json() {
        assert!(parse_ingest_message(b"not json at all").is_err());
    }

    #[test]
    fn parse_ingest_message_rejects_missing_fields() {
        assert!(parse_ingest_message(b"{\"record\": null}").is_err());
    }

    #[test]
    fn response_accepted_fields() {
        let (record, _) = make_record();
        let resp = MqttIngestResponse::accepted(&record);
        assert_eq!(resp.status, "accepted");
        assert_eq!(resp.device_id, record.device_id);
        assert_eq!(resp.sequence, record.sequence);
        assert!(resp.error.is_none());
    }

    #[test]
    fn response_rejected_fields() {
        let (record, _) = make_record();
        let resp = MqttIngestResponse::rejected(&record, "unknown device");
        assert_eq!(resp.status, "rejected");
        assert_eq!(resp.error.as_deref(), Some("unknown device"));
    }

    #[test]
    fn response_accepted_serialises_without_error_field() {
        let (record, _) = make_record();
        let resp = MqttIngestResponse::accepted(&record);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(!json.contains("\"error\""), "accepted response must not include 'error' key");
    }
}
