use ed25519_dalek::{SigningKey, VerifyingKey};
use edgesentry_rs::{
    build_signed_record, AuditRecord, InMemoryAuditLedger, InMemoryOperationLog,
    InMemoryRawDataStore, IngestDecision, IngestError, IngestService, IngestServiceError,
    IntegrityPolicyGate,
};

#[test]
fn persists_raw_data_audit_ledger_and_operation_log() {
    let signing_key = SigningKey::from_bytes(&[61u8; 32]);
    let verifying_key = VerifyingKey::from(&signing_key);

    let mut service = IngestService::new(
        IntegrityPolicyGate::default(),
        InMemoryRawDataStore::default(),
        InMemoryAuditLedger::default(),
        InMemoryOperationLog::default(),
    );
    service.register_device("lift-01", verifying_key);

    let payload = b"door-open";
    let record = build_signed_record(
        "lift-01",
        1,
        1,
        payload,
        AuditRecord::zero_hash(),
        "s3://bucket/lift-01/1.bin",
        &signing_key,
    );

    service.ingest(record.clone(), payload, Some("lift-01")).expect("ingest should succeed");

    let stored_raw = service
        .raw_data_store()
        .get("s3://bucket/lift-01/1.bin")
        .expect("raw data should be stored");
    assert_eq!(stored_raw, payload);

    let ledger_records = service.audit_ledger().records();
    assert_eq!(ledger_records.len(), 1);
    assert_eq!(ledger_records[0], record);

    let op_logs = service.operation_log().entries();
    assert_eq!(op_logs.len(), 1);
    assert_eq!(op_logs[0].decision, IngestDecision::Accepted);
    assert_eq!(op_logs[0].device_id, "lift-01");
    assert_eq!(op_logs[0].sequence, 1);
}

#[test]
fn rejects_payload_hash_mismatch_and_logs_rejection() {
    let signing_key = SigningKey::from_bytes(&[71u8; 32]);
    let verifying_key = VerifyingKey::from(&signing_key);

    let mut service = IngestService::new(
        IntegrityPolicyGate::default(),
        InMemoryRawDataStore::default(),
        InMemoryAuditLedger::default(),
        InMemoryOperationLog::default(),
    );
    service.register_device("lift-01", verifying_key);

    let record = build_signed_record(
        "lift-01",
        1,
        1,
        b"door-open",
        AuditRecord::zero_hash(),
        "s3://bucket/lift-01/1.bin",
        &signing_key,
    );

    let err = service
        .ingest(record, b"tampered-payload", Some("lift-01"))
        .expect_err("ingest should fail");
    assert!(matches!(
        err,
        IngestServiceError::PayloadHashMismatch { .. }
    ));

    assert!(
        service
            .raw_data_store()
            .get("s3://bucket/lift-01/1.bin")
            .is_none()
    );
    assert!(service.audit_ledger().records().is_empty());

    let logs = service.operation_log().entries();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].decision, IngestDecision::Rejected);
    assert_eq!(logs[0].device_id, "lift-01");
    assert_eq!(logs[0].sequence, 1);
}

#[test]
fn rejects_cert_device_mismatch_and_logs_rejection() {
    let signing_key = SigningKey::from_bytes(&[81u8; 32]);
    let verifying_key = VerifyingKey::from(&signing_key);

    let mut service = IngestService::new(
        IntegrityPolicyGate::default(),
        InMemoryRawDataStore::default(),
        InMemoryAuditLedger::default(),
        InMemoryOperationLog::default(),
    );
    service.register_device("lift-01", verifying_key);

    let payload = b"door-open";
    let record = build_signed_record(
        "lift-01",
        1,
        1,
        payload,
        AuditRecord::zero_hash(),
        "s3://bucket/lift-01/1.bin",
        &signing_key,
    );

    let err = service
        .ingest(record, payload, Some("spoofed-device"))
        .expect_err("ingest should fail on cert mismatch");

    assert!(
        matches!(
            err,
            IngestServiceError::Verify(IngestError::CertDeviceMismatch {
                ref cert_identity,
                ref device_id,
            }) if cert_identity == "spoofed-device" && device_id == "lift-01"
        ),
        "expected CertDeviceMismatch, got: {err}"
    );

    assert!(
        service
            .raw_data_store()
            .get("s3://bucket/lift-01/1.bin")
            .is_none()
    );
    assert!(service.audit_ledger().records().is_empty());

    let logs = service.operation_log().entries();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].decision, IngestDecision::Rejected);
    assert_eq!(logs[0].device_id, "lift-01");
    assert_eq!(logs[0].sequence, 1);
    assert!(
        logs[0].message.contains("auth/device mismatch"),
        "rejection log should contain auth/device mismatch context, got: {}",
        logs[0].message
    );
}

// --- P0 integrity policy gate: acceptance-criteria tests ---

#[test]
fn rejects_tampered_signature_via_ingest_service() {
    let signing_key = SigningKey::from_bytes(&[11u8; 32]);
    let verifying_key = VerifyingKey::from(&signing_key);

    let mut service = IngestService::new(
        IntegrityPolicyGate::default(),
        InMemoryRawDataStore::default(),
        InMemoryAuditLedger::default(),
        InMemoryOperationLog::default(),
    );
    service.register_device("lift-01", verifying_key);

    let payload = b"door-open";
    let mut record = build_signed_record(
        "lift-01",
        1,
        1,
        payload,
        AuditRecord::zero_hash(),
        "s3://bucket/lift-01/1.bin",
        &signing_key,
    );
    record.signature[0] ^= 0x01;

    let err = service
        .ingest(record, payload, Some("lift-01"))
        .expect_err("tampered signature must be rejected");
    assert!(
        matches!(err, IngestServiceError::Verify(IngestError::InvalidSignature(_))),
        "expected InvalidSignature, got: {err}"
    );

    assert!(service.audit_ledger().records().is_empty());
    let logs = service.operation_log().entries();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].decision, IngestDecision::Rejected);
}

#[test]
fn rejects_replay_attempt_via_ingest_service() {
    let signing_key = SigningKey::from_bytes(&[22u8; 32]);
    let verifying_key = VerifyingKey::from(&signing_key);

    let mut service = IngestService::new(
        IntegrityPolicyGate::default(),
        InMemoryRawDataStore::default(),
        InMemoryAuditLedger::default(),
        InMemoryOperationLog::default(),
    );
    service.register_device("lift-01", verifying_key);

    let payload = b"door-open";
    let record = build_signed_record(
        "lift-01",
        1,
        1,
        payload,
        AuditRecord::zero_hash(),
        "s3://bucket/lift-01/1.bin",
        &signing_key,
    );

    service.ingest(record.clone(), payload, None).expect("first ingest must succeed");

    // Replay: same sequence number again
    let replay = build_signed_record(
        "lift-01",
        1,
        2,
        payload,
        AuditRecord::zero_hash(),
        "s3://bucket/lift-01/1b.bin",
        &signing_key,
    );
    let err = service
        .ingest(replay, payload, None)
        .expect_err("replay must be rejected");
    assert!(
        matches!(err, IngestServiceError::Verify(IngestError::Duplicate { .. })),
        "expected Duplicate, got: {err}"
    );

    // Only the first record should be persisted
    assert_eq!(service.audit_ledger().records().len(), 1);
    let logs = service.operation_log().entries();
    assert_eq!(logs.len(), 2);
    assert_eq!(logs[0].decision, IngestDecision::Accepted);
    assert_eq!(logs[1].decision, IngestDecision::Rejected);
}

#[test]
fn rejects_tampered_signature_and_logs_rejection() {
    let signing_key = SigningKey::from_bytes(&[93u8; 32]);
    let verifying_key = VerifyingKey::from(&signing_key);

    let mut service = IngestService::new(
        IntegrityPolicyGate::default(),
        InMemoryRawDataStore::default(),
        InMemoryAuditLedger::default(),
        InMemoryOperationLog::default(),
    );
    service.register_device("lift-01", verifying_key);

    let payload = b"door-open";
    let mut record = build_signed_record(
        "lift-01",
        1,
        1,
        payload,
        AuditRecord::zero_hash(),
        "s3://bucket/lift-01/1.bin",
        &signing_key,
    );
    record.signature[0] ^= 0x01;

    let err = service
        .ingest(record, payload, Some("lift-01"))
        .expect_err("ingest should fail on tampered signature");

    assert!(
        matches!(
            err,
            IngestServiceError::Verify(IngestError::InvalidSignature(ref id)) if id == "lift-01"
        ),
        "expected InvalidSignature, got: {err}"
    );

    assert!(
        service
            .raw_data_store()
            .get("s3://bucket/lift-01/1.bin")
            .is_none()
    );
    assert!(service.audit_ledger().records().is_empty());

    let logs = service.operation_log().entries();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].decision, IngestDecision::Rejected);
    assert_eq!(logs[0].device_id, "lift-01");
    assert_eq!(logs[0].sequence, 1);
}

#[test]
fn rejects_replay_and_logs_rejection() {
    let signing_key = SigningKey::from_bytes(&[95u8; 32]);
    let verifying_key = VerifyingKey::from(&signing_key);

    let mut service = IngestService::new(
        IntegrityPolicyGate::default(),
        InMemoryRawDataStore::default(),
        InMemoryAuditLedger::default(),
        InMemoryOperationLog::default(),
    );
    service.register_device("lift-01", verifying_key);

    let payload = b"door-open";
    let record = build_signed_record(
        "lift-01",
        1,
        1,
        payload,
        AuditRecord::zero_hash(),
        "s3://bucket/lift-01/1.bin",
        &signing_key,
    );

    service
        .ingest(record.clone(), payload, Some("lift-01"))
        .expect("first ingest should succeed");

    // Replay with a distinct object_ref so we can assert the store was not written on rejection
    let mut replay = record;
    replay.object_ref = "s3://bucket/lift-01/1-replay.bin".to_string();

    let err = service
        .ingest(replay, payload, Some("lift-01"))
        .expect_err("replay ingest should fail");

    assert!(
        matches!(
            err,
            IngestServiceError::Verify(IngestError::Duplicate {
                ref device_id,
                sequence: 1,
            }) if device_id == "lift-01"
        ),
        "expected Duplicate, got: {err}"
    );

    assert_eq!(service.audit_ledger().records().len(), 1);
    assert!(
        service
            .raw_data_store()
            .get("s3://bucket/lift-01/1-replay.bin")
            .is_none(),
        "replayed record must not be written to the raw data store"
    );

    let logs = service.operation_log().entries();
    assert_eq!(logs.len(), 2);
    assert_eq!(logs[0].decision, IngestDecision::Accepted);
    assert_eq!(logs[1].decision, IngestDecision::Rejected);
    assert_eq!(logs[1].device_id, "lift-01");
    assert_eq!(logs[1].sequence, 1);
}

#[test]
fn rejects_out_of_order_sequence_and_logs_rejection() {
    let signing_key = SigningKey::from_bytes(&[97u8; 32]);
    let verifying_key = VerifyingKey::from(&signing_key);

    let mut service = IngestService::new(
        IntegrityPolicyGate::default(),
        InMemoryRawDataStore::default(),
        InMemoryAuditLedger::default(),
        InMemoryOperationLog::default(),
    );
    service.register_device("lift-01", verifying_key);

    let payload1 = b"door-open";
    let r1 = build_signed_record(
        "lift-01",
        1,
        1,
        payload1,
        AuditRecord::zero_hash(),
        "s3://bucket/lift-01/1.bin",
        &signing_key,
    );
    service
        .ingest(r1.clone(), payload1, Some("lift-01"))
        .expect("first ingest should succeed");

    // Skip sequence 2, jump straight to 3
    let payload3 = b"vibration-ok";
    let r3 = build_signed_record(
        "lift-01",
        3,
        3,
        payload3,
        r1.hash(),
        "s3://bucket/lift-01/3.bin",
        &signing_key,
    );
    let err = service
        .ingest(r3, payload3, Some("lift-01"))
        .expect_err("out-of-order ingest should fail");

    assert!(
        matches!(
            err,
            IngestServiceError::Verify(IngestError::InvalidSequence {
                ref device_id,
                expected: 2,
                actual: 3,
            }) if device_id == "lift-01"
        ),
        "expected InvalidSequence, got: {err}"
    );

    assert_eq!(service.audit_ledger().records().len(), 1);
    assert!(
        service
            .raw_data_store()
            .get("s3://bucket/lift-01/3.bin")
            .is_none(),
        "out-of-order record must not be written to the raw data store"
    );

    let logs = service.operation_log().entries();
    assert_eq!(logs.len(), 2);
    assert_eq!(logs[0].decision, IngestDecision::Accepted);
    assert_eq!(logs[1].decision, IngestDecision::Rejected);
    assert_eq!(logs[1].device_id, "lift-01");
    assert_eq!(logs[1].sequence, 3);
}

#[test]
fn cert_identity_check_precedes_payload_hash_check() {
    // A request with a spoofed cert_identity AND a mismatched payload hash must
    // yield CertDeviceMismatch (not PayloadHashMismatch), confirming that the
    // identity gate runs before the payload integrity check.
    let signing_key = SigningKey::from_bytes(&[42u8; 32]);
    let verifying_key = VerifyingKey::from(&signing_key);

    let mut service = IngestService::new(
        IntegrityPolicyGate::default(),
        InMemoryRawDataStore::default(),
        InMemoryAuditLedger::default(),
        InMemoryOperationLog::default(),
    );
    service.register_device("lift-01", verifying_key);

    let record = build_signed_record(
        "lift-01",
        1,
        1,
        b"door-open",
        AuditRecord::zero_hash(),
        "s3://bucket/lift-01/1.bin",
        &signing_key,
    );

    let err = service
        .ingest(record, b"tampered-payload", Some("spoofed-device"))
        .expect_err("ingest should fail");

    assert!(
        matches!(
            err,
            IngestServiceError::Verify(IngestError::CertDeviceMismatch {
                ref cert_identity,
                ref device_id,
            }) if cert_identity == "spoofed-device" && device_id == "lift-01"
        ),
        "expected CertDeviceMismatch (not PayloadHashMismatch), got: {err}"
    );
}

#[test]
fn accepts_ingest_without_cert_identity() {
    let signing_key = SigningKey::from_bytes(&[91u8; 32]);
    let verifying_key = VerifyingKey::from(&signing_key);

    let mut service = IngestService::new(
        IntegrityPolicyGate::default(),
        InMemoryRawDataStore::default(),
        InMemoryAuditLedger::default(),
        InMemoryOperationLog::default(),
    );
    service.register_device("lift-01", verifying_key);

    let payload = b"door-open";
    let record = build_signed_record(
        "lift-01",
        1,
        1,
        payload,
        AuditRecord::zero_hash(),
        "s3://bucket/lift-01/1.bin",
        &signing_key,
    );

    service
        .ingest(record, payload, None)
        .expect("ingest without cert_identity should succeed");

    assert_eq!(service.audit_ledger().records().len(), 1);
}
