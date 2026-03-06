use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

use crate::{Hash32, Signature64};

pub fn compute_payload_hash(payload: &[u8]) -> Hash32 {
    *blake3::hash(payload).as_bytes()
}

pub fn sign_payload_hash(signing_key: &SigningKey, payload_hash: &Hash32) -> Signature64 {
    signing_key.sign(payload_hash).to_bytes()
}

pub fn verify_payload_signature(
    verifying_key: &VerifyingKey,
    payload_hash: &Hash32,
    signature: &Signature64,
) -> bool {
    let parsed = Signature::from_bytes(signature);
    verifying_key.verify(payload_hash, &parsed).is_ok()
}
