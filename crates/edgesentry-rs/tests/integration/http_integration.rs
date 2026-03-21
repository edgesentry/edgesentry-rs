//! HTTP ingest transport integration tests.
//!
//! Spins up an ephemeral axum server against in-memory stores, then exercises
//! the `POST /api/v1/ingest` endpoint via `reqwest`.
//!
//! All tests run unconditionally (no external services required).

#![cfg(feature = "transport-http")]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use ed25519_dalek::{SigningKey, VerifyingKey};
use edgesentry_rs::{
    build_signed_record, AsyncInMemoryAuditLedger, AsyncInMemoryOperationLog,
    AsyncInMemoryRawDataStore, AsyncIngestService, AuditRecord, IntegrityPolicyGate, NetworkPolicy,
};

/// Bind an ephemeral port, return the address.
async fn bind_ephemeral() -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap()
}

/// Spawn the HTTP server in the background and return its bound address.
async fn spawn_server(
    signing_key: &SigningKey,
    device_id: &str,
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

    tokio::spawn(edgesentry_rs::transport::http::serve(service, network_policy, addr));

    // Give the server a moment to bind.
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    addr
}

fn ingest_url(addr: SocketAddr) -> String {
    format!("http://{addr}/api/v1/ingest")
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn http_accepted_record_returns_202() {
    let signing_key = SigningKey::from_bytes(&[10u8; 32]);
    let addr = spawn_server(&signing_key, "http-dev-01").await;

    let payload = b"http-test-payload";
    let record = build_signed_record(
        "http-dev-01",
        1,
        1_700_000_000_000,
        payload,
        AuditRecord::zero_hash(),
        "http-dev-01/1.bin",
        &signing_key,
    );

    let body = serde_json::json!({
        "record": record,
        "raw_payload_hex": hex::encode(payload),
    });

    let resp = reqwest::Client::new()
        .post(ingest_url(addr))
        .json(&body)
        .send()
        .await
        .expect("request must succeed");

    assert_eq!(resp.status(), 202, "accepted record must return 202");

    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["status"], "accepted");
}

#[tokio::test]
async fn http_tampered_payload_returns_422() {
    let signing_key = SigningKey::from_bytes(&[11u8; 32]);
    let addr = spawn_server(&signing_key, "http-dev-02").await;

    let record = build_signed_record(
        "http-dev-02",
        1,
        1_700_000_000_001,
        b"original",
        AuditRecord::zero_hash(),
        "http-dev-02/1.bin",
        &signing_key,
    );

    // Send a different payload — hash won't match.
    let body = serde_json::json!({
        "record": record,
        "raw_payload_hex": hex::encode(b"tampered"),
    });

    let resp = reqwest::Client::new()
        .post(ingest_url(addr))
        .json(&body)
        .send()
        .await
        .expect("request must succeed");

    assert_eq!(resp.status(), 422, "hash mismatch must return 422");

    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["status"], "rejected");
    assert!(json["error"].as_str().unwrap().contains("hash mismatch"));
}

#[tokio::test]
async fn http_invalid_hex_payload_returns_400() {
    let signing_key = SigningKey::from_bytes(&[12u8; 32]);
    let addr = spawn_server(&signing_key, "http-dev-03").await;

    let record = build_signed_record(
        "http-dev-03",
        1,
        1_700_000_000_002,
        b"data",
        AuditRecord::zero_hash(),
        "http-dev-03/1.bin",
        &signing_key,
    );

    let body = serde_json::json!({
        "record": record,
        "raw_payload_hex": "not-valid-hex!!",
    });

    let resp = reqwest::Client::new()
        .post(ingest_url(addr))
        .json(&body)
        .send()
        .await
        .expect("request must succeed");

    assert_eq!(resp.status(), 400, "invalid hex must return 400");

    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["status"], "rejected");
}

#[tokio::test]
async fn http_unknown_device_returns_422() {
    // Server registers http-dev-04, but the record is signed by an unregistered key.
    let server_key = SigningKey::from_bytes(&[13u8; 32]);
    let addr = spawn_server(&server_key, "http-dev-04").await;

    // Different key — not registered on the server.
    let unregistered_key = SigningKey::from_bytes(&[99u8; 32]);
    let payload = b"unknown";
    let record = build_signed_record(
        "http-dev-04",
        1,
        1_700_000_000_003,
        payload,
        AuditRecord::zero_hash(),
        "http-dev-04/1.bin",
        &unregistered_key,
    );

    let body = serde_json::json!({
        "record": record,
        "raw_payload_hex": hex::encode(payload),
    });

    let resp = reqwest::Client::new()
        .post(ingest_url(addr))
        .json(&body)
        .send()
        .await
        .expect("request must succeed");

    assert_eq!(resp.status(), 422, "unregistered device must return 422");
}

#[tokio::test]
async fn http_sequential_records_all_accepted() {
    let signing_key = SigningKey::from_bytes(&[14u8; 32]);
    let addr = spawn_server(&signing_key, "http-dev-05").await;

    let client = reqwest::Client::new();
    let mut prev_hash = AuditRecord::zero_hash();

    for seq in 1u64..=3 {
        let payload = format!("seq={seq}").into_bytes();
        let record = build_signed_record(
            "http-dev-05",
            seq,
            1_700_000_000_000 + seq,
            &payload,
            prev_hash,
            format!("http-dev-05/{seq}.bin"),
            &signing_key,
        );
        prev_hash = record.hash();

        let body = serde_json::json!({
            "record": record,
            "raw_payload_hex": hex::encode(&payload),
        });

        let resp = client
            .post(ingest_url(addr))
            .json(&body)
            .send()
            .await
            .expect("request must succeed");

        assert_eq!(
            resp.status(),
            202,
            "record {seq} must be accepted"
        );
    }
}
