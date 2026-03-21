//! Integration tests for [`edgesentry_rs::buffer::OfflineBuffer`].
//!
//! These tests use only in-memory components (no external services required).

use ed25519_dalek::SigningKey;
use edgesentry_rs::{
    build_lift_inspection_demo_records_with_payloads, parse_fixed_hex, AuditRecord,
    FlushReport, InMemoryAuditLedger, InMemoryBufferStore, InMemoryOperationLog,
    InMemoryRawDataStore, IngestService, IntegrityPolicyGate, OfflineBuffer,
};

const PRIV_HEX: &str = "0101010101010101010101010101010101010101010101010101010101010101";

fn make_service(
) -> IngestService<InMemoryRawDataStore, InMemoryAuditLedger, InMemoryOperationLog> {
    let key_bytes = parse_fixed_hex::<32>(PRIV_HEX).unwrap();
    let signing_key = SigningKey::from_bytes(&key_bytes);
    let verifying_key = signing_key.verifying_key();
    let mut policy = IntegrityPolicyGate::new();
    policy.register_device("lift-01", verifying_key);
    IngestService::new(
        policy,
        InMemoryRawDataStore::default(),
        InMemoryAuditLedger::default(),
        InMemoryOperationLog::default(),
    )
}

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
// Basic buffer behaviour
// ---------------------------------------------------------------------------

#[test]
fn buffer_starts_empty() {
    let buf: OfflineBuffer<InMemoryBufferStore> = OfflineBuffer::new(InMemoryBufferStore::new());
    assert_eq!(buf.len().unwrap(), 0);
    assert!(buf.is_empty().unwrap());
}

#[test]
fn push_increments_len() {
    let mut buf: OfflineBuffer<InMemoryBufferStore> =
        OfflineBuffer::new(InMemoryBufferStore::new());
    let pairs = demo_pairs();
    buf.push(pairs[0].0.clone(), pairs[0].1.clone()).unwrap();
    assert_eq!(buf.len().unwrap(), 1);
    buf.push(pairs[1].0.clone(), pairs[1].1.clone()).unwrap();
    assert_eq!(buf.len().unwrap(), 2);
    buf.push(pairs[2].0.clone(), pairs[2].1.clone()).unwrap();
    assert_eq!(buf.len().unwrap(), 3);
}

// ---------------------------------------------------------------------------
// Flush — success path
// ---------------------------------------------------------------------------

#[test]
fn flush_delivers_all_records_and_empties_buffer() {
    let mut buf: OfflineBuffer<InMemoryBufferStore> =
        OfflineBuffer::new(InMemoryBufferStore::new());
    let pairs = demo_pairs();
    for (r, p) in &pairs {
        buf.push(r.clone(), p.clone()).unwrap();
    }

    let mut svc = make_service();
    let report = buf.flush(&mut svc).unwrap();

    assert_eq!(
        report,
        FlushReport { accepted: 3, remaining: 0 }
    );
    assert!(buf.is_empty().unwrap());
    assert_eq!(svc.audit_ledger().records().len(), 3);
}

#[test]
fn flush_is_idempotent_via_duplicate_handling() {
    let mut buf: OfflineBuffer<InMemoryBufferStore> =
        OfflineBuffer::new(InMemoryBufferStore::new());
    let pairs = demo_pairs();
    for (r, p) in &pairs {
        buf.push(r.clone(), p.clone()).unwrap();
    }

    let mut svc = make_service();
    buf.flush(&mut svc).unwrap();

    // Re-push the same records and flush again — all should be treated as duplicates.
    for (r, p) in &pairs {
        buf.push(r.clone(), p.clone()).unwrap();
    }
    let report = buf.flush(&mut svc).unwrap();
    assert_eq!(report.accepted, 3, "duplicates must be counted as accepted");
    assert_eq!(report.remaining, 0);
}

// ---------------------------------------------------------------------------
// Flush — failure path
// ---------------------------------------------------------------------------

#[test]
fn flush_stops_and_preserves_buffer_on_unknown_device() {
    let unknown_pairs = build_lift_inspection_demo_records_with_payloads(
        "unknown-device",
        PRIV_HEX,
        1_700_000_000_000,
        "s3://bucket/unknown",
    )
    .unwrap();

    let mut buf: OfflineBuffer<InMemoryBufferStore> =
        OfflineBuffer::new(InMemoryBufferStore::new());
    for (r, p) in &unknown_pairs {
        buf.push(r.clone(), p.clone()).unwrap();
    }

    let mut svc = make_service(); // "unknown-device" NOT registered
    let result = buf.flush(&mut svc);
    assert!(result.is_err(), "flush must fail for unknown device");
    // Records must remain buffered so the next flush can retry.
    assert_eq!(buf.len().unwrap(), unknown_pairs.len());
}

// ---------------------------------------------------------------------------
// Partial flush — accepted records are removed, failed record stays
// ---------------------------------------------------------------------------

#[test]
fn partial_flush_removes_accepted_prefix() {
    // Push two good records followed by one from an unregistered device.
    let good_pairs = demo_pairs();
    let bad_pairs = build_lift_inspection_demo_records_with_payloads(
        "unknown-device",
        PRIV_HEX,
        1_700_000_000_000,
        "s3://bucket/unknown",
    )
    .unwrap();

    let mut buf: OfflineBuffer<InMemoryBufferStore> =
        OfflineBuffer::new(InMemoryBufferStore::new());
    buf.push(good_pairs[0].0.clone(), good_pairs[0].1.clone()).unwrap();
    buf.push(good_pairs[1].0.clone(), good_pairs[1].1.clone()).unwrap();
    buf.push(bad_pairs[0].0.clone(), bad_pairs[0].1.clone()).unwrap();

    let mut svc = make_service();
    let result = buf.flush(&mut svc);
    assert!(result.is_err());
    // The two good records should have been consumed; the bad one stays.
    assert_eq!(buf.len().unwrap(), 1);
}
