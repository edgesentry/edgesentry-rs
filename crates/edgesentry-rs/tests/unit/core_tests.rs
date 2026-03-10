use ed25519_dalek::{SigningKey, VerifyingKey};
use edgesentry_rs::{
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

#[test]
fn chain_verification_accepts_empty_slice() {
    assert!(verify_chain(&[]).is_ok());
}

#[test]
fn chain_verification_accepts_single_valid_record() {
    let record = dummy_record(1, AuditRecord::zero_hash());
    assert!(verify_chain(&[record]).is_ok());
}

#[test]
fn chain_verification_rejects_first_record_with_nonzero_prev_hash() {
    let bad_first = dummy_record(1, [1u8; 32]);
    let result = verify_chain(&[bad_first]);
    assert_eq!(result, Err(ChainError::InvalidPrevHash { index: 0 }));
}

#[test]
fn verify_payload_signature_rejects_wrong_key() {
    let signing_key = SigningKey::from_bytes(&[8u8; 32]);
    let wrong_key = VerifyingKey::from(&SigningKey::from_bytes(&[9u8; 32]));

    let payload_hash = compute_payload_hash(b"lift-data");
    let sig = sign_payload_hash(&signing_key, &payload_hash);

    assert!(!verify_payload_signature(&wrong_key, &payload_hash, &sig));
}
