use ed25519_dalek::{SigningKey, VerifyingKey};
use edgesentry_rs::{
    build_signed_record, AuditRecord, InMemoryAuditLedger, InMemoryOperationLog,
    InMemoryRawDataStore, IngestDecision, IngestService, IngestServiceError, IngestState,
};

#[test]
fn persists_raw_data_audit_ledger_and_operation_log() {
    let signing_key = SigningKey::from_bytes(&[61u8; 32]);
    let verifying_key = VerifyingKey::from(&signing_key);

    let mut service = IngestService::new(
        IngestState::default(),
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

    service.ingest(record.clone(), payload).expect("ingest should succeed");

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
        IngestState::default(),
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
        .ingest(record, b"tampered-payload")
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
