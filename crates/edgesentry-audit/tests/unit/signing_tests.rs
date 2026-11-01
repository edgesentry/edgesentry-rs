use ed25519_dalek::{SigningKey, VerifyingKey};
use edgesentry_audit::{
    build_signed_record, compute_payload_hash, verify_audit_record, AuditRecord,
};

#[test]
fn build_signed_record_creates_verifiable_signature() {
    let signing_key = SigningKey::from_bytes(&[11u8; 32]);
    let verifying_key = VerifyingKey::from(&signing_key);
    let payload = b"inspection:temperature=42";

    let record = build_signed_record(
        "lift-01",
        1,
        1_710_000_000_001,
        payload,
        AuditRecord::zero_hash(),
        "s3://bucket/lift-01/1.bin",
        &signing_key,
    );

    assert_eq!(record.payload_hash, compute_payload_hash(payload));
    assert!(verify_audit_record(&verifying_key, &record));
}

#[test]
fn signature_rejects_substituted_context_with_same_payload_hash() {
    let signing_key = SigningKey::from_bytes(&[11u8; 32]);
    let verifying_key = VerifyingKey::from(&signing_key);
    let payload = b"same-payload";
    let record = build_signed_record(
        "lift-01",
        1,
        1_710_000_000_001,
        payload,
        AuditRecord::zero_hash(),
        "s3://bucket/lift-01/1.bin",
        &signing_key,
    );

    let mut rebound = record.clone();
    rebound.sequence = 2;
    rebound.timestamp_ms = 1;
    rebound.object_ref = "s3://bucket/lift-01/2.bin".to_string();
    assert!(!verify_audit_record(&verifying_key, &rebound));

    let mut flipped = record.clone();
    flipped.payload_hash[0] ^= 0x01;
    assert!(!verify_audit_record(&verifying_key, &flipped));

    let mut other_device = record;
    other_device.device_id = "lift-02".to_string();
    assert!(!verify_audit_record(&verifying_key, &other_device));
}
