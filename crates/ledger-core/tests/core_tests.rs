use ed25519_dalek::{SigningKey, VerifyingKey};
use ledger_core::{
    compute_payload_hash, sign_payload_hash, verify_chain, verify_payload_signature, AuditRecord,
    ChainError,
};

fn dummy_record(sequence: u64, prev_record_hash: [u8; 32]) -> AuditRecord {
    AuditRecord {
        device_id: "lift-01".to_string(),
        sequence,
        timestamp_ms: 1_710_000_000_000 + sequence,
        payload_hash: [sequence as u8; 32],
        signature: [9u8; 64],
        prev_record_hash,
        object_ref: format!("s3://bucket/lift-01/{sequence}.bin"),
    }
}

#[test]
fn payload_hash_changes_on_input_change() {
    let h1 = compute_payload_hash(b"door-open");
    let h2 = compute_payload_hash(b"door-close");

    assert_ne!(h1, h2);
}

#[test]
fn sign_and_verify_payload_hash() {
    let signing_key = SigningKey::from_bytes(&[7u8; 32]);
    let verifying_key = VerifyingKey::from(&signing_key);

    let payload_hash = compute_payload_hash(b"lift-vibration-data");
    let sig = sign_payload_hash(&signing_key, &payload_hash);

    assert!(verify_payload_signature(&verifying_key, &payload_hash, &sig));

    let mut tampered_hash = payload_hash;
    tampered_hash[0] ^= 0x01;
    assert!(!verify_payload_signature(
        &verifying_key,
        &tampered_hash,
        &sig
    ));
}

#[test]
fn chain_verification_detects_broken_link() {
    let mut first = dummy_record(1, AuditRecord::zero_hash());
    let first_hash = first.hash();
    let second = dummy_record(2, first_hash);

    assert!(verify_chain(&[first.clone(), second]).is_ok());

    first.payload_hash[0] ^= 0xFF;
    let second_after_tamper = dummy_record(2, first_hash);

    let result = verify_chain(&[first, second_after_tamper]);
    assert_eq!(result, Err(ChainError::InvalidPrevHash { index: 1 }));
}

#[test]
fn chain_verification_detects_invalid_sequence() {
    let first = dummy_record(1, AuditRecord::zero_hash());
    let second = dummy_record(3, first.hash());

    let result = verify_chain(&[first, second]);
    assert_eq!(
        result,
        Err(ChainError::InvalidSequence {
            index: 1,
            expected: 2,
            actual: 3,
        })
    );
}
