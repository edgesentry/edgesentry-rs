//! MQTT transport integration tests.
//!
//! Tests that do not require a broker exercise the message parsing and response
//! serialisation logic directly.
//!
//! Broker-level round-trip tests require a running MQTT broker and are gated
//! behind the `TEST_MQTT_BROKER` environment variable (e.g. `mosquitto` on
//! localhost:1883).  They are skipped automatically in CI unless the variable
//! is set.

#![cfg(feature = "transport-mqtt")]

use edgesentry_audit::{
    build_lift_inspection_demo_records_with_payloads, parse_fixed_hex,
    transport::mqtt::{
        parse_ingest_message, MqttIngestConfig, MqttIngestRequest, MqttIngestResponse, MqttQos,
    },
    AuditRecord,
};
use ed25519_dalek::SigningKey;

const PRIV_HEX: &str = "0101010101010101010101010101010101010101010101010101010101010101";

fn demo_pairs() -> Vec<(AuditRecord, Vec<u8>)> {
    build_lift_inspection_demo_records_with_payloads(
        "lift-01",
        PRIV_HEX,
        1_700_000_000_000,
        "s3://bucket/lift-01",
    )
    .unwrap()
}

// ---------------------------------------------------------------------------
// Message parsing — no broker required
// ---------------------------------------------------------------------------

#[test]
fn parse_valid_ingest_message() {
    let pairs = demo_pairs();
    let (record, payload) = &pairs[0];

    let json = serde_json::json!({
        "record": record,
        "raw_payload_hex": hex::encode(payload),
    })
    .to_string();

    let req: MqttIngestRequest = parse_ingest_message(json.as_bytes()).unwrap();
    assert_eq!(req.record.device_id, record.device_id);
    assert_eq!(req.record.sequence, record.sequence);
}

#[test]
fn parse_rejects_empty_bytes() {
    assert!(parse_ingest_message(b"").is_err());
}

#[test]
fn parse_rejects_garbage() {
    assert!(parse_ingest_message(b"\x00\x01\x02").is_err());
}

#[test]
fn parse_rejects_json_missing_record_field() {
    assert!(parse_ingest_message(b"{\"raw_payload_hex\": \"deadbeef\"}").is_err());
}

#[test]
fn parse_rejects_json_missing_payload_field() {
    let pairs = demo_pairs();
    let json = serde_json::json!({ "record": pairs[0].0 }).to_string();
    assert!(parse_ingest_message(json.as_bytes()).is_err());
}

// ---------------------------------------------------------------------------
// Config defaults
// ---------------------------------------------------------------------------

#[test]
fn config_new_sets_sensible_defaults() {
    let cfg = MqttIngestConfig::new("10.0.0.1", "edgesentry/ingest", "eds-test");
    assert_eq!(cfg.broker_host, "10.0.0.1");
    assert_eq!(cfg.broker_port, 1883);
    assert_eq!(cfg.topic, "edgesentry/ingest");
    assert_eq!(cfg.client_id, "eds-test");
    assert_eq!(cfg.qos, MqttQos::AtLeastOnce);
    assert_eq!(cfg.keep_alive_secs, 30);
    assert_eq!(cfg.channel_capacity, 64);
}

// ---------------------------------------------------------------------------
// Response serialisation
// ---------------------------------------------------------------------------

#[test]
fn accepted_response_roundtrip() {
    let pairs = demo_pairs();
    let record = &pairs[0].0;
    let resp = MqttIngestResponse {
        device_id: record.device_id.clone(),
        sequence: record.sequence,
        status: "accepted".into(),
        error: None,
    };
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("\"accepted\""));
    assert!(!json.contains("\"error\""), "accepted response must omit 'error' key");
}

#[test]
fn rejected_response_contains_error() {
    let pairs = demo_pairs();
    let record = &pairs[0].0;
    let resp = MqttIngestResponse {
        device_id: record.device_id.clone(),
        sequence: record.sequence,
        status: "rejected".into(),
        error: Some("unknown device".into()),
    };
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("\"rejected\""));
    assert!(json.contains("\"unknown device\""));
}

// ---------------------------------------------------------------------------
// TLS configuration — no broker required
// ---------------------------------------------------------------------------

#[cfg(feature = "transport-mqtt-tls")]
mod tls_config_tests {
    use edgesentry_audit::transport::mqtt::{MqttIngestConfig, MqttTlsConfig};
    use std::sync::atomic::{AtomicU64, Ordering};

    fn unique_ca_cert_path() -> std::path::PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let pid = std::process::id();
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir()
            .join(format!("edgesentry_mqtt_tls_test_{pid}_{id}_ca.pem"))
    }

    fn write_self_signed_ca() -> std::path::PathBuf {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])
            .expect("rcgen must generate self-signed cert");
        let path = unique_ca_cert_path();
        std::fs::write(&path, cert.cert.pem()).expect("write CA PEM");
        path
    }

    #[test]
    fn mqtt_tls_config_stores_ca_cert_path() {
        let path = write_self_signed_ca();
        let tls = MqttTlsConfig::from_ca_cert_file(&path);
        assert_eq!(tls.ca_cert_path, path);
    }

    #[test]
    fn mqtt_ingest_config_tls_defaults_to_none() {
        let cfg = MqttIngestConfig::new("broker.local", "edgesentry/ingest", "eds-tls-test");
        assert!(cfg.tls.is_none(), "TLS must be disabled by default");
    }

    #[test]
    fn mqtt_ingest_config_accepts_tls_config() {
        let path = write_self_signed_ca();
        let mut cfg = MqttIngestConfig::new("broker.local", "edgesentry/ingest", "eds-tls-test");
        cfg.tls = Some(MqttTlsConfig::from_ca_cert_file(&path));
        assert!(cfg.tls.is_some());
        assert_eq!(cfg.tls.as_ref().unwrap().ca_cert_path, path);
    }
}

// ---------------------------------------------------------------------------
// Broker round-trip — requires TEST_MQTT_BROKER env var
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "requires a running MQTT broker — set TEST_MQTT_BROKER=host:port to enable"]
async fn broker_roundtrip_accepts_valid_record() {
    use edgesentry_audit::{
        AsyncInMemoryAuditLedger, AsyncInMemoryOperationLog, AsyncInMemoryRawDataStore,
        AsyncIngestService, IntegrityPolicyGate,
    };
    use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
    use std::time::Duration;

    let broker_addr = std::env::var("TEST_MQTT_BROKER")
        .unwrap_or_else(|_| "localhost:1883".to_string());
    let (host, port_str) = broker_addr.split_once(':').unwrap_or((&broker_addr, "1883"));
    let port: u16 = port_str.parse().unwrap_or(1883);

    // Register device
    let key_bytes = parse_fixed_hex::<32>(PRIV_HEX).unwrap();
    let signing_key = SigningKey::from_bytes(&key_bytes);
    let mut policy = IntegrityPolicyGate::new();
    policy.register_device("lift-01", signing_key.verifying_key());

    let service = AsyncIngestService::new(
        policy,
        AsyncInMemoryRawDataStore::default(),
        AsyncInMemoryAuditLedger::default(),
        AsyncInMemoryOperationLog::default(),
    );

    let ingest_topic = "edgesentry/ingest/test-broker-roundtrip";
    let response_topic = format!("{ingest_topic}/response");

    // Start serve_mqtt in a background task
    let cfg = {
        let mut c = MqttIngestConfig::new(host, ingest_topic, "eds-server-test");
        c.broker_port = port;
        c
    };
    let serve_handle = tokio::spawn(async move {
        let _ = edgesentry_audit::transport::mqtt::serve_mqtt(cfg, service).await;
    });

    // Give the server time to connect and subscribe
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Publish client — uses a different client_id
    let mut opts = MqttOptions::new("eds-test-publisher", host, port);
    opts.set_keep_alive(Duration::from_secs(10));
    let (pub_client, mut pub_eventloop) = AsyncClient::new(opts, 16);

    // Subscribe to the response topic before publishing
    pub_client.subscribe(&response_topic, QoS::AtLeastOnce).await.unwrap();

    // Build and publish a valid record
    let pairs = demo_pairs();
    let (record, payload) = &pairs[0];
    let msg = serde_json::json!({
        "record": record,
        "raw_payload_hex": hex::encode(payload),
    })
    .to_string();
    pub_client
        .publish(ingest_topic, QoS::AtLeastOnce, false, msg.as_bytes())
        .await
        .unwrap();

    // Drain events and wait for the response
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if tokio::time::Instant::now() > deadline {
            panic!("timed out waiting for MQTT response");
        }
        match pub_eventloop.poll().await {
            Ok(Event::Incoming(Packet::Publish(p))) if p.topic == response_topic => {
                let resp: MqttIngestResponse =
                    serde_json::from_slice(&p.payload).expect("valid response JSON");
                assert_eq!(resp.status, "accepted");
                assert_eq!(resp.device_id, record.device_id);
                assert_eq!(resp.sequence, record.sequence);
                break;
            }
            Ok(_) => {}
            Err(e) => panic!("publisher event loop error: {e}"),
        }
    }

    serve_handle.abort();
}
