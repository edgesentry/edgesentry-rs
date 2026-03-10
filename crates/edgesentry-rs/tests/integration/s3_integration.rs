//! S3/MinIO integration tests.
//!
//! These tests require a running MinIO (or S3-compatible) instance.
//! Set the following environment variables to enable them:
//!
//!   TEST_S3_ENDPOINT   e.g. http://localhost:9000
//!   TEST_S3_ACCESS_KEY e.g. minioadmin
//!   TEST_S3_SECRET_KEY e.g. minioadmin
//!   TEST_S3_BUCKET     e.g. bucket
//!
//! Tests skip automatically when any variable is unset.

#![cfg(feature = "s3")]

use ed25519_dalek::{SigningKey, VerifyingKey};
    use edgesentry_rs::{
        build_signed_record, AuditRecord, InMemoryAuditLedger, InMemoryOperationLog,
        IngestService, IngestServiceError, IntegrityPolicyGate, S3CompatibleRawDataStore,
        S3ObjectStoreConfig,
    };

    /// Returns (endpoint, access_key, secret_key, bucket) if all env vars are set.
    fn s3_env() -> Option<(String, String, String, String)> {
        let endpoint = std::env::var("TEST_S3_ENDPOINT").ok()?;
        let access_key = std::env::var("TEST_S3_ACCESS_KEY").ok()?;
        let secret_key = std::env::var("TEST_S3_SECRET_KEY").ok()?;
        let bucket = std::env::var("TEST_S3_BUCKET").ok()?;
        Some((endpoint, access_key, secret_key, bucket))
    }

    /// Strip a `s3://bucket/` URI prefix from an object reference, returning the bare key.
    fn s3_key(object_ref: &str) -> &str {
        if let Some(rest) = object_ref.strip_prefix("s3://") {
            if let Some(slash) = rest.find('/') {
                return &rest[slash + 1..];
            }
        }
        object_ref
    }

    /// Fetch an object from MinIO; returns `None` when the key does not exist.
    fn get_object(
        endpoint: &str,
        access_key: &str,
        secret_key: &str,
        bucket: &str,
        key: &str,
    ) -> Option<Vec<u8>> {
        let key = s3_key(key);
        use aws_config::BehaviorVersion;
        use aws_config::Region;
        use aws_credential_types::Credentials;

        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async {
            let creds =
                Credentials::new(access_key, secret_key, None, None, "static");
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
        })
    }

    #[test]
    fn accepted_ingest_uploads_payload_to_s3() {
        let Some((endpoint, access_key, secret_key, bucket)) = s3_env() else {
            eprintln!(
                "s3_integration: skipping accepted_ingest_uploads_payload_to_s3 \
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

        let signing_key = SigningKey::from_bytes(&[41u8; 32]);
        let verifying_key = VerifyingKey::from(&signing_key);

        let raw_store =
            S3CompatibleRawDataStore::new(config).expect("S3 store should initialise");
        let mut service = IngestService::new(
            IntegrityPolicyGate::default(),
            raw_store,
            InMemoryAuditLedger::default(),
            InMemoryOperationLog::default(),
        );
        service.register_device("lift-s3-test-01", verifying_key);

        let payload = b"s3-integration-test-accepted-payload";
        let key = "test/s3-integration-accepted.bin";
        let record = build_signed_record(
            "lift-s3-test-01",
            1,
            1_700_000_000_001,
            payload,
            AuditRecord::zero_hash(),
            key,
            &signing_key,
        );

        service.ingest(record, payload, None).expect("ingest should succeed");

        let stored = get_object(&endpoint, &access_key, &secret_key, &bucket, key);
        assert!(stored.is_some(), "object must exist in S3 after accepted ingest");
        assert_eq!(
            stored.unwrap().as_slice(),
            payload,
            "stored bytes must match the original payload exactly"
        );
    }

    #[test]
    fn rejected_ingest_does_not_write_to_s3() {
        let Some((endpoint, access_key, secret_key, bucket)) = s3_env() else {
            eprintln!(
                "s3_integration: skipping rejected_ingest_does_not_write_to_s3 \
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

        let signing_key = SigningKey::from_bytes(&[42u8; 32]);
        let verifying_key = VerifyingKey::from(&signing_key);

        let raw_store =
            S3CompatibleRawDataStore::new(config).expect("S3 store should initialise");
        let mut service = IngestService::new(
            IntegrityPolicyGate::default(),
            raw_store,
            InMemoryAuditLedger::default(),
            InMemoryOperationLog::default(),
        );
        service.register_device("lift-s3-test-02", verifying_key);

        // Build a valid record but present a tampered payload — hash mismatch triggers rejection.
        let key = "test/s3-integration-rejected.bin";
        let record = build_signed_record(
            "lift-s3-test-02",
            1,
            1_700_000_000_002,
            b"original-payload",
            AuditRecord::zero_hash(),
            key,
            &signing_key,
        );

        let err = service
            .ingest(record, b"tampered-payload", None)
            .expect_err("ingest must be rejected on payload hash mismatch");
        assert!(
            matches!(err, IngestServiceError::PayloadHashMismatch { .. }),
            "expected PayloadHashMismatch, got: {err}"
        );

        let stored = get_object(&endpoint, &access_key, &secret_key, &bucket, key);
        assert!(
            stored.is_none(),
            "rejected ingest must not write any object to S3"
        );
    }

    #[test]
    fn accepted_ingest_key_matches_object_ref() {
        // Verify that the S3 key used matches the record's object_ref field exactly.
        let Some((endpoint, access_key, secret_key, bucket)) = s3_env() else {
            eprintln!(
                "s3_integration: skipping accepted_ingest_key_matches_object_ref \
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

        let signing_key = SigningKey::from_bytes(&[43u8; 32]);
        let verifying_key = VerifyingKey::from(&signing_key);

        let raw_store =
            S3CompatibleRawDataStore::new(config).expect("S3 store should initialise");
        let mut service = IngestService::new(
            IntegrityPolicyGate::default(),
            raw_store,
            InMemoryAuditLedger::default(),
            InMemoryOperationLog::default(),
        );
        service.register_device("lift-s3-test-03", verifying_key);

        let payload = b"key-match-test-payload";
        let object_ref = "test/s3-integration-key-match.bin";
        let record = build_signed_record(
            "lift-s3-test-03",
            1,
            1_700_000_000_003,
            payload,
            AuditRecord::zero_hash(),
            object_ref,
            &signing_key,
        );

        service.ingest(record, payload, None).expect("ingest should succeed");

        // Fetch using the exact object_ref — must be present and correct.
        let stored = get_object(&endpoint, &access_key, &secret_key, &bucket, object_ref);
        assert!(
            stored.is_some(),
            "object must be stored under the record's object_ref key"
        );
        assert_eq!(
            stored.unwrap().as_slice(),
            payload,
            "content stored under object_ref must equal the original payload"
        );

        // A slightly different key must not exist.
        let wrong_key = "test/s3-integration-key-match-WRONG.bin";
        let absent = get_object(&endpoint, &access_key, &secret_key, &bucket, wrong_key);
        assert!(
            absent.is_none(),
            "no object should exist under a key that was never written"
        );
    }
