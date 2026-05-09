/// Generic ZKP infrastructure for edgesentry-rs.
///
/// This crate provides framework-agnostic abstractions for zero-knowledge proof
/// generation and verification. It carries no business logic — callers implement
/// [`ZkProgram`] with their domain-specific guest program (e.g. BCA Green Mark,
/// vessel navigation integrity).
///
/// # Architecture
///
/// ```text
/// edgesentry-zkp  (this crate)
///   ├── ZkProgram trait  — implemented by each application
///   ├── ZkProof          — serialisable proof envelope stored in AuditRecord
///   └── verify()         — lightweight verification, no proving key required
///
/// clarus / green_mark.rs  — implements ZkProgram for BCA Green Mark
/// arktrace / ais_integrity.rs — implements ZkProgram for AIS position proofs
/// ```

pub mod proof;
pub mod program;
pub mod error;

#[cfg(feature = "sp1")]
pub mod sp1;

#[cfg(feature = "sp1-verifier")]
pub mod sp1_verify;

pub use error::ZkError;
pub use proof::{ZkProof, ZkFramework};
pub use program::{ZkProgram, verify};

#[cfg(test)]
mod tests;
