//! TLS ingest transport integration tests.
//!
//! Generates a self-signed certificate with `rcgen`, spins up an ephemeral
//! HTTPS server backed by in-memory stores, and exercises `POST /api/v1/ingest`
//! via `reqwest` with the self-signed cert added as a trusted root CA.

#![cfg(feature = "transport-tls")]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::fs;

use ed25519_dalek::{SigningKey, VerifyingKey};
use edgesentry_audit::{
    build_signed_record, AsyncInMemoryAuditLedger, AsyncInMemoryOperationLog,
    AsyncInMemoryRawDataStore, AsyncIngestService, AuditRecord, IntegrityPolicyGate, NetworkPolicy,
};
use edgesentry_audit::transport::tls::{serve_tls, TlsConfig};

// ── cert helpers ──────────────────────────────────────────────────────────────

/// Write a self-signed cert + key to temp files and return their paths.
///
/// Each call generates unique file paths via an atomic counter so concurrent
/// tests do not overwrite each other's PEM files.
fn generate_self_signed_cert() -> (PathBuf, PathBuf) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let subject_alt_names = vec!["localhost".to_string()];
    let cert = rcgen::generate_simple_self_signed(subject_alt_names)
        .expect("rcgen must generate self-signed cert");

    let cert_pem = cert.cert.pem();
    let key_pem = cert.key_pair.serialize_pem();

    let pid = std::process::id();
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let cert_path = std::env::temp_dir().join(format!("edgesentry_tls_test_{pid}_{id}_cert.pem"));
    let key_path = std::env::temp_dir().join(format!("edgesentry_tls_test_{pid}_{id}_key.pem"));

    std::fs::write(&cert_path, &cert_pem).expect("write cert PEM");
    std::fs::write(&key_path, &key_pem).expect("write key PEM");

    (cert_path, key_path)
}

// ── server helpers ────────────────────────────────────────────────────────────

async fn bind_ephemeral() -> SocketAddr {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    l.local_addr().unwrap()
}

async fn spawn_tls_server(
    signing_key: &SigningKey,
    device_id: &str,
    cert_path: PathBuf,
    key_path: PathBuf,
) -> SocketAddr {
    let verifying_key = VerifyingKey::from(signing_key);
    let addr = bind_ephemeral().await;

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

    let tls = TlsConfig::from_pem_files(cert_path, key_path);
    tokio::spawn(async move {
        let _ = serve_tls(service, network_policy, addr, tls).await;
    });

    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    addr
}

fn tls_client(cert_path: &PathBuf) -> reqwest::Client {
    let cert_pem = fs::read(cert_path).expect("must read test TLS certificate");
    let cert = reqwest::Certificate::from_pem(&cert_pem).expect("must parse test TLS certificate");

    reqwest::Client::builder()
        .add_root_certificate(cert)
        .danger_accept_invalid_certs(false)
        .build()
        .expect("TLS client must build")
}

fn ingest_url(addr: SocketAddr) -> String {
    format!("https://localhost:{}/api/v1/ingest", addr.port())
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn tls_accepted_record_returns_202() {
    let (cert_path, key_path) = generate_self_signed_cert();
    let signing_key = SigningKey::from_bytes(&[40u8; 32]);
    let addr = spawn_tls_server(&signing_key, "tls-dev-01", cert_path.clone(), key_path).await;

    let payload = b"tls-test-payload";
    let record = build_signed_record(
        "tls-dev-01",
        1,
        1_700_000_000_000,
        payload,
        AuditRecord::zero_hash(),
        "tls-dev-01/1.bin",
        &signing_key,
    );

    let body = serde_json::json!({
        "record": record,
        "raw_payload_hex": hex::encode(payload),
    });

    let resp = tls_client(&cert_path)
        .post(ingest_url(addr))
        .json(&body)
        .send()
        .await
        .expect("TLS request must succeed");

    assert_eq!(resp.status(), 202, "valid record over TLS must return 202");

    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["status"], "accepted");
}

#[tokio::test]
async fn tls_tampered_payload_returns_422() {
    let (cert_path, key_path) = generate_self_signed_cert();
    let signing_key = SigningKey::from_bytes(&[41u8; 32]);
    let addr = spawn_tls_server(&signing_key, "tls-dev-02", cert_path.clone(), key_path).await;

    let record = build_signed_record(
        "tls-dev-02",
        1,
        1_700_000_000_001,
        b"original",
        AuditRecord::zero_hash(),
        "tls-dev-02/1.bin",
        &signing_key,
    );

    let body = serde_json::json!({
        "record": record,
        "raw_payload_hex": hex::encode(b"tampered"),
    });

    let resp = tls_client(&cert_path)
        .post(ingest_url(addr))
        .json(&body)
        .send()
        .await
        .expect("request");

    assert_eq!(resp.status(), 422, "tampered payload over TLS must return 422");
    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["status"], "rejected");
}

#[tokio::test]
async fn tls_invalid_hex_returns_400() {
    let (cert_path, key_path) = generate_self_signed_cert();
    let signing_key = SigningKey::from_bytes(&[42u8; 32]);
    let addr = spawn_tls_server(&signing_key, "tls-dev-03", cert_path.clone(), key_path).await;

    let record = build_signed_record(
        "tls-dev-03",
        1,
        1_700_000_000_002,
        b"data",
        AuditRecord::zero_hash(),
        "tls-dev-03/1.bin",
        &signing_key,
    );

    let body = serde_json::json!({
        "record": record,
        "raw_payload_hex": "not-valid-hex!!",
    });

    let resp = tls_client(&cert_path)
        .post(ingest_url(addr))
        .json(&body)
        .send()
        .await
        .expect("request");

    assert_eq!(resp.status(), 400, "invalid hex over TLS must return 400");
}
