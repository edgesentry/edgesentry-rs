//! TLS ingest transport integration tests.
//!
//! Spins up an ephemeral HTTPS server using a self-signed certificate generated
//! with `rcgen`, then exercises `POST /api/v1/ingest` via `reqwest` with a
//! custom root CA.
//!
//! All tests run unconditionally (no external services or pre-existing certs required).

#![cfg(feature = "transport-tls")]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use ed25519_dalek::{SigningKey, VerifyingKey};
use edgesentry_rs::{
    build_signed_record, AsyncInMemoryAuditLedger, AsyncInMemoryOperationLog,
    AsyncInMemoryRawDataStore, AsyncIngestService, AuditRecord, IntegrityPolicyGate, NetworkPolicy,
};
use edgesentry_rs::transport::tls::TlsConfig;

/// Generate a self-signed certificate and return (cert_pem, key_pem, der_bytes_for_reqwest).
fn self_signed_cert() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let subject_alt_names = vec!["127.0.0.1".to_string()];
    let cert = rcgen::generate_simple_self_signed(subject_alt_names).unwrap();
    let cert_pem = cert.cert.pem().into_bytes();
    let key_pem = cert.key_pair.serialize_pem().into_bytes();
    let cert_der = cert.cert.der().to_vec();
    (cert_pem, key_pem, cert_der)
}

/// Bind an ephemeral port.
async fn bind_ephemeral() -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap()
}

/// Spawn the HTTPS server and return (addr, reqwest_client_with_ca).
async fn spawn_tls_server(
    signing_key: &SigningKey,
    device_id: &str,
) -> (SocketAddr, reqwest::Client) {
    let verifying_key = VerifyingKey::from(signing_key);
    let addr = bind_ephemeral().await;

    let (cert_pem, key_pem, cert_der) = self_signed_cert();

    let mut policy = IntegrityPolicyGate::new();
    policy.register_device(device_id, verifying_key);

    let mut network_policy = NetworkPolicy::new();
    network_policy.allow_ip(IpAddr::V4(Ipv4Addr::LOCALHOST));

    let service = AsyncIngestService::new(
        policy,
        AsyncInMemoryRawDataStore::default(),
        AsyncInMemoryAuditLedger::default(),
        AsyncInMemoryOperationLog::default(),
    );

    let tls_config = TlsConfig::from_pem_bytes(&cert_pem, &key_pem)
        .expect("TlsConfig should build from self-signed cert");

    tokio::spawn(edgesentry_rs::transport::http::serve_tls(
        service,
        network_policy,
        addr,
        tls_config,
    ));

    tokio::time::sleep(std::time::Duration::from_millis(30)).await;

    // Build a reqwest client that trusts the self-signed CA.
    let root_cert = reqwest::tls::Certificate::from_der(&cert_der).unwrap();
    let client = reqwest::Client::builder()
        .add_root_certificate(root_cert)
        .use_rustls_tls()
        .build()
        .unwrap();

    (addr, client)
}

fn ingest_url(addr: SocketAddr) -> String {
    format!("https://{addr}/api/v1/ingest")
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn tls_accepted_record_returns_202() {
    let signing_key = SigningKey::from_bytes(&[20u8; 32]);
    let (addr, client) = spawn_tls_server(&signing_key, "tls-dev-01").await;

    let payload = b"tls-test-payload";
    let record = build_signed_record(
        "tls-dev-01",
        1,
        1_700_000_002_000,
        payload,
        AuditRecord::zero_hash(),
        "tls-dev-01/1.bin",
        &signing_key,
    );

    let body = serde_json::json!({
        "record": record,
        "raw_payload_hex": hex::encode(payload),
    });

    let resp = client
        .post(ingest_url(addr))
        .json(&body)
        .send()
        .await
        .expect("HTTPS request must succeed");

    assert_eq!(resp.status(), 202, "accepted record over TLS must return 202");

    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["status"], "accepted");
}

#[tokio::test]
async fn tls_tampered_payload_returns_422() {
    let signing_key = SigningKey::from_bytes(&[21u8; 32]);
    let (addr, client) = spawn_tls_server(&signing_key, "tls-dev-02").await;

    let record = build_signed_record(
        "tls-dev-02",
        1,
        1_700_000_002_001,
        b"original",
        AuditRecord::zero_hash(),
        "tls-dev-02/1.bin",
        &signing_key,
    );

    let body = serde_json::json!({
        "record": record,
        "raw_payload_hex": hex::encode(b"tampered"),
    });

    let resp = client
        .post(ingest_url(addr))
        .json(&body)
        .send()
        .await
        .expect("HTTPS request must succeed");

    assert_eq!(resp.status(), 422, "hash mismatch over TLS must return 422");
    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["status"], "rejected");
}

#[tokio::test]
async fn tls_config_rejects_missing_cert() {
    let result = TlsConfig::from_pem_bytes(b"not a cert", b"not a key");
    assert!(result.is_err(), "invalid PEM must fail");
}

#[tokio::test]
async fn tls_config_loads_from_self_signed() {
    let (cert_pem, key_pem, _) = self_signed_cert();
    let result = TlsConfig::from_pem_bytes(&cert_pem, &key_pem);
    assert!(result.is_ok(), "self-signed cert must load: {:?}", result.err());
}
