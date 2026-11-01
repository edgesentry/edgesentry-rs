//! Ed25519 device identity — signing and verification primitives.
//!
//! Audit records are signed over a length-prefixed encoding of `device_id`,
//! `sequence`, `timestamp_ms`, `prev_record_hash`, `object_ref`, and
//! `payload_hash`. Software updates are signed over `package_id`, `version`,
//! and `payload_hash`. [`sign_payload_hash`] covers the payload hash alone and
//! does not authorize either of those messages.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

use crate::record::{AuditRecord, Hash32, Signature64};

const AUDIT_SIG_DOMAIN: &[u8] = b"eds.audit.v1\0";
const UPDATE_SIG_DOMAIN: &[u8] = b"eds.update.v1\0";

/// Audit-record fields covered by the device signature, in canonical order.
pub struct AuditSigContext<'a> {
    pub device_id: &'a str,
    pub sequence: u64,
    pub timestamp_ms: u64,
    pub prev_record_hash: &'a Hash32,
    pub object_ref: &'a str,
    pub payload_hash: &'a Hash32,
}

impl<'a> AuditSigContext<'a> {
    pub fn from_record(record: &'a AuditRecord) -> Self {
        Self {
            device_id: &record.device_id,
            sequence: record.sequence,
            timestamp_ms: record.timestamp_ms,
            prev_record_hash: &record.prev_record_hash,
            object_ref: &record.object_ref,
            payload_hash: &record.payload_hash,
        }
    }
}

fn append_len_prefixed(out: &mut Vec<u8>, bytes: &[u8]) {
    let len = bytes.len() as u64;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(bytes);
}

/// Canonical bytes signed for an audit record.
pub fn canonical_audit_message(ctx: &AuditSigContext<'_>) -> Vec<u8> {
    let mut out = Vec::with_capacity(
        AUDIT_SIG_DOMAIN.len() + 8 + ctx.device_id.len() + 16 + 32 + 8 + ctx.object_ref.len() + 32,
    );
    out.extend_from_slice(AUDIT_SIG_DOMAIN);
    append_len_prefixed(&mut out, ctx.device_id.as_bytes());
    out.extend_from_slice(&ctx.sequence.to_le_bytes());
    out.extend_from_slice(&ctx.timestamp_ms.to_le_bytes());
    out.extend_from_slice(ctx.prev_record_hash);
    append_len_prefixed(&mut out, ctx.object_ref.as_bytes());
    out.extend_from_slice(ctx.payload_hash);
    out
}

/// Sign the canonical audit context with the device private key.
pub fn sign_audit_context(signing_key: &SigningKey, ctx: &AuditSigContext<'_>) -> Signature64 {
    signing_key.sign(&canonical_audit_message(ctx)).to_bytes()
}

/// Verify an audit-context signature.
pub fn verify_audit_context(
    verifying_key: &VerifyingKey,
    ctx: &AuditSigContext<'_>,
    signature: &Signature64,
) -> bool {
    let message = canonical_audit_message(ctx);
    let parsed = Signature::from_bytes(signature);
    verifying_key.verify(&message, &parsed).is_ok()
}

/// Verify the signature on `record` against the canonical audit context.
pub fn verify_audit_record(verifying_key: &VerifyingKey, record: &AuditRecord) -> bool {
    verify_audit_context(
        verifying_key,
        &AuditSigContext::from_record(record),
        &record.signature,
    )
}

/// Canonical bytes signed for a software update (`package_id`, `version`, `payload_hash`).
pub fn canonical_update_message(package_id: &str, version: &str, payload_hash: &Hash32) -> Vec<u8> {
    let mut out =
        Vec::with_capacity(UPDATE_SIG_DOMAIN.len() + 16 + package_id.len() + version.len() + 32);
    out.extend_from_slice(UPDATE_SIG_DOMAIN);
    append_len_prefixed(&mut out, package_id.as_bytes());
    append_len_prefixed(&mut out, version.as_bytes());
    out.extend_from_slice(payload_hash);
    out
}

/// Sign `package_id`, `version`, and `payload_hash` with the publisher key.
pub fn sign_update(
    signing_key: &SigningKey,
    package_id: &str,
    version: &str,
    payload_hash: &Hash32,
) -> Signature64 {
    signing_key
        .sign(&canonical_update_message(package_id, version, payload_hash))
        .to_bytes()
}

/// Verify a publisher signature over `package_id`, `version`, and `payload_hash`.
pub fn verify_update_signature(
    verifying_key: &VerifyingKey,
    package_id: &str,
    version: &str,
    payload_hash: &Hash32,
    signature: &Signature64,
) -> bool {
    let message = canonical_update_message(package_id, version, payload_hash);
    let parsed = Signature::from_bytes(signature);
    verifying_key.verify(&message, &parsed).is_ok()
}

/// Sign a payload hash alone.
///
/// This does not authorize an audit record or a software update. Use
/// [`sign_audit_context`] or [`sign_update`].
pub fn sign_payload_hash(signing_key: &SigningKey, payload_hash: &Hash32) -> Signature64 {
    signing_key.sign(payload_hash).to_bytes()
}

/// Verify that `signature` over `payload_hash` alone was produced by `verifying_key`.
///
/// This does not authorize an audit record or a software update. Use
/// [`verify_audit_record`] or [`verify_update_signature`].
pub fn verify_payload_signature(
    verifying_key: &VerifyingKey,
    payload_hash: &Hash32,
    signature: &Signature64,
) -> bool {
    let parsed = Signature::from_bytes(signature);
    verifying_key.verify(payload_hash, &parsed).is_ok()
}
