use crate::{ZkError, ZkProof};

/// Implemented by each application-specific ZK guest program.
///
/// # Example (BCA Green Mark — lives in `clarus`, not here)
///
/// ```ignore
/// struct GreenMarkProgram;
///
/// impl ZkProgram for GreenMarkProgram {
///     fn program_id(&self) -> &str { GREEN_MARK_VKEY_HASH }
///
///     fn prove(&self, private_inputs: &[u8]) -> Result<ZkProof, ZkError> {
///         // call SP1 prover with green_mark ELF + inputs
///     }
/// }
/// ```
pub trait ZkProgram: Send + Sync {
    /// Stable identifier for this guest program (e.g. SP1 vkey hash, hex).
    /// Verifiers must check this before trusting `public_values`.
    fn program_id(&self) -> &str;

    /// Generate a proof that the guest computation ran correctly on
    /// `private_inputs`.  The proof envelope contains `public_values`
    /// (the values the guest `commit`-ed) and is safe to store in the
    /// WORM audit chain.
    fn prove(&self, private_inputs: &[u8]) -> Result<ZkProof, ZkError>;
}

/// Verify a [`ZkProof`] without access to the private inputs.
///
/// Returns `Ok(true)` if the proof is valid for the claimed `public_values`
/// and `program_id`.  Returns `Ok(false)` or `Err` if verification fails.
///
/// # Feature flags
///
/// - With `sp1-verifier` feature: uses the SP1 Groth16 verifier.
/// - Without any verifier feature: returns `Err(ZkError::UnsupportedFramework)`.
pub fn verify(proof: &ZkProof) -> Result<bool, ZkError> {
    match &proof.framework {
        #[cfg(feature = "sp1-verifier")]
        crate::ZkFramework::Sp1 => crate::sp1_verify::verify(proof),

        crate::ZkFramework::Mock => {
            // Mock proofs are always valid (test / demo only).
            Ok(proof.proof_bytes == ZkProof::encode(b"mock-proof"))
        }

        other => Err(ZkError::UnsupportedFramework(
            other.as_str().to_string(),
        )),
    }
}
