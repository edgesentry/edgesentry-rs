//! Async ingest integration tests.
//!
//! These tests exercise `AsyncIngestService` with in-memory stores and,
//! optionally, against a live MinIO instance when the S3 environment variables
//! are set (same variables as the sync S3 integration tests).
//!
//! In-memory tests run unconditionally.  MinIO tests skip when any env var is
//! absent.
//!
//! Environment variables for MinIO tests:
//!
//!   TEST_S3_ENDPOINT   e.g. http://localhost:9000
//!   TEST_S3_ACCESS_KEY e.g. minioadmin
//!   TEST_S3_SECRET_KEY e.g. minioadmin
//!   TEST_S3_BUCKET     e.g. bucket

#![cfg(feature = "async-ingest")]

use ed25519_dalek::{SigningKey, VerifyingKey};
use edgesentry_rs::{
    build_signed_record, AsyncInMemoryAuditLedger, AsyncInMemoryOperationLog,
    AsyncInMemoryRawDataStore, AsyncIngestService, AuditRecord, IngestDecision,
    IngestServiceError, IntegrityPolicyGate,
};

fn make_service() -> AsyncIngestService<
    AsyncInMemoryRawDataStore,
    AsyncInMemoryAuditLedger,
    AsyncInMemoryOperationLog,
> {
    AsyncIngestService::new(
        IntegrityPolicyGate::default(),
        AsyncInMemoryRawDataStore::default(),
        AsyncInMemoryAuditLedger::default(),
        AsyncInMemoryOperationLog::default(),
    )
}

// ── In-memory async tests ─────────────────────────────────────────────────────

#[tokio::test]
async fn async_ingest_accepted_record_stored_in_ledger() {
    let signing_key = SigningKey::from_bytes(&[1u8; 32]);
    let verifying_key = VerifyingKey::from(&signing_key);

    let svc = make_service();
    svc.register_device("sensor-async-01", verifying_key).await;

    let payload = b"async-test-payload";
    let record = build_signed_record(
        "sensor-async-01",
        1,
        1_700_000_000_000,
        payload,
        AuditRecord::zero_hash(),
        "sensor-async-01/1.bin",
        &signing_key,
    );

    svc.ingest(record.clone(), payload, None)
        .await
        .expect("ingest should succeed");

    let ledger_records = svc.audit_ledger().records().await;
    assert_eq!(ledger_records.len(), 1);
    assert_eq!(ledger_records[0].sequence, 1);
    assert_eq!(ledger_records[0].device_id, "sensor-async-01");

    let raw = svc
        .raw_data_store()
        .get("sensor-async-01/1.bin")
        .await
        .expect("raw payload must be stored");
    assert_eq!(raw.as_slice(), payload);
}

#[tokio::test]
async fn async_ingest_operation_log_records_accepted_entry() {
    let signing_key = SigningKey::from_bytes(&[2u8; 32]);
    let verifying_key = VerifyingKey::from(&signing_key);

    let svc = make_service();
    svc.register_device("sensor-async-02", verifying_key).await;

    let payload = b"op-log-test";
    let record = build_signed_record(
        "sensor-async-02",
        1,
        1_700_000_000_001,
        payload,
        AuditRecord::zero_hash(),
        "sensor-async-02/1.bin",
        &signing_key,
    );

    svc.ingest(record, payload, None)
        .await
        .expect("ingest should succeed");

    let entries = svc.operation_log().entries().await;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].decision, IngestDecision::Accepted);
    assert_eq!(entries[0].device_id, "sensor-async-02");
    assert_eq!(entries[0].sequence, 1);
}

#[tokio::test]
async fn async_ingest_payload_hash_mismatch_is_rejected() {
    let signing_key = SigningKey::from_bytes(&[3u8; 32]);
    let verifying_key = VerifyingKey::from(&signing_key);

    let svc = make_service();
    svc.register_device("sensor-async-03", verifying_key).await;

    let record = build_signed_record(
        "sensor-async-03",
        1,
        1_700_000_000_002,
        b"original",
        AuditRecord::zero_hash(),
        "sensor-async-03/1.bin",
        &signing_key,
    );

    let err = svc
        .ingest(record, b"tampered", None)
        .await
        .expect_err("hash mismatch must be rejected");

    assert!(
        matches!(err, IngestServiceError::PayloadHashMismatch { .. }),
        "expected PayloadHashMismatch, got: {err}"
    );

    let entries = svc.operation_log().entries().await;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].decision, IngestDecision::Rejected);

    assert!(
        svc.raw_data_store()
            .get("sensor-async-03/1.bin")
            .await
            .is_none(),
        "rejected ingest must not write to the raw data store"
    );
}

#[tokio::test]
async fn async_ingest_unknown_device_is_rejected() {
    let signing_key = SigningKey::from_bytes(&[4u8; 32]);
    let svc = make_service();
    // Device is NOT registered.

    let payload = b"unknown-device";
    let record = build_signed_record(
        "unknown-device",
        1,
        1_700_000_000_003,
        payload,
        AuditRecord::zero_hash(),
        "unknown-device/1.bin",
        &signing_key,
    );

    let err = svc
        .ingest(record, payload, None)
        .await
        .expect_err("unregistered device must be rejected");

    assert!(
        matches!(err, IngestServiceError::Verify(_)),
        "expected Verify error, got: {err}"
    );
}

#[tokio::test]
async fn async_ingest_sequential_records_accepted_in_order() {
    let signing_key = SigningKey::from_bytes(&[5u8; 32]);
    let verifying_key = VerifyingKey::from(&signing_key);

    let svc = make_service();
    svc.register_device("sensor-async-05", verifying_key).await;

    let mut prev_hash = AuditRecord::zero_hash();
    for seq in 1u64..=3 {
        let payload = format!("seq={seq}").into_bytes();
        let record = build_signed_record(
            "sensor-async-05",
            seq,
            1_700_000_000_000 + seq,
            &payload,
            prev_hash,
            format!("sensor-async-05/{seq}.bin"),
            &signing_key,
        );
        prev_hash = record.hash();
        svc.ingest(record, &payload, None)
            .await
            .unwrap_or_else(|e| panic!("record {seq} should be accepted: {e}"));
    }

    let ledger = svc.audit_ledger().records().await;
    assert_eq!(ledger.len(), 3);
    for (i, r) in ledger.iter().enumerate() {
        assert_eq!(r.sequence, (i as u64) + 1);
    }
}

// ── MinIO async tests ─────────────────────────────────────────────────────────

#[cfg(all(feature = "s3", feature = "async-ingest"))]
mod minio_async {
    use edgesentry_rs::{
        build_signed_record, AsyncIngestService, AuditRecord, AsyncInMemoryAuditLedger,
        AsyncInMemoryOperationLog, IntegrityPolicyGate, S3CompatibleRawDataStore,
        S3ObjectStoreConfig,
    };
    use ed25519_dalek::{SigningKey, VerifyingKey};

    fn s3_env() -> Option<(String, String, String, String)> {
        let endpoint = std::env::var("TEST_S3_ENDPOINT").ok()?;
        let access_key = std::env::var("TEST_S3_ACCESS_KEY").ok()?;
        let secret_key = std::env::var("TEST_S3_SECRET_KEY").ok()?;
        let bucket = std::env::var("TEST_S3_BUCKET").ok()?;
        Some((endpoint, access_key, secret_key, bucket))
    }

    async fn get_object_from_s3(
        endpoint: &str,
        access_key: &str,
        secret_key: &str,
        bucket: &str,
        key: &str,
    ) -> Option<Vec<u8>> {
        use aws_config::BehaviorVersion;
        use aws_config::Region;
        use aws_credential_types::Credentials;

        let creds = Credentials::new(access_key, secret_key, None, None, "static");
        let shared = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new("us-east-1"))
            .endpoint_url(endpoint)
            .credentials_provider(creds)
            .load()
            .await;
        let s3_conf = aws_sdk_s3::config::Builder::from(&shared)
            .force_path_style(true)
            .build();
        let client = aws_sdk_s3::Client::from_conf(s3_conf);
        match client.get_object().bucket(bucket).key(key).send().await {
            Ok(output) => {
                let data = output.body.collect().await.ok()?;
                Some(data.into_bytes().to_vec())
            }
            Err(_) => None,
        }
    }

    #[tokio::test]
    async fn async_ingest_accepted_record_uploads_to_minio() {
        let Some((endpoint, access_key, secret_key, bucket)) = s3_env() else {
            eprintln!(
                "async_ingest: skipping MinIO test \
                 (TEST_S3_ENDPOINT/KEY/BUCKET not set)"
            );
            return;
        };

        let config = S3ObjectStoreConfig::for_minio(
            bucket.clone(),
            "us-east-1",
            &endpoint,
            &access_key,
            &secret_key,
        );

        let signing_key = SigningKey::from_bytes(&[50u8; 32]);
        let verifying_key = VerifyingKey::from(&signing_key);

        let raw_store =
            S3CompatibleRawDataStore::new(config).expect("S3 store should initialise");
        let svc = AsyncIngestService::new(
            IntegrityPolicyGate::default(),
            raw_store,
            AsyncInMemoryAuditLedger::default(),
            AsyncInMemoryOperationLog::default(),
        );
        svc.register_device("lift-async-test-01", verifying_key).await;

        let payload = b"async-s3-integration-accepted";
        let key = "test/async-s3-integration-accepted.bin";
        let record = build_signed_record(
            "lift-async-test-01",
            1,
            1_700_000_001_000,
            payload,
            AuditRecord::zero_hash(),
            key,
            &signing_key,
        );

        svc.ingest(record, payload, None)
            .await
            .expect("async ingest should succeed");

        let stored = get_object_from_s3(&endpoint, &access_key, &secret_key, &bucket, key).await;
        assert!(stored.is_some(), "object must exist in S3 after async accepted ingest");
        assert_eq!(stored.unwrap().as_slice(), payload);
    }

    #[tokio::test]
    async fn async_ingest_rejected_does_not_upload_to_minio() {
        let Some((endpoint, access_key, secret_key, bucket)) = s3_env() else {
            eprintln!(
                "async_ingest: skipping MinIO test \
                 (TEST_S3_ENDPOINT/KEY/BUCKET not set)"
            );
            return;
        };

        let config = S3ObjectStoreConfig::for_minio(
            bucket.clone(),
            "us-east-1",
            &endpoint,
            &access_key,
            &secret_key,
        );

        let signing_key = SigningKey::from_bytes(&[51u8; 32]);
        let verifying_key = VerifyingKey::from(&signing_key);

        let raw_store =
            S3CompatibleRawDataStore::new(config).expect("S3 store should initialise");
        let svc = AsyncIngestService::new(
            IntegrityPolicyGate::default(),
            raw_store,
            AsyncInMemoryAuditLedger::default(),
            AsyncInMemoryOperationLog::default(),
        );
        svc.register_device("lift-async-test-02", verifying_key).await;

        let key = "test/async-s3-integration-rejected.bin";
        let record = build_signed_record(
            "lift-async-test-02",
            1,
            1_700_000_001_001,
            b"original",
            AuditRecord::zero_hash(),
            key,
            &signing_key,
        );

        svc.ingest(record, b"tampered", None)
            .await
            .expect_err("hash mismatch must be rejected");

        let stored = get_object_from_s3(&endpoint, &access_key, &secret_key, &bucket, key).await;
        assert!(stored.is_none(), "rejected async ingest must not write to S3");
    }
}
