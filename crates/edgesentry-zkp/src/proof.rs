use serde::{Deserialize, Serialize};

/// Identifies which ZK proving framework generated this proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZkFramework {
    Sp1,
    RiscZero,
    /// For tests / stubs that do not use a real proving system.
    #[serde(rename = "mock")]
    Mock,
}

impl ZkFramework {
    pub fn as_str(&self) -> &'static str {
        match self {
            ZkFramework::Sp1 => "sp1",
            ZkFramework::RiscZero => "risc0",
            ZkFramework::Mock => "mock",
        }
    }
}

/// A serialisable proof envelope stored in [`edgesentry_audit::AuditRecord`].
///
/// `proof_bytes` and `public_values` are base64-encoded so the envelope
/// round-trips through JSON cleanly.  Callers should treat the contents
/// as opaque and use [`crate::verify`] (or the framework-specific verifier)
/// to check validity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZkProof {
    /// The proving framework that generated this proof.
    pub framework: ZkFramework,

    /// Verification key hash (hex) that identifies the guest program.
    /// Verifiers must check this matches the expected program before trusting
    /// `public_values`.
    pub program_id: String,

    /// Base64-encoded proof bytes.
    pub proof_bytes: String,

    /// Base64-encoded committed public outputs (what the guest program
    /// `commit`-ed — i.e. the values the verifier can read without
    /// knowing the private inputs).
    pub public_values: String,
}

impl ZkProof {
    pub fn decode_proof_bytes(&self) -> Result<Vec<u8>, base64::DecodeError> {
        use base64::{Engine, engine::general_purpose::STANDARD};
        STANDARD.decode(&self.proof_bytes)
    }

    pub fn decode_public_values(&self) -> Result<Vec<u8>, base64::DecodeError> {
        use base64::{Engine, engine::general_purpose::STANDARD};
        STANDARD.decode(&self.public_values)
    }

    /// Convenience: encode raw bytes to a base64 string for constructing proofs.
    pub fn encode(bytes: &[u8]) -> String {
        use base64::{Engine, engine::general_purpose::STANDARD};
        STANDARD.encode(bytes)
    }
}
