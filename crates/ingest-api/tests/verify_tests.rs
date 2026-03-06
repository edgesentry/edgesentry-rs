use device_agent::build_signed_record;
use ed25519_dalek::{SigningKey, VerifyingKey};
use ingest::{IngestError, IngestState};
use ledger_core::AuditRecord;

#[test]
fn accepts_valid_sequential_records() {
    let signing_key = SigningKey::from_bytes(&[21u8; 32]);
    let verifying_key = VerifyingKey::from(&signing_key);

    let mut state = IngestState::default();
    state.register_device("lift-01", verifying_key);

    let r1 = build_signed_record(
        "lift-01",
        1,
        1,
        b"payload-1",
        AuditRecord::zero_hash(),
        "s3://bucket/r1.bin",
        &signing_key,
    );
    let r2 = build_signed_record(
        "lift-01",
        2,
        2,
        b"payload-2",
        r1.hash(),
        "s3://bucket/r2.bin",
        &signing_key,
    );

    assert!(state.verify_and_accept(&r1).is_ok());
    assert!(state.verify_and_accept(&r2).is_ok());
}

#[test]
fn rejects_duplicate_sequence() {
    let signing_key = SigningKey::from_bytes(&[31u8; 32]);
    let verifying_key = VerifyingKey::from(&signing_key);

    let mut state = IngestState::default();
    state.register_device("lift-01", verifying_key);

    let r1 = build_signed_record(
        "lift-01",
        1,
        1,
        b"payload-1",
        AuditRecord::zero_hash(),
        "s3://bucket/r1.bin",
        &signing_key,
    );

    assert!(state.verify_and_accept(&r1).is_ok());

    let duplicate = build_signed_record(
        "lift-01",
        1,
        2,
        b"payload-duplicate",
        AuditRecord::zero_hash(),
        "s3://bucket/r1b.bin",
        &signing_key,
    );

    let err = state.verify_and_accept(&duplicate).unwrap_err();
    assert_eq!(
        err,
        IngestError::Duplicate {
            device_id: "lift-01".to_string(),
            sequence: 1,
        }
    );
}

#[test]
fn rejects_invalid_prev_hash() {
    let signing_key = SigningKey::from_bytes(&[41u8; 32]);
    let verifying_key = VerifyingKey::from(&signing_key);

    let mut state = IngestState::default();
    state.register_device("lift-01", verifying_key);

    let r1 = build_signed_record(
        "lift-01",
        1,
        1,
        b"payload-1",
        AuditRecord::zero_hash(),
        "s3://bucket/r1.bin",
        &signing_key,
    );
    assert!(state.verify_and_accept(&r1).is_ok());

    let wrong_prev = [9u8; 32];
    let r2 = build_signed_record(
        "lift-01",
        2,
        2,
        b"payload-2",
        wrong_prev,
        "s3://bucket/r2.bin",
        &signing_key,
    );

    let err = state.verify_and_accept(&r2).unwrap_err();
    assert_eq!(err, IngestError::InvalidPrevHash("lift-01".to_string()));
}

#[test]
fn rejects_tampered_signature() {
    let signing_key = SigningKey::from_bytes(&[51u8; 32]);
    let verifying_key = VerifyingKey::from(&signing_key);

    let mut state = IngestState::default();
    state.register_device("lift-01", verifying_key);

    let mut r1 = build_signed_record(
        "lift-01",
        1,
        1,
        b"payload-1",
        AuditRecord::zero_hash(),
        "s3://bucket/r1.bin",
        &signing_key,
    );

    r1.signature[0] ^= 0x01;

    let err = state.verify_and_accept(&r1).unwrap_err();
    assert_eq!(err, IngestError::InvalidSignature("lift-01".to_string()));
}
