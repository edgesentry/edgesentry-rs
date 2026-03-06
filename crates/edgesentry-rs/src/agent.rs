use ed25519_dalek::SigningKey;

use crate::crypto::{compute_payload_hash, sign_payload_hash};
use crate::record::{AuditRecord, Hash32};

pub fn build_signed_record(
    device_id: impl Into<String>,
    sequence: u64,
    timestamp_ms: u64,
    payload: &[u8],
    prev_record_hash: Hash32,
    object_ref: impl Into<String>,
    signing_key: &SigningKey,
) -> AuditRecord {
    let payload_hash = compute_payload_hash(payload);
    let signature = sign_payload_hash(signing_key, &payload_hash);

    AuditRecord {
        device_id: device_id.into(),
        sequence,
        timestamp_ms,
        payload_hash,
        signature,
        prev_record_hash,
        object_ref: object_ref.into(),
    }
}
