use ed25519_dalek::VerifyingKey;

use crate::record::AuditRecord;
use super::verify::{IngestError, IngestState};

/// Enforces signature, sequence, and route identity integrity before persistence.
///
/// This is the P0 policy gate: every record must pass all checks here
/// before it is allowed through to storage.
#[derive(Default)]
pub struct IntegrityPolicyGate {
    state: IngestState,
}

impl IntegrityPolicyGate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_device(&mut self, device_id: impl Into<String>, key: VerifyingKey) {
        self.state.register_device(device_id, key);
    }

    /// Run all integrity checks for `record`.
    ///
    /// Checks (in order):
    /// 1. Route identity — `cert_identity` must match `record.device_id` when present.
    /// 2. Signature — payload hash must be signed by the registered device key.
    /// 3. Sequence — must be strictly monotonic and non-duplicate.
    /// 4. Previous-record hash — must match the last accepted record's hash.
    pub fn enforce(
        &mut self,
        record: &AuditRecord,
        cert_identity: Option<&str>,
    ) -> Result<(), IngestError> {
        if let Some(identity) = cert_identity {
            if identity != record.device_id {
                return Err(IngestError::CertDeviceMismatch {
                    cert_identity: identity.to_string(),
                    device_id: record.device_id.clone(),
                });
            }
        }

        self.state.verify_and_accept(record)
    }
}

