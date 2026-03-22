use ed25519_dalek::{SigningKey, VerifyingKey};
use edgesentry_rs::{
    build_lift_inspection_demo_records, compute_payload_hash, sign_payload_hash, verify_chain,
    verify_chain_file, verify_payload_signature, write_record_json, write_records_json, AuditRecord,
    ChainError,
};
use std::{fs, path::PathBuf};

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

// Helper: unique temp path that is cleaned up on drop via a guard
fn tmp_path(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("edgesentry_test_{}_{name}", std::process::id()));
    p
}

struct TmpFile(PathBuf);
impl Drop for TmpFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[test]
fn write_records_json_roundtrips_through_verify_chain_file() {
    // Build a small valid chain via the public demo helper
    let keypair = edgesentry_rs::generate_keypair();
    let records =
        build_lift_inspection_demo_records("lift-01", &keypair.private_key_hex, 1_710_000_000_000, "s3://b")
            .expect("demo records");

    let path = tmp_path("write_records_json.json");
    let _guard = TmpFile(path.clone());

    // write_records_json serialises the slice to a JSON file
    write_records_json(&path, &records).expect("write_records_json");
    assert!(path.exists());

    // verify_chain_file deserialises and verifies the chain
    verify_chain_file(&path).expect("verify_chain_file");
}

#[test]
fn verify_chain_file_rejects_tampered_json() {
    let keypair = edgesentry_rs::generate_keypair();
    let mut records =
        build_lift_inspection_demo_records("lift-01", &keypair.private_key_hex, 1_710_000_000_000, "s3://b")
            .expect("demo records");

    // Tamper with the second record's payload hash to break the chain link
    records[1].payload_hash[0] ^= 0xFF;

    let path = tmp_path("verify_chain_file_tampered.json");
    let _guard = TmpFile(path.clone());

    write_records_json(&path, &records).expect("write");
    let result = verify_chain_file(&path);
    assert!(result.is_err(), "tampered chain should be rejected");
}

#[test]
fn verify_chain_file_rejects_malformed_json() {
    let path = tmp_path("verify_chain_file_malformed.json");
    let _guard = TmpFile(path.clone());

    fs::write(&path, b"not valid json at all").expect("write");
    let result = verify_chain_file(&path);
    assert!(result.is_err(), "malformed JSON should be rejected");
}

#[test]
fn write_record_json_to_file_roundtrips() {
    let keypair = edgesentry_rs::generate_keypair();
    let records =
        build_lift_inspection_demo_records("lift-01", &keypair.private_key_hex, 1_710_000_000_000, "s3://b")
            .expect("demo records");
    let record = &records[0];

    let path = tmp_path("write_record_json.json");
    let _guard = TmpFile(path.clone());

    write_record_json(Some(&path), record).expect("write_record_json");

    let content = fs::read_to_string(&path).expect("read back");
    let decoded: AuditRecord = serde_json::from_str(&content).expect("deserialise");
    assert_eq!(decoded.device_id, record.device_id);
    assert_eq!(decoded.sequence, record.sequence);
    assert_eq!(decoded.payload_hash, record.payload_hash);
}

#[test]
fn write_record_json_stdout_path_is_none() {
    // Passing None should print to stdout without error (no file created)
    let keypair = edgesentry_rs::generate_keypair();
    let records =
        build_lift_inspection_demo_records("lift-01", &keypair.private_key_hex, 1_710_000_000_000, "s3://b")
            .expect("demo records");
    // Should not panic or error
    write_record_json(None, &records[0]).expect("write_record_json stdout");
}
