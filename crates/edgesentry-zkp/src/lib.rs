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
///   ├── ZkProgram trait  — implemented by each application crate
///   ├── ZkProof          — serialisable proof envelope stored in AuditRecord
///   └── verify()         — lightweight verification (mock only here;
///                          real SP1/Risc Zero verifiers live in the implementing crate)
///
/// clarus / green_mark    — implements ZkProgram for BCA Green Mark via SP1
/// arktrace / ais_proof   — implements ZkProgram for AIS position integrity
/// ```
///
/// # Why no SP1 dependency here?
///
/// `sp1-sdk` and `risc0-zkvm` are heavyweight build-time dependencies that
/// require a specific toolchain and target architecture.  Adding them here
/// would force every consumer of `edgesentry-zkp` (including embedded targets)
/// to build them.  Instead, the proving SDK is declared only in the crate that
/// implements [`ZkProgram`] for a specific domain.

pub mod proof;
pub mod program;
pub mod error;

pub use error::ZkError;
pub use proof::{ZkProof, ZkFramework};
pub use program::{ZkProgram, verify};

#[cfg(test)]
mod tests;
