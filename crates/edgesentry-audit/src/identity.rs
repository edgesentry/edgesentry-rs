//! Ed25519 device identity — signing and verification primitives.
//!
//! This module covers the cryptographic identity layer: signing a payload hash
//! with a device private key and verifying the signature with the registered
//! public key.  All key-generation helpers live in the crate root (`lib.rs`).

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

use crate::record::{Hash32, Signature64};

/// Sign a payload hash with the device's Ed25519 signing key.
pub fn sign_payload_hash(signing_key: &SigningKey, payload_hash: &Hash32) -> Signature64 {
    signing_key.sign(payload_hash).to_bytes()
}

/// Verify that `signature` over `payload_hash` was produced by `verifying_key`.
pub fn verify_payload_signature(
    verifying_key: &VerifyingKey,
    payload_hash: &Hash32,
    signature: &Signature64,
) -> bool {
    let parsed = Signature::from_bytes(signature);
    verifying_key.verify(payload_hash, &parsed).is_ok()
}
