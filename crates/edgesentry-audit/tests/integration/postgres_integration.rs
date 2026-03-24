//! PostgreSQL integration tests for `PostgresAuditLedger` and `PostgresOperationLog`.
//!
//! These tests require a running PostgreSQL instance initialised with
//! `db/init/001_schema.sql`.  Set the following environment variable to enable:
//!
//!   TEST_POSTGRES_URL   e.g. postgres://postgres:password@localhost/edgesentry_test
//!
//! Tests skip automatically when the variable is unset.
//!
//! Run with:
//!
//! ```bash
//! TEST_POSTGRES_URL=postgres://postgres:password@localhost/edgesentry_test \
//!   cargo test -p edgesentry-rs --test integration --features postgres postgres_integration
//! ```

#![cfg(feature = "postgres")]

use ed25519_dalek::{SigningKey, VerifyingKey};
use edgesentry_audit::{
    build_signed_record, AuditLedger, AuditRecord, InMemoryRawDataStore, IngestService,
    IngestServiceError, IntegrityPolicyGate, PostgresAuditLedger, PostgresOperationLog,
    PostgresStoreError,
};

/// Returns the Postgres URL if `TEST_POSTGRES_URL` is set.
fn postgres_url() -> Option<String> {
    std::env::var("TEST_POSTGRES_URL").ok()
}

/// Open a raw postgres client for read-back queries in tests.
fn connect(url: &str) -> ::postgres::Client {
    ::postgres::Client::connect(url, ::postgres::NoTls).expect("postgres connect")
}

/// Truncate both tables so each test starts with an empty database.
fn reset(url: &str) {
    connect(url)
        .batch_execute("TRUNCATE TABLE operation_logs, audit_records RESTART IDENTITY;")
        .expect("reset failed");
}

// ── PostgresAuditLedger ───────────────────────────────────────────────────────

#[test]
fn audit_ledger_persists_accepted_record() {
    let Some(url) = postgres_url() else {
        eprintln!("postgres_integration: skipping (TEST_POSTGRES_URL not set)");
        return;
    };

    reset(&url);

    let signing_key = SigningKey::from_bytes(&[51u8; 32]);
    let verifying_key = VerifyingKey::from(&signing_key);

    let ledger = PostgresAuditLedger::connect(&url).expect("audit ledger connect");
    let op_log = PostgresOperationLog::connect(&url).expect("op log connect");
    let mut service = IngestService::new(
        IntegrityPolicyGate::default(),
        InMemoryRawDataStore::default(),
        ledger,
        op_log,
    );
    service.register_device("pg-device-01", verifying_key);

    let payload = b"pg-integration-accepted-payload";
    let record = build_signed_record(
        "pg-device-01",
        1,
        1_700_000_001_000,
        payload,
        AuditRecord::zero_hash(),
        "pg/device-01/1.bin",
        &signing_key,
    );

    service.ingest(record, payload, None).expect("ingest should succeed");

    let row = connect(&url)
        .query_one(
            "SELECT COUNT(*) FROM audit_records WHERE device_id = 'pg-device-01'",
            &[],
        )
        .expect("count query must succeed");
    let count: i64 = row.get(0);
    assert_eq!(count, 1, "one audit record must be persisted");
}

#[test]
fn audit_ledger_binary_fields_round_trip_exactly() {
    // Regression guard for #133: payload_hash (32 B), signature (64 B), and
    // prev_record_hash (32 B) must survive a Postgres round-trip as exact byte
    // sequences — not as JSON-encoded integer arrays.
    let Some(url) = postgres_url() else {
        eprintln!("postgres_integration: skipping (TEST_POSTGRES_URL not set)");
        return;
    };

    reset(&url);

    let signing_key = SigningKey::from_bytes(&[52u8; 32]);

    let payload = b"binary-round-trip-test";
    let record = build_signed_record(
        "pg-device-rt",
        1,
        1_700_000_002_000,
        payload,
        AuditRecord::zero_hash(),
        "pg/device-rt/1.bin",
        &signing_key,
    );

    let expected_hash = record.payload_hash;
    let expected_sig = record.signature;
    let expected_prev = record.prev_record_hash;

    let mut ledger = PostgresAuditLedger::connect(&url).expect("audit ledger connect");
    ledger.append(record).expect("append should succeed");

    let row = connect(&url)
        .query_one(
            "SELECT payload_hash, signature, prev_record_hash \
             FROM audit_records WHERE device_id = 'pg-device-rt' AND sequence = 1",
            &[],
        )
        .expect("row must exist");

    let stored_hash: Vec<u8> = row.get(0);
    let stored_sig: Vec<u8> = row.get(1);
    let stored_prev: Vec<u8> = row.get(2);

    assert_eq!(stored_hash.len(), 32, "payload_hash must be exactly 32 bytes");
    assert_eq!(stored_sig.len(), 64, "signature must be exactly 64 bytes");
    assert_eq!(stored_prev.len(), 32, "prev_record_hash must be exactly 32 bytes");
    assert_eq!(stored_hash.as_slice(), &expected_hash, "payload_hash must survive round-trip exactly");
    assert_eq!(stored_sig.as_slice(), &expected_sig, "signature must survive round-trip exactly");
    assert_eq!(stored_prev.as_slice(), &expected_prev, "prev_record_hash must survive round-trip exactly");
}

#[test]
fn audit_ledger_rejects_duplicate_sequence() {
    // The UNIQUE (device_id, sequence) constraint must cause a Postgres error
    // when two records with the same device+sequence are inserted.
    let Some(url) = postgres_url() else {
        eprintln!("postgres_integration: skipping (TEST_POSTGRES_URL not set)");
        return;
    };

    reset(&url);

    let signing_key = SigningKey::from_bytes(&[53u8; 32]);

    let payload = b"first-payload";
    let first = build_signed_record(
        "pg-device-dup",
        1,
        1_700_000_003_000,
        payload,
        AuditRecord::zero_hash(),
        "pg/device-dup/1.bin",
        &signing_key,
    );

    let mut ledger = PostgresAuditLedger::connect(&url).expect("ledger connect");
    ledger.append(first).expect("first append should succeed");

    let dup = build_signed_record(
        "pg-device-dup",
        1,                          // same device_id + sequence
        1_700_000_003_001,
        b"duplicate-payload",
        AuditRecord::zero_hash(),
        "pg/device-dup/1b.bin",
        &signing_key,
    );

    let result = ledger.append(dup);
    assert!(result.is_err(), "duplicate (device_id, sequence) must be rejected by UNIQUE constraint");
    assert!(
        matches!(result.unwrap_err(), PostgresStoreError::Postgres(_)),
        "error must be a Postgres unique-violation error"
    );
}

#[test]
fn audit_ledger_persists_sequential_records_in_order() {
    let Some(url) = postgres_url() else {
        eprintln!("postgres_integration: skipping (TEST_POSTGRES_URL not set)");
        return;
    };

    reset(&url);

    let signing_key = SigningKey::from_bytes(&[55u8; 32]);
    let verifying_key = VerifyingKey::from(&signing_key);

    let ledger = PostgresAuditLedger::connect(&url).expect("ledger connect");
    let op_log = PostgresOperationLog::connect(&url).expect("op log connect");
    let mut service = IngestService::new(
        IntegrityPolicyGate::default(),
        InMemoryRawDataStore::default(),
        ledger,
        op_log,
    );
    service.register_device("pg-device-seq", verifying_key);

    let payloads: &[&[u8]] = &[b"check=door", b"check=vibration", b"check=brake"];
    let mut prev_hash = AuditRecord::zero_hash();

    for (i, payload) in payloads.iter().enumerate() {
        let seq = (i as u64) + 1;
        let record = build_signed_record(
            "pg-device-seq",
            seq,
            1_700_000_004_000 + seq,
            payload,
            prev_hash,
            format!("pg/device-seq/{seq}.bin"),
            &signing_key,
        );
        prev_hash = record.hash();
        service
            .ingest(record, payload, None)
            .unwrap_or_else(|e| panic!("ingest #{seq} should succeed: {e}"));
    }

    let row = connect(&url)
        .query_one(
            "SELECT COUNT(*) FROM audit_records WHERE device_id = 'pg-device-seq'",
            &[],
        )
        .expect("count query must succeed");
    let count: i64 = row.get(0);
    assert_eq!(count, 3, "all 3 sequential records must be stored in Postgres");

    // Verify sequence numbers are monotonically stored.
    let rows = connect(&url)
        .query(
            "SELECT sequence FROM audit_records \
             WHERE device_id = 'pg-device-seq' ORDER BY sequence ASC",
            &[],
        )
        .expect("sequence query must succeed");
    let sequences: Vec<i64> = rows.iter().map(|r| r.get(0)).collect();
    assert_eq!(sequences, vec![1, 2, 3], "sequences must be 1, 2, 3 in order");
}

// ── PostgresOperationLog ──────────────────────────────────────────────────────

#[test]
fn operation_log_persists_accepted_entry() {
    let Some(url) = postgres_url() else {
        eprintln!("postgres_integration: skipping (TEST_POSTGRES_URL not set)");
        return;
    };

    reset(&url);

    let signing_key = SigningKey::from_bytes(&[61u8; 32]);
    let verifying_key = VerifyingKey::from(&signing_key);

    let ledger = PostgresAuditLedger::connect(&url).expect("ledger connect");
    let op_log = PostgresOperationLog::connect(&url).expect("op log connect");
    let mut service = IngestService::new(
        IntegrityPolicyGate::default(),
        InMemoryRawDataStore::default(),
        ledger,
        op_log,
    );
    service.register_device("pg-oplog-01", verifying_key);

    let payload = b"oplog-accepted-payload";
    let record = build_signed_record(
        "pg-oplog-01",
        1,
        1_700_000_005_000,
        payload,
        AuditRecord::zero_hash(),
        "pg/oplog-01/1.bin",
        &signing_key,
    );

    service.ingest(record, payload, None).expect("ingest should succeed");

    let row = connect(&url)
        .query_one(
            "SELECT decision, device_id, sequence \
             FROM operation_logs WHERE device_id = 'pg-oplog-01' ORDER BY id DESC LIMIT 1",
            &[],
        )
        .expect("log entry must exist");

    let decision: &str = row.get(0);
    let device_id: &str = row.get(1);
    let sequence: i64 = row.get(2);

    assert_eq!(decision, "Accepted");
    assert_eq!(device_id, "pg-oplog-01");
    assert_eq!(sequence, 1);
}

#[test]
fn operation_log_persists_rejected_entry_on_hash_mismatch() {
    let Some(url) = postgres_url() else {
        eprintln!("postgres_integration: skipping (TEST_POSTGRES_URL not set)");
        return;
    };

    reset(&url);

    let signing_key = SigningKey::from_bytes(&[62u8; 32]);
    let verifying_key = VerifyingKey::from(&signing_key);

    let ledger = PostgresAuditLedger::connect(&url).expect("ledger connect");
    let op_log = PostgresOperationLog::connect(&url).expect("op log connect");
    let mut service = IngestService::new(
        IntegrityPolicyGate::default(),
        InMemoryRawDataStore::default(),
        ledger,
        op_log,
    );
    service.register_device("pg-oplog-02", verifying_key);

    let record = build_signed_record(
        "pg-oplog-02",
        1,
        1_700_000_006_000,
        b"original",
        AuditRecord::zero_hash(),
        "pg/oplog-02/1.bin",
        &signing_key,
    );

    let err = service
        .ingest(record, b"tampered", None)
        .expect_err("hash mismatch must be rejected");
    assert!(matches!(err, IngestServiceError::PayloadHashMismatch { .. }));

    let row = connect(&url)
        .query_one(
            "SELECT decision, device_id, sequence \
             FROM operation_logs WHERE device_id = 'pg-oplog-02' ORDER BY id DESC LIMIT 1",
            &[],
        )
        .expect("rejection log entry must exist");

    let decision: &str = row.get(0);
    assert_eq!(decision, "Rejected");
}

#[test]
fn operation_log_records_both_accepted_and_rejected_entries() {
    let Some(url) = postgres_url() else {
        eprintln!("postgres_integration: skipping (TEST_POSTGRES_URL not set)");
        return;
    };

    reset(&url);

    let signing_key = SigningKey::from_bytes(&[63u8; 32]);
    let verifying_key = VerifyingKey::from(&signing_key);

    let ledger = PostgresAuditLedger::connect(&url).expect("ledger connect");
    let op_log = PostgresOperationLog::connect(&url).expect("op log connect");
    let mut service = IngestService::new(
        IntegrityPolicyGate::default(),
        InMemoryRawDataStore::default(),
        ledger,
        op_log,
    );
    service.register_device("pg-oplog-03", verifying_key);

    // First ingest: accepted.
    let payload = b"valid-payload";
    let r1 = build_signed_record(
        "pg-oplog-03", 1, 1_700_000_007_000, payload,
        AuditRecord::zero_hash(), "pg/oplog-03/1.bin", &signing_key,
    );
    service.ingest(r1, payload, None).expect("first ingest should succeed");

    // Second ingest: rejected (tampered payload).
    let r2 = build_signed_record(
        "pg-oplog-03", 2, 1_700_000_007_001, b"original",
        AuditRecord::zero_hash(), "pg/oplog-03/2.bin", &signing_key,
    );
    let _ = service.ingest(r2, b"tampered", None);

    let rows = connect(&url)
        .query(
            "SELECT decision FROM operation_logs \
             WHERE device_id = 'pg-oplog-03' ORDER BY id ASC",
            &[],
        )
        .expect("log query must succeed");

    let decisions: Vec<&str> = rows.iter().map(|r| r.get(0)).collect();
    assert_eq!(decisions, vec!["Accepted", "Rejected"], "must log one Accepted then one Rejected");
}
