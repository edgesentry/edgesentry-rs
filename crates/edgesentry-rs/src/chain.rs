use thiserror::Error;

use crate::record::AuditRecord;

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
