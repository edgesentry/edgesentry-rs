//! Integration tests for software update integrity verification (CLS-03).
//!
//! These tests exercise `UpdateVerifier` end-to-end together with the ingest
//! audit pipeline: a tampered-update rejection is recorded in
//! `UpdateVerificationLog`, and the same device that produced the update can
//! still submit valid audit records through `IngestService`.

use ed25519_dalek::SigningKey;
use edgesentry_rs::{
    build_signed_record,
    identity::sign_payload_hash,
    integrity::compute_payload_hash,
    update::{SoftwareUpdate, UpdateVerificationLog, UpdateVerifyDecision, UpdateVerifier},
    AuditRecord, InMemoryAuditLedger, InMemoryOperationLog, InMemoryRawDataStore, IngestService,
    IntegrityPolicyGate,
};

fn signed_update(payload: &[u8], signing_key: &SigningKey, package_id: &str, version: &str) -> SoftwareUpdate {
    let payload_hash = compute_payload_hash(payload);
    SoftwareUpdate {
        package_id:   package_id.to_string(),
        version:      version.to_string(),
        payload_hash,
        signature:    sign_payload_hash(signing_key, &payload_hash),
    }
}

/// Full happy path: update verified, then device audit records ingested.
///
/// Simulates a device that:
/// 1. Receives a firmware update and verifies it before applying
/// 2. After applying, submits inspection audit records to the cloud backend
#[test]
fn verified_update_followed_by_audit_ingest() {
    let publisher_key = SigningKey::from_bytes(&[50u8; 32]);
    let device_key    = SigningKey::from_bytes(&[51u8; 32]);
    let device_id     = "lift-02";

    // ── Update verification (edge device side) ───────────────────────────────
    let fw_payload = b"firmware-v2.0.0-lift-02";
    let update = signed_update(fw_payload, &publisher_key, "lift-fw", "2.0.0");

    let mut verifier = UpdateVerifier::new();
    verifier.register_publisher("acme-firmware", publisher_key.verifying_key());

    let mut update_log = UpdateVerificationLog::default();
    verifier
        .verify(&update, fw_payload, "acme-firmware", &mut update_log)
        .expect("valid update should be accepted");

    assert_eq!(update_log.entries().len(), 1);
    assert_eq!(update_log.entries()[0].decision, UpdateVerifyDecision::Accepted);

    // ── Audit record ingest (cloud backend side) ─────────────────────────────
    let mut ingest = IngestService::new(
        IntegrityPolicyGate::default(),
        InMemoryRawDataStore::default(),
        InMemoryAuditLedger::default(),
        InMemoryOperationLog::default(),
    );
    ingest.register_device(device_id, device_key.verifying_key());

    let payload = b"check=door,status=ok,post_update=true";
    let record = build_signed_record(
        device_id,
        1,
        1_720_000_000_000,
        payload,
        AuditRecord::zero_hash(),
        "s3://bucket/lift-02/1.bin",
        &device_key,
    );

    ingest
        .ingest(record, payload, Some(device_id))
        .expect("audit record should be ingested after update");

    assert_eq!(ingest.audit_ledger().records().len(), 1);
}

/// Tampered update is rejected and logged; subsequent valid update still passes.
#[test]
fn tampered_update_rejected_then_valid_update_accepted() {
    let publisher_key = SigningKey::from_bytes(&[52u8; 32]);

    let real_payload    = b"firmware-v3.0.0-real";
    let tampered_payload = b"firmware-v3.0.0-hacked";
    let update = signed_update(real_payload, &publisher_key, "lift-fw", "3.0.0");

    let mut verifier = UpdateVerifier::new();
    verifier.register_publisher("acme", publisher_key.verifying_key());

    let mut log = UpdateVerificationLog::default();

    // Attempt with tampered payload — must be rejected
    let err = verifier
        .verify(&update, tampered_payload, "acme", &mut log)
        .unwrap_err();
    assert!(format!("{err}").contains("payload hash mismatch"));

    // Attempt with real payload — must be accepted
    verifier
        .verify(&update, real_payload, "acme", &mut log)
        .expect("real payload should be accepted");

    assert_eq!(log.entries().len(), 2);
    assert_eq!(log.entries()[0].decision, UpdateVerifyDecision::Rejected);
    assert_eq!(log.entries()[1].decision, UpdateVerifyDecision::Accepted);
}

/// Untrusted publisher cannot push updates even with a valid payload.
#[test]
fn untrusted_publisher_cannot_install_update_and_ingest_is_unaffected() {
    let trusted_key   = SigningKey::from_bytes(&[53u8; 32]);
    let attacker_key  = SigningKey::from_bytes(&[54u8; 32]);
    let device_key    = SigningKey::from_bytes(&[55u8; 32]);
    let device_id     = "lift-03";

    let payload = b"malicious-firmware";
    let malicious_update = signed_update(payload, &attacker_key, "lift-fw", "3.0.0");

    let mut verifier = UpdateVerifier::new();
    verifier.register_publisher("trusted-vendor", trusted_key.verifying_key());
    // attacker's key is NOT registered

    let mut update_log = UpdateVerificationLog::default();
    verifier
        .verify(&malicious_update, payload, "trusted-vendor", &mut update_log)
        .expect_err("attacker-signed update must be rejected");

    assert_eq!(update_log.entries()[0].decision, UpdateVerifyDecision::Rejected);

    // The device's audit ingest pipeline is completely independent — still works
    let mut ingest = IngestService::new(
        IntegrityPolicyGate::default(),
        InMemoryRawDataStore::default(),
        InMemoryAuditLedger::default(),
        InMemoryOperationLog::default(),
    );
    ingest.register_device(device_id, device_key.verifying_key());

    let audit_payload = b"check=vibration,status=ok";
    let record = build_signed_record(
        device_id,
        1,
        1_720_000_001_000,
        audit_payload,
        AuditRecord::zero_hash(),
        "s3://bucket/lift-03/1.bin",
        &device_key,
    );

    ingest
        .ingest(record, audit_payload, Some(device_id))
        .expect("audit ingest must work regardless of update rejection");
}
