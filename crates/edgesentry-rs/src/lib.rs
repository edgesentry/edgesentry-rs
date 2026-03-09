mod agent;
pub mod identity;
pub mod integrity;
pub mod ingest;
mod record;

pub use agent::build_signed_record;
pub use identity::{sign_payload_hash, verify_payload_signature};
pub use integrity::{compute_payload_hash, verify_chain, ChainError};
pub use ingest::{
    AuditLedger, InMemoryAuditLedger, InMemoryOperationLog, InMemoryRawDataStore, IngestDecision,
    IngestError, IngestService, IngestServiceError, IngestState, IntegrityPolicyGate,
    OperationLogEntry, OperationLogStore, RawDataStore,
};
#[cfg(feature = "s3")]
pub use ingest::{S3Backend, S3CompatibleRawDataStore, S3ObjectStoreConfig, S3StoreError};
#[cfg(feature = "postgres")]
pub use ingest::{PostgresAuditLedger, PostgresOperationLog, PostgresStoreError};
pub use record::{AuditRecord, Hash32, Signature64};

use std::{fs, path::Path};

use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("invalid hex input: {0}")]
    InvalidHex(String),
    #[error("invalid byte length: expected {expected}, actual {actual}")]
    InvalidLength { expected: usize, actual: usize },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("chain verification failed: {0}")]
    Chain(String),
}

pub fn parse_fixed_hex<const N: usize>(value: &str) -> Result<[u8; N], CliError> {
    let raw = hex::decode(value).map_err(|e| CliError::InvalidHex(e.to_string()))?;
    if raw.len() != N {
        return Err(CliError::InvalidLength {
            expected: N,
            actual: raw.len(),
        });
    }
    let mut out = [0u8; N];
    out.copy_from_slice(&raw);
    Ok(out)
}

pub fn sign_record(
    device_id: String,
    sequence: u64,
    timestamp_ms: u64,
    payload: Vec<u8>,
    prev_hash: Hash32,
    object_ref: String,
    private_key_hex: &str,
) -> Result<AuditRecord, CliError> {
    let key_bytes = parse_fixed_hex::<32>(private_key_hex)?;
    let signing_key = SigningKey::from_bytes(&key_bytes);

    Ok(build_signed_record(
        device_id,
        sequence,
        timestamp_ms,
        &payload,
        prev_hash,
        object_ref,
        &signing_key,
    ))
}

pub fn verify_record(record: &AuditRecord, public_key_hex: &str) -> Result<bool, CliError> {
    let public_key_bytes = parse_fixed_hex::<32>(public_key_hex)?;
    let key = VerifyingKey::from_bytes(&public_key_bytes)
        .map_err(|e| CliError::InvalidHex(e.to_string()))?;
    Ok(verify_payload_signature(
        &key,
        &record.payload_hash,
        &record.signature,
    ))
}

pub fn verify_chain_file(path: &Path) -> Result<(), CliError> {
    let content = fs::read_to_string(path)?;
    let records: Vec<AuditRecord> = serde_json::from_str(&content)?;
    verify_chain(&records).map_err(|e| CliError::Chain(e.to_string()))
}

pub fn verify_chain_records(records: &[AuditRecord]) -> Result<(), CliError> {
    verify_chain(records).map_err(|e| CliError::Chain(e.to_string()))
}

pub fn build_lift_inspection_demo_records_with_payloads(
    device_id: &str,
    private_key_hex: &str,
    start_timestamp_ms: u64,
    object_prefix: &str,
) -> Result<Vec<(AuditRecord, Vec<u8>)>, CliError> {
    let steps = [
        "check=door,status=ok,open_close_cycle=3",
        "check=vibration,status=ok,rms=0.18",
        "check=emergency_brake,status=ok,response_ms=120",
    ];

    let key_bytes = parse_fixed_hex::<32>(private_key_hex)?;
    let signing_key = SigningKey::from_bytes(&key_bytes);

    let mut results = Vec::with_capacity(steps.len());
    let mut prev_hash = AuditRecord::zero_hash();

    for (index, step) in steps.iter().enumerate() {
        let sequence = (index as u64) + 1;
        let timestamp_ms = start_timestamp_ms + (index as u64) * 60_000;
        let payload = format!(
            "scenario=lift-inspection,device={device_id},sequence={sequence},{step}"
        )
        .into_bytes();
        let object_ref = format!("{object_prefix}/inspection-{sequence}.bin");

        let record = build_signed_record(
            device_id.to_string(),
            sequence,
            timestamp_ms,
            &payload,
            prev_hash,
            object_ref,
            &signing_key,
        );

        prev_hash = record.hash();
        results.push((record, payload));
    }

    Ok(results)
}

pub fn build_lift_inspection_demo_records(
    device_id: &str,
    private_key_hex: &str,
    start_timestamp_ms: u64,
    object_prefix: &str,
) -> Result<Vec<AuditRecord>, CliError> {
    let pairs = build_lift_inspection_demo_records_with_payloads(
        device_id,
        private_key_hex,
        start_timestamp_ms,
        object_prefix,
    )?;
    Ok(pairs.into_iter().map(|(r, _)| r).collect())
}

pub fn write_record_json(path: Option<&Path>, record: &AuditRecord) -> Result<(), CliError> {
    let json = serde_json::to_string_pretty(record)?;
    match path {
        Some(file) => {
            fs::write(file, json)?;
            Ok(())
        }
        None => {
            println!("{json}");
            Ok(())
        }
    }
}

pub fn write_records_json(path: &Path, records: &[AuditRecord]) -> Result<(), CliError> {
    let json = serde_json::to_string_pretty(records)?;
    fs::write(path, json)?;
    Ok(())
}

/// An Ed25519 keypair represented as hex strings.
///
/// `private_key_hex` is 32 bytes (64 hex chars) — keep this secret on the device.
/// `public_key_hex`  is 32 bytes (64 hex chars) — register this on the cloud side
/// via `IntegrityPolicyGate::register_device`.
#[derive(Debug, Serialize, Deserialize)]
pub struct KeyPair {
    pub private_key_hex: String,
    pub public_key_hex: String,
}

/// Generate a fresh Ed25519 keypair using the OS CSPRNG.
pub fn generate_keypair() -> KeyPair {
    let signing_key = SigningKey::generate(&mut OsRng);
    KeyPair {
        private_key_hex: hex::encode(signing_key.to_bytes()),
        public_key_hex: hex::encode(signing_key.verifying_key().to_bytes()),
    }
}

/// Derive the public key from an existing private key hex string.
pub fn inspect_key(private_key_hex: &str) -> Result<KeyPair, CliError> {
    let key_bytes = parse_fixed_hex::<32>(private_key_hex)?;
    let signing_key = SigningKey::from_bytes(&key_bytes);
    Ok(KeyPair {
        private_key_hex: hex::encode(signing_key.to_bytes()),
        public_key_hex: hex::encode(signing_key.verifying_key().to_bytes()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_fixed_hex_requires_exact_length() {
        let err = parse_fixed_hex::<32>("abcd").unwrap_err();
        match err {
            CliError::InvalidLength { expected, actual } => {
                assert_eq!(expected, 32);
                assert_eq!(actual, 2);
            }
            _ => panic!("unexpected error variant"),
        }
    }

    #[test]
    fn sign_and_verify_record_roundtrip() {
        let private_key_hex = "0101010101010101010101010101010101010101010101010101010101010101";
        let private_key = parse_fixed_hex::<32>(private_key_hex).expect("valid private key hex");
        let signing_key = SigningKey::from_bytes(&private_key);
        let public_key_hex = hex::encode(signing_key.verifying_key().to_bytes());

        let record = sign_record(
            "lift-01".to_string(),
            1,
            1_700_000_000_000,
            b"temperature=40".to_vec(),
            AuditRecord::zero_hash(),
            "s3://bucket/lift-01/1.bin".to_string(),
            private_key_hex,
        )
        .expect("record should be signed");

        let valid = verify_record(&record, &public_key_hex).expect("verify should run");
        assert!(valid);
    }

    #[test]
    fn build_lift_demo_records_are_chain_valid() {
        let private_key_hex = "0101010101010101010101010101010101010101010101010101010101010101";
        let records = build_lift_inspection_demo_records(
            "lift-01",
            private_key_hex,
            1_700_000_000_000,
            "s3://bucket/lift-01",
        )
        .expect("demo records should be generated");

        assert_eq!(records.len(), 3);
        verify_chain_records(&records).expect("demo chain should be valid");
    }

    #[test]
    fn parse_fixed_hex_rejects_invalid_hex_chars() {
        let invalid = "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"; // 64 chars but not valid hex
        let err = parse_fixed_hex::<32>(invalid).unwrap_err();
        assert!(matches!(err, CliError::InvalidHex(_)), "expected InvalidHex, got: {err:?}");
    }

    #[test]
    fn verify_record_returns_false_for_wrong_public_key() {
        let private_key_hex = "0202020202020202020202020202020202020202020202020202020202020202";
        let wrong_key_hex   = "0303030303030303030303030303030303030303030303030303030303030303";

        let wrong_signing_key = SigningKey::from_bytes(
            &parse_fixed_hex::<32>(wrong_key_hex).unwrap()
        );
        let wrong_public_key_hex = hex::encode(wrong_signing_key.verifying_key().to_bytes());

        let record = sign_record(
            "lift-01".to_string(),
            1,
            1_700_000_000_000,
            b"temperature=40".to_vec(),
            AuditRecord::zero_hash(),
            "s3://bucket/lift-01/1.bin".to_string(),
            private_key_hex,
        )
        .expect("record should be signed");

        let valid = verify_record(&record, &wrong_public_key_hex).expect("verify should run");
        assert!(!valid, "wrong public key must not verify the signature");
    }

    #[test]
    fn generate_keypair_produces_unique_pairs() {
        let kp1 = generate_keypair();
        let kp2 = generate_keypair();
        assert_ne!(kp1.private_key_hex, kp2.private_key_hex, "each call must produce a unique key");
        assert_ne!(kp1.public_key_hex, kp2.public_key_hex);
        assert_eq!(kp1.private_key_hex.len(), 64);
        assert_eq!(kp1.public_key_hex.len(), 64);
    }

    #[test]
    fn inspect_key_roundtrips_with_generate_keypair() {
        let kp = generate_keypair();
        let inspected = inspect_key(&kp.private_key_hex).expect("inspect_key should succeed");
        assert_eq!(kp.private_key_hex, inspected.private_key_hex);
        assert_eq!(kp.public_key_hex, inspected.public_key_hex);
    }

    #[test]
    fn inspect_key_matches_known_public_key() {
        // This is the well-known test vector used throughout the test suite.
        let private_key_hex = "0101010101010101010101010101010101010101010101010101010101010101";
        let kp = inspect_key(private_key_hex).expect("inspect_key should succeed");
        assert_eq!(kp.public_key_hex, "8a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c");
    }

    #[test]
    fn generated_keypair_can_sign_and_verify() {
        let kp = generate_keypair();
        let record = sign_record(
            "lift-gen".to_string(),
            1,
            1_700_000_000_000,
            b"payload".to_vec(),
            AuditRecord::zero_hash(),
            "s3://bucket/lift-gen/1.bin".to_string(),
            &kp.private_key_hex,
        )
        .expect("sign_record should succeed with generated key");
        let valid = verify_record(&record, &kp.public_key_hex).expect("verify should run");
        assert!(valid, "generated keypair must verify its own signature");
    }

    #[test]
    fn tampered_lift_demo_chain_is_detected() {
        let private_key_hex = "0101010101010101010101010101010101010101010101010101010101010101";
        let mut records = build_lift_inspection_demo_records(
            "lift-01",
            private_key_hex,
            1_700_000_000_000,
            "s3://bucket/lift-01",
        )
        .expect("demo records should be generated");

        records[0].payload_hash[0] ^= 0xFF;

        let err = verify_chain_records(&records).expect_err("tampered chain must fail");
        match err {
            CliError::Chain(message) => {
                assert!(message.contains("invalid previous hash"));
            }
            _ => panic!("unexpected error variant"),
        }
    }
}
