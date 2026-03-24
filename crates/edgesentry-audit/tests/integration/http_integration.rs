//! HTTP ingest transport integration tests.
//!
//! Spins up an ephemeral axum server against in-memory stores, then exercises
//! the `POST /api/v1/ingest` endpoint via `reqwest`.
//!
//! All tests run unconditionally (no external services required).

#![cfg(feature = "transport-http")]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use ed25519_dalek::{SigningKey, VerifyingKey};
use edgesentry_audit::{
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

    tokio::spawn(edgesentry_audit::transport::http::serve(service, network_policy, addr));

    // Give the server a moment to bind.
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    addr
}

/// Spawn a server with an *empty* network policy (denies every source IP).
async fn spawn_server_no_allowlist(signing_key: &SigningKey, device_id: &str) -> SocketAddr {
    let verifying_key = VerifyingKey::from(signing_key);
    let addr = bind_ephemeral().await;

    let mut policy = IntegrityPolicyGate::new();
    policy.register_device(device_id, verifying_key);

    // Empty NetworkPolicy — no IP is allowed.
    let network_policy = NetworkPolicy::new();

    let service = AsyncIngestService::new(
        policy,
        AsyncInMemoryRawDataStore::default(),
        AsyncInMemoryAuditLedger::default(),
        AsyncInMemoryOperationLog::default(),
    );

    tokio::spawn(edgesentry_audit::transport::http::serve(service, network_policy, addr));
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

// ── malformed / half-valid JSON rejection tests (#158) ────────────────────────

#[tokio::test]
async fn http_missing_record_field_returns_4xx() {
    let signing_key = SigningKey::from_bytes(&[20u8; 32]);
    let addr = spawn_server(&signing_key, "http-dev-20").await;

    // Valid JSON but missing the required `record` field.
    let body = serde_json::json!({ "raw_payload_hex": "deadbeef" });

    let resp = reqwest::Client::new()
        .post(ingest_url(addr))
        .json(&body)
        .send()
        .await
        .expect("request");

    let status = resp.status().as_u16();
    assert!(
        status == 400 || status == 422,
        "missing `record` field must return 400 or 422, got {status}"
    );
}

#[tokio::test]
async fn http_missing_payload_field_returns_4xx() {
    let signing_key = SigningKey::from_bytes(&[21u8; 32]);
    let addr = spawn_server(&signing_key, "http-dev-21").await;

    let record = build_signed_record(
        "http-dev-21", 1, 1_700_000_000_000, b"data",
        AuditRecord::zero_hash(), "http-dev-21/1.bin", &signing_key,
    );
    // Missing `raw_payload_hex`.
    let body = serde_json::json!({ "record": record });

    let resp = reqwest::Client::new()
        .post(ingest_url(addr))
        .json(&body)
        .send()
        .await
        .expect("request");

    let status = resp.status().as_u16();
    assert!(
        status == 400 || status == 422,
        "missing `raw_payload_hex` must return 400 or 422, got {status}"
    );
}

#[tokio::test]
async fn http_record_field_as_string_returns_4xx() {
    let signing_key = SigningKey::from_bytes(&[22u8; 32]);
    let addr = spawn_server(&signing_key, "http-dev-22").await;

    // `record` is a string instead of an object — wrong type.
    let body = serde_json::json!({
        "record": "this-should-be-an-object",
        "raw_payload_hex": "deadbeef",
    });

    let resp = reqwest::Client::new()
        .post(ingest_url(addr))
        .json(&body)
        .send()
        .await
        .expect("request");

    let status = resp.status().as_u16();
    assert!(
        status == 400 || status == 422,
        "`record` as string must return 400 or 422, got {status}"
    );
}

#[tokio::test]
async fn http_payload_hex_as_integer_returns_4xx() {
    let signing_key = SigningKey::from_bytes(&[23u8; 32]);
    let addr = spawn_server(&signing_key, "http-dev-23").await;

    let record = build_signed_record(
        "http-dev-23", 1, 1_700_000_000_000, b"data",
        AuditRecord::zero_hash(), "http-dev-23/1.bin", &signing_key,
    );
    // `raw_payload_hex` is an integer instead of a string.
    let body = serde_json::json!({ "record": record, "raw_payload_hex": 42 });

    let resp = reqwest::Client::new()
        .post(ingest_url(addr))
        .json(&body)
        .send()
        .await
        .expect("request");

    let status = resp.status().as_u16();
    assert!(
        status == 400 || status == 422,
        "`raw_payload_hex` as integer must return 400 or 422, got {status}"
    );
}

#[tokio::test]
async fn http_empty_body_returns_4xx() {
    let signing_key = SigningKey::from_bytes(&[24u8; 32]);
    let addr = spawn_server(&signing_key, "http-dev-24").await;

    let resp = reqwest::Client::new()
        .post(ingest_url(addr))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body("")
        .send()
        .await
        .expect("request");

    let status = resp.status().as_u16();
    assert!(
        (400..500).contains(&status),
        "empty body must return 4xx, got {status}"
    );
}

#[tokio::test]
async fn http_json_array_body_returns_4xx() {
    let signing_key = SigningKey::from_bytes(&[25u8; 32]);
    let addr = spawn_server(&signing_key, "http-dev-25").await;

    // Top-level JSON array instead of an object.
    let resp = reqwest::Client::new()
        .post(ingest_url(addr))
        .json(&serde_json::json!([1, 2, 3]))
        .send()
        .await
        .expect("request");

    let status = resp.status().as_u16();
    assert!(
        status == 400 || status == 422,
        "JSON array body must return 400 or 422, got {status}"
    );
}

#[tokio::test]
async fn http_blocked_source_ip_returns_403() {
    // Server with empty NetworkPolicy — loopback is not in the allowlist.
    let signing_key = SigningKey::from_bytes(&[26u8; 32]);
    let addr = spawn_server_no_allowlist(&signing_key, "http-dev-26").await;

    // The NetworkPolicy check runs inside the handler after JSON extraction,
    // so the body must be structurally valid (both fields present) for the
    // 403 branch to be reached.
    let payload = b"blocked-test";
    let record = build_signed_record(
        "http-dev-26", 1, 1_700_000_000_000, payload,
        AuditRecord::zero_hash(), "http-dev-26/1.bin", &signing_key,
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
        .expect("request");

    assert_eq!(resp.status(), 403, "blocked IP must return 403 Forbidden");

    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["status"], "rejected");
}
