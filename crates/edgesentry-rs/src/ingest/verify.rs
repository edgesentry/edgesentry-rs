use std::collections::{HashMap, HashSet};

use ed25519_dalek::VerifyingKey;
use thiserror::Error;

use crate::crypto::verify_payload_signature;
use crate::record::{AuditRecord, Hash32};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum IngestError {
    #[error("unknown device: {0}")]
    UnknownDevice(String),
    #[error("duplicate record for device={device_id} sequence={sequence}")]
    Duplicate { device_id: String, sequence: u64 },
    #[error("invalid sequence for device={device_id}: expected={expected} actual={actual}")]
    InvalidSequence {
        device_id: String,
        expected: u64,
        actual: u64,
    },
    #[error("invalid previous hash for device={0}")]
    InvalidPrevHash(String),
    #[error("invalid signature for device={0}")]
    InvalidSignature(String),
    #[error("auth/device mismatch: cert_identity={cert_identity} device_id={device_id}")]
    CertDeviceMismatch {
        cert_identity: String,
        device_id: String,
    },
}

#[derive(Default)]
pub struct IngestState {
    public_keys: HashMap<String, VerifyingKey>,
    seen: HashSet<(String, u64)>,
    last_sequence: HashMap<String, u64>,
    last_hash: HashMap<String, Hash32>,
}

impl IngestState {
    pub fn register_device(&mut self, device_id: impl Into<String>, key: VerifyingKey) {
        self.public_keys.insert(device_id.into(), key);
    }

    pub fn verify_and_accept(&mut self, record: &AuditRecord) -> Result<(), IngestError> {
        let device_id = &record.device_id;
        let key = self
            .public_keys
            .get(device_id)
            .ok_or_else(|| IngestError::UnknownDevice(device_id.clone()))?;

        if !verify_payload_signature(key, &record.payload_hash, &record.signature) {
            return Err(IngestError::InvalidSignature(device_id.clone()));
        }

        if self.seen.contains(&(device_id.clone(), record.sequence)) {
            return Err(IngestError::Duplicate {
                device_id: device_id.clone(),
                sequence: record.sequence,
            });
        }

        let expected_sequence = self
            .last_sequence
            .get(device_id)
            .map_or(1, |prev| prev.saturating_add(1));
        if record.sequence != expected_sequence {
            return Err(IngestError::InvalidSequence {
                device_id: device_id.clone(),
                expected: expected_sequence,
                actual: record.sequence,
            });
        }

        let expected_prev_hash = self
            .last_hash
            .get(device_id)
            .copied()
            .unwrap_or_else(AuditRecord::zero_hash);

        if record.prev_record_hash != expected_prev_hash {
            return Err(IngestError::InvalidPrevHash(device_id.clone()));
        }

        self.seen.insert((device_id.clone(), record.sequence));
        self.last_sequence.insert(device_id.clone(), record.sequence);
        self.last_hash.insert(device_id.clone(), record.hash());

        Ok(())
    }
}
