mod chain;
mod crypto;
mod record;

pub use chain::{verify_chain, ChainError};
pub use crypto::{compute_payload_hash, sign_payload_hash, verify_payload_signature};
pub use record::{AuditRecord, Hash32, Signature64};
