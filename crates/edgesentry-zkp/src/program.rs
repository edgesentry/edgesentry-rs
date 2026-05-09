use crate::{ZkError, ZkFramework, ZkProof};

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
///         // sp1-sdk is declared as a dependency of *clarus*, not edgesentry-zkp
///     }
/// }
/// ```
pub trait ZkProgram: Send + Sync {
    /// Stable identifier for this guest program (e.g. SP1 vkey hash, hex).
    /// Verifiers must check this matches the expected program before trusting
    /// `public_values`.
    fn program_id(&self) -> &str;

    /// Generate a proof that the guest computation ran correctly on
    /// `private_inputs`.  The proof envelope contains `public_values`
    /// (the values the guest committed) and is safe to store in the WORM
    /// audit chain.
    fn prove(&self, private_inputs: &[u8]) -> Result<ZkProof, ZkError>;
}

/// Verify a [`ZkProof`] without access to the private inputs.
///
/// Returns `Ok(true)` if the proof is valid, `Ok(false)` if it is invalid,
/// and `Err` if the framework is not supported in this build.
///
/// # Framework support
///
/// - `Mock`: always supported; used in tests and demos.
/// - `Sp1` / `RiscZero`: verification is delegated to the implementing crate
///   (e.g. clarus) which carries the proving SDK as a dependency.  Calling
///   this function with an SP1 proof returns `Err(UnsupportedFramework)`;
///   call the SP1 verifier directly in the implementing crate instead.
pub fn verify(proof: &ZkProof) -> Result<bool, ZkError> {
    match &proof.framework {
        ZkFramework::Mock => {
            Ok(proof.proof_bytes == ZkProof::encode(b"mock-proof"))
        }
        other => Err(ZkError::UnsupportedFramework(
            other.as_str().to_string(),
        )),
    }
}
