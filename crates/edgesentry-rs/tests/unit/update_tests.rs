use ed25519_dalek::SigningKey;
use edgesentry_rs::{
    identity::sign_payload_hash,
    integrity::compute_payload_hash,
    update::{SoftwareUpdate, UpdateVerificationLog, UpdateVerifyDecision, UpdateVerifyError, UpdateVerifier},
};

fn make_update(payload: &[u8], signing_key: &SigningKey) -> SoftwareUpdate {
    let payload_hash = compute_payload_hash(payload);
    let signature = sign_payload_hash(signing_key, &payload_hash);
    SoftwareUpdate {
        package_id:   "firmware".to_string(),
        version:      "1.2.3".to_string(),
        payload_hash,
        signature,
    }
}

// ── accepted path ────────────────────────────────────────────────────────────

#[test]
fn valid_update_is_accepted() {
    let signing_key = SigningKey::from_bytes(&[1u8; 32]);
    let payload = b"firmware-v1.2.3-image";
    let update = make_update(payload, &signing_key);

    let mut verifier = UpdateVerifier::new();
    verifier.register_publisher("acme", signing_key.verifying_key());

    let mut log = UpdateVerificationLog::default();
    assert!(verifier.verify(&update, payload, "acme", &mut log).is_ok());
}

#[test]
fn accepted_update_is_recorded_in_log() {
    let signing_key = SigningKey::from_bytes(&[2u8; 32]);
    let payload = b"app-core-v2.0.0";
    let update = make_update(payload, &signing_key);

    let mut verifier = UpdateVerifier::new();
    verifier.register_publisher("publisher-a", signing_key.verifying_key());

    let mut log = UpdateVerificationLog::default();
    verifier.verify(&update, payload, "publisher-a", &mut log).unwrap();

    assert_eq!(log.entries().len(), 1);
    assert_eq!(log.entries()[0].decision, UpdateVerifyDecision::Accepted);
    assert_eq!(log.entries()[0].package_id, "firmware");
    assert_eq!(log.entries()[0].publisher_id, "publisher-a");
}

// ── unknown publisher ────────────────────────────────────────────────────────

#[test]
fn unknown_publisher_is_rejected() {
    let signing_key = SigningKey::from_bytes(&[3u8; 32]);
    let payload = b"some-firmware";
    let update = make_update(payload, &signing_key);

    let verifier = UpdateVerifier::new(); // no publishers registered
    let mut log = UpdateVerificationLog::default();
    let err = verifier.verify(&update, payload, "unknown-publisher", &mut log).unwrap_err();

    assert_eq!(
        err,
        UpdateVerifyError::UnknownPublisher { publisher_id: "unknown-publisher".to_string() }
    );
}

#[test]
fn unknown_publisher_rejection_is_logged() {
    let signing_key = SigningKey::from_bytes(&[4u8; 32]);
    let update = make_update(b"fw", &signing_key);

    let verifier = UpdateVerifier::new();
    let mut log = UpdateVerificationLog::default();
    let _ = verifier.verify(&update, b"fw", "ghost", &mut log);

    assert_eq!(log.entries().len(), 1);
    assert_eq!(log.entries()[0].decision, UpdateVerifyDecision::Rejected);
}

// ── payload hash mismatch ────────────────────────────────────────────────────

#[test]
fn tampered_payload_is_rejected() {
    let signing_key = SigningKey::from_bytes(&[5u8; 32]);
    let original_payload = b"original-firmware";
    let update = make_update(original_payload, &signing_key);

    let mut verifier = UpdateVerifier::new();
    verifier.register_publisher("pub", signing_key.verifying_key());

    let mut log = UpdateVerificationLog::default();
    let err = verifier
        .verify(&update, b"tampered-firmware", "pub", &mut log)
        .unwrap_err();

    assert!(matches!(err, UpdateVerifyError::PayloadHashMismatch { .. }));
}

#[test]
fn tampered_payload_rejection_is_logged() {
    let signing_key = SigningKey::from_bytes(&[6u8; 32]);
    let update = make_update(b"real", &signing_key);

    let mut verifier = UpdateVerifier::new();
    verifier.register_publisher("pub", signing_key.verifying_key());

    let mut log = UpdateVerificationLog::default();
    let _ = verifier.verify(&update, b"fake", "pub", &mut log);

    assert_eq!(log.entries()[0].decision, UpdateVerifyDecision::Rejected);
}

// ── invalid signature ────────────────────────────────────────────────────────

#[test]
fn wrong_publisher_key_is_rejected() {
    let real_signing_key  = SigningKey::from_bytes(&[7u8; 32]);
    let wrong_signing_key = SigningKey::from_bytes(&[8u8; 32]);
    let payload = b"firmware-image";
    let update = make_update(payload, &real_signing_key); // signed by real key

    let mut verifier = UpdateVerifier::new();
    verifier.register_publisher("pub", wrong_signing_key.verifying_key()); // registered wrong key

    let mut log = UpdateVerificationLog::default();
    let err = verifier.verify(&update, payload, "pub", &mut log).unwrap_err();

    assert!(matches!(err, UpdateVerifyError::InvalidSignature { .. }));
}

#[test]
fn corrupted_signature_is_rejected() {
    let signing_key = SigningKey::from_bytes(&[9u8; 32]);
    let payload = b"firmware-image";
    let mut update = make_update(payload, &signing_key);
    update.signature[0] ^= 0xFF; // corrupt the signature

    let mut verifier = UpdateVerifier::new();
    verifier.register_publisher("pub", signing_key.verifying_key());

    let mut log = UpdateVerificationLog::default();
    let err = verifier.verify(&update, payload, "pub", &mut log).unwrap_err();

    assert!(matches!(err, UpdateVerifyError::InvalidSignature { .. }));
}

#[test]
fn invalid_signature_rejection_is_logged() {
    let signing_key = SigningKey::from_bytes(&[10u8; 32]);
    let wrong_key   = SigningKey::from_bytes(&[11u8; 32]);
    let payload = b"firmware";
    let update = make_update(payload, &signing_key);

    let mut verifier = UpdateVerifier::new();
    verifier.register_publisher("pub", wrong_key.verifying_key());

    let mut log = UpdateVerificationLog::default();
    let _ = verifier.verify(&update, payload, "pub", &mut log);

    assert_eq!(log.entries()[0].decision, UpdateVerifyDecision::Rejected);
}

// ── multiple publishers ───────────────────────────────────────────────────────

#[test]
fn multiple_publishers_each_verified_independently() {
    let key_a = SigningKey::from_bytes(&[20u8; 32]);
    let key_b = SigningKey::from_bytes(&[21u8; 32]);
    let payload_a = b"pkg-a-image";
    let payload_b = b"pkg-b-image";

    let update_a = make_update(payload_a, &key_a);
    let update_b = make_update(payload_b, &key_b);

    let mut verifier = UpdateVerifier::new();
    verifier.register_publisher("vendor-a", key_a.verifying_key());
    verifier.register_publisher("vendor-b", key_b.verifying_key());

    let mut log = UpdateVerificationLog::default();
    assert!(verifier.verify(&update_a, payload_a, "vendor-a", &mut log).is_ok());
    assert!(verifier.verify(&update_b, payload_b, "vendor-b", &mut log).is_ok());
    assert_eq!(log.entries().len(), 2);
}

#[test]
fn publisher_a_key_cannot_verify_publisher_b_update() {
    let key_a = SigningKey::from_bytes(&[22u8; 32]);
    let key_b = SigningKey::from_bytes(&[23u8; 32]);
    let payload = b"shared-payload";

    let update_signed_by_b = make_update(payload, &key_b);

    let mut verifier = UpdateVerifier::new();
    verifier.register_publisher("vendor-a", key_a.verifying_key());

    let mut log = UpdateVerificationLog::default();
    // update signed by B, but we ask verifier to check against vendor-a's key
    let err = verifier
        .verify(&update_signed_by_b, payload, "vendor-a", &mut log)
        .unwrap_err();

    assert!(matches!(err, UpdateVerifyError::InvalidSignature { .. }));
}

// ── log accumulates all attempts ─────────────────────────────────────────────

#[test]
fn log_accumulates_accepted_and_rejected_entries() {
    let signing_key = SigningKey::from_bytes(&[30u8; 32]);
    let payload = b"firmware";
    let update = make_update(payload, &signing_key);

    let mut verifier = UpdateVerifier::new();
    verifier.register_publisher("pub", signing_key.verifying_key());

    let mut log = UpdateVerificationLog::default();

    // Accepted
    verifier.verify(&update, payload, "pub", &mut log).unwrap();
    // Rejected (unknown publisher)
    let _ = verifier.verify(&update, payload, "unknown", &mut log);

    assert_eq!(log.entries().len(), 2);
    assert_eq!(log.entries()[0].decision, UpdateVerifyDecision::Accepted);
    assert_eq!(log.entries()[1].decision, UpdateVerifyDecision::Rejected);
}
