use device_agent::build_signed_record;
use ed25519_dalek::{SigningKey, VerifyingKey};
use ledger_core::{compute_payload_hash, verify_payload_signature, AuditRecord};

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
    assert!(verify_payload_signature(
        &verifying_key,
        &record.payload_hash,
        &record.signature
    ));
}
