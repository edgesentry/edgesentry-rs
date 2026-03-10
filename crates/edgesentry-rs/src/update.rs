//! Software update integrity verification (CLS-03 / STAR-2 R2.2).
//!
//! Before applying any firmware or software update on a device, the update
//! package must be authenticated:
//!
//! 1. The raw payload is hashed with BLAKE3 and compared to [`SoftwareUpdate::payload_hash`].
//! 2. The publisher's Ed25519 signature over `payload_hash` is verified against a
//!    registered trusted key.
//!
//! A failed check returns [`UpdateVerifyError`] and is recorded in
//! [`UpdateVerificationLog`] so the rejection appears in the audit trail.
//!
//! # Example
//!
//! ```rust
//! use ed25519_dalek::SigningKey;
//! use edgesentry_rs::update::{SoftwareUpdate, UpdateVerifier};
//! use edgesentry_rs::integrity::compute_payload_hash;
//! use edgesentry_rs::identity::sign_payload_hash;
//!
//! let signing_key = SigningKey::from_bytes(&[7u8; 32]);
//! let verifying_key = signing_key.verifying_key();
//! let payload = b"firmware-v1.2.3-image";
//!
//! let payload_hash = compute_payload_hash(payload);
//! let signature   = sign_payload_hash(&signing_key, &payload_hash);
//!
//! let update = SoftwareUpdate {
//!     package_id:   "firmware".to_string(),
//!     version:      "1.2.3".to_string(),
//!     payload_hash,
//!     signature,
//! };
//!
//! let mut verifier = UpdateVerifier::new();
//! verifier.register_publisher("acme-firmware", verifying_key);
//!
//! let mut log = edgesentry_rs::update::UpdateVerificationLog::default();
//! assert!(verifier.verify(&update, payload, "acme-firmware", &mut log).is_ok());
//! ```

use std::collections::HashMap;

use ed25519_dalek::VerifyingKey;
use thiserror::Error;

use crate::identity::verify_payload_signature;
use crate::integrity::compute_payload_hash;
use crate::record::{Hash32, Signature64};

// ── Types ────────────────────────────────────────────────────────────────────

/// A signed software update package ready for pre-installation verification.
#[derive(Debug, Clone)]
pub struct SoftwareUpdate {
    /// Unique identifier for the update package (e.g. `"firmware"`, `"app-core"`).
    pub package_id: String,
    /// Human-readable version string (e.g. `"1.2.3"`).
    pub version: String,
    /// BLAKE3 hash of the raw update payload.
    pub payload_hash: Hash32,
    /// Ed25519 signature over `payload_hash` produced by the trusted publisher.
    pub signature: Signature64,
}

/// Outcome recorded in [`UpdateVerificationLog`] for every verification attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateVerifyDecision {
    Accepted,
    Rejected,
}

/// A single entry in the update verification audit trail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateVerificationEntry {
    pub decision:     UpdateVerifyDecision,
    pub package_id:   String,
    pub version:      String,
    pub publisher_id: String,
    pub message:      String,
}

/// In-memory log of all update verification attempts.
#[derive(Debug, Default)]
pub struct UpdateVerificationLog {
    entries: Vec<UpdateVerificationEntry>,
}

impl UpdateVerificationLog {
    pub fn entries(&self) -> &[UpdateVerificationEntry] {
        &self.entries
    }

    fn record(&mut self, entry: UpdateVerificationEntry) {
        self.entries.push(entry);
    }
}

// ── Errors ───────────────────────────────────────────────────────────────────

/// Errors produced by [`UpdateVerifier::verify`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum UpdateVerifyError {
    /// No key has been registered for the given publisher.
    #[error("unknown publisher '{publisher_id}'")]
    UnknownPublisher { publisher_id: String },

    /// The BLAKE3 hash of the supplied payload does not match the update manifest.
    #[error("payload hash mismatch for package '{package_id}' version '{version}'")]
    PayloadHashMismatch { package_id: String, version: String },

    /// The publisher signature is invalid or was produced by a different key.
    #[error("invalid publisher signature for package '{package_id}' version '{version}'")]
    InvalidSignature { package_id: String, version: String },
}

// ── Verifier ─────────────────────────────────────────────────────────────────

/// Verifies software update packages before installation.
///
/// Register one or more trusted publisher keys with [`register_publisher`](Self::register_publisher),
/// then call [`verify`](Self::verify) for each candidate update. Failed
/// verifications are automatically recorded in a supplied [`UpdateVerificationLog`].
#[derive(Debug, Default)]
pub struct UpdateVerifier {
    trusted_keys: HashMap<String, VerifyingKey>,
}

impl UpdateVerifier {
    /// Create a verifier with no trusted publishers.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a trusted publisher key.
    ///
    /// Only updates signed by a registered publisher will pass verification.
    pub fn register_publisher(&mut self, publisher_id: &str, key: VerifyingKey) {
        self.trusted_keys.insert(publisher_id.to_string(), key);
    }

    /// Verify `update` against `payload` and record the outcome in `log`.
    ///
    /// Returns `Ok(())` only when:
    /// - `publisher_id` is registered,
    /// - `BLAKE3(payload) == update.payload_hash`, and
    /// - the Ed25519 signature is valid.
    ///
    /// Any failure appends a [`UpdateVerifyDecision::Rejected`] entry to `log`
    /// and returns the corresponding [`UpdateVerifyError`].
    pub fn verify(
        &self,
        update: &SoftwareUpdate,
        payload: &[u8],
        publisher_id: &str,
        log: &mut UpdateVerificationLog,
    ) -> Result<(), UpdateVerifyError> {
        let result = self.check(update, payload, publisher_id);

        let (decision, message) = match &result {
            Ok(()) => (
                UpdateVerifyDecision::Accepted,
                format!(
                    "update accepted: package={} version={}",
                    update.package_id, update.version
                ),
            ),
            Err(e) => (UpdateVerifyDecision::Rejected, e.to_string()),
        };

        log.record(UpdateVerificationEntry {
            decision,
            package_id:   update.package_id.clone(),
            version:      update.version.clone(),
            publisher_id: publisher_id.to_string(),
            message,
        });

        result
    }

    fn check(
        &self,
        update: &SoftwareUpdate,
        payload: &[u8],
        publisher_id: &str,
    ) -> Result<(), UpdateVerifyError> {
        let key = self.trusted_keys.get(publisher_id).ok_or_else(|| {
            UpdateVerifyError::UnknownPublisher {
                publisher_id: publisher_id.to_string(),
            }
        })?;

        let actual_hash = compute_payload_hash(payload);
        if actual_hash != update.payload_hash {
            return Err(UpdateVerifyError::PayloadHashMismatch {
                package_id: update.package_id.clone(),
                version:    update.version.clone(),
            });
        }

        if !verify_payload_signature(key, &update.payload_hash, &update.signature) {
            return Err(UpdateVerifyError::InvalidSignature {
                package_id: update.package_id.clone(),
                version:    update.version.clone(),
            });
        }

        Ok(())
    }
}
