use ed25519_dalek::{SigningKey, VerifyingKey};
use edgesentry_rs::{build_signed_record, AuditRecord, IngestError, IngestState};

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
fn rejects_out_of_order_sequence() {
    let signing_key = SigningKey::from_bytes(&[43u8; 32]);
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

    // Skip sequence 2 and jump to sequence 3
    let r3 = build_signed_record(
        "lift-01",
        3,
        3,
        b"payload-3",
        r1.hash(),
        "s3://bucket/r3.bin",
        &signing_key,
    );

    let err = state.verify_and_accept(&r3).unwrap_err();
    assert_eq!(
        err,
        IngestError::InvalidSequence {
            device_id: "lift-01".to_string(),
            expected: 2,
            actual: 3,
        }
    );
}

#[test]
fn rejects_unknown_device() {
    let signing_key = SigningKey::from_bytes(&[55u8; 32]);

    let mut state = IngestState::default();
    // intentionally do NOT register any device

    let r1 = build_signed_record(
        "lift-99",
        1,
        1,
        b"payload-1",
        AuditRecord::zero_hash(),
        "s3://bucket/r1.bin",
        &signing_key,
    );

    let err = state.verify_and_accept(&r1).unwrap_err();
    assert_eq!(err, IngestError::UnknownDevice("lift-99".to_string()));
}

#[test]
fn two_devices_are_isolated() {
    let sk_a = SigningKey::from_bytes(&[57u8; 32]);
    let sk_b = SigningKey::from_bytes(&[59u8; 32]);
    let vk_a = VerifyingKey::from(&sk_a);
    let vk_b = VerifyingKey::from(&sk_b);

    let mut state = IngestState::default();
    state.register_device("lift-01", vk_a);
    state.register_device("lift-02", vk_b);

    let a1 = build_signed_record(
        "lift-01",
        1,
        1,
        b"payload-a1",
        AuditRecord::zero_hash(),
        "s3://bucket/a1.bin",
        &sk_a,
    );
    let b1 = build_signed_record(
        "lift-02",
        1,
        1,
        b"payload-b1",
        AuditRecord::zero_hash(),
        "s3://bucket/b1.bin",
        &sk_b,
    );

    assert!(state.verify_and_accept(&a1).is_ok());
    assert!(state.verify_and_accept(&b1).is_ok());

    // Each device's sequence advances independently
    let a2 = build_signed_record(
        "lift-01",
        2,
        2,
        b"payload-a2",
        a1.hash(),
        "s3://bucket/a2.bin",
        &sk_a,
    );
    let b2 = build_signed_record(
        "lift-02",
        2,
        2,
        b"payload-b2",
        b1.hash(),
        "s3://bucket/b2.bin",
        &sk_b,
    );

    assert!(state.verify_and_accept(&a2).is_ok());
    assert!(state.verify_and_accept(&b2).is_ok());
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
