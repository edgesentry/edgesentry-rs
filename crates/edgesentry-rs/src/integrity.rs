//! BLAKE3 hash-chain integrity — payload hashing and chain verification.
//!
//! This module covers the tamper-detection layer: hashing a raw payload with
//! BLAKE3 and verifying that a sequence of `AuditRecord`s forms an unbroken
//! hash chain.

use thiserror::Error;

use crate::record::AuditRecord;

/// Compute the BLAKE3 hash of a raw payload.
pub fn compute_payload_hash(payload: &[u8]) -> [u8; 32] {
    *blake3::hash(payload).as_bytes()
}

/// Errors produced by [`verify_chain`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ChainError {
    #[error("invalid previous hash at index {index}")]
    InvalidPrevHash { index: usize },
    #[error("invalid sequence at index {index}: expected {expected}, actual {actual}")]
    InvalidSequence {
        index: usize,
        expected: u64,
        actual: u64,
    },
}

/// Verify that `records` form a valid hash chain.
///
/// - The first record must have `prev_record_hash == [0u8; 32]`.
/// - Each subsequent record's `prev_record_hash` must equal the hash of the
///   preceding record.
/// - Sequences must be strictly monotonically increasing by 1.
pub fn verify_chain(records: &[AuditRecord]) -> Result<(), ChainError> {
    if records.is_empty() {
        return Ok(());
    }

    for (index, record) in records.iter().enumerate() {
        if index == 0 {
            if record.prev_record_hash != AuditRecord::zero_hash() {
                return Err(ChainError::InvalidPrevHash { index });
            }
            continue;
        }

        let previous = &records[index - 1];
        if record.prev_record_hash != previous.hash() {
            return Err(ChainError::InvalidPrevHash { index });
        }

        let expected_sequence = previous.sequence + 1;
        if record.sequence != expected_sequence {
            return Err(ChainError::InvalidSequence {
                index,
                expected: expected_sequence,
                actual: record.sequence,
            });
        }
    }

    Ok(())
}
