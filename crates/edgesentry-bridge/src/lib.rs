//! C/C++ FFI bridge for edgesentry-rs.
//!
//! Exposes Ed25519 signing, signature verification, and BLAKE3 hash-chain
//! verification as a stable C ABI so that C/C++ firmware and gateways can
//! use Singapore-grade security without a full Rust rewrite.
//!
//! # Memory safety
//!
//! - Every function that takes a pointer asserts it is non-NULL and returns
//!   `EDS_ERR_NULL_PTR` immediately if the check fails.
//! - `EdsAuditRecord` is always **caller-allocated**; Rust never calls `malloc`
//!   or returns heap-allocated data.  No corresponding `_free` function is
//!   needed.
//! - All string fields in `EdsAuditRecord` (`device_id`, `object_ref`) are
//!   null-terminated byte arrays.  Strings that exceed the buffer capacity
//!   are rejected with `EDS_ERR_STRING_TOO_LONG`.
//! - Raw-pointer arguments for keys and hashes must point to exactly 32 bytes
//!   (keys / hashes) or 64 bytes (signatures) of valid memory.
//! - Rust panics are caught at every FFI boundary and converted to
//!   `EDS_ERR_PANIC`.

use std::ffi::CStr;
use std::os::raw::c_char;

use ed25519_dalek::{SigningKey, VerifyingKey};
use edgesentry_rs::{
    compute_payload_hash, sign_payload_hash, verify_chain, verify_payload_signature, AuditRecord,
};
use rand::rngs::OsRng;

// ── Error codes ──────────────────────────────────────────────────────────────

/// Operation completed successfully.
pub const EDS_OK: i32 = 0;
/// A required pointer argument was NULL.
pub const EDS_ERR_NULL_PTR: i32 = -1;
/// A string argument contains invalid UTF-8.
pub const EDS_ERR_INVALID_UTF8: i32 = -2;
/// A key or hash byte buffer is invalid or has the wrong length.
pub const EDS_ERR_INVALID_KEY: i32 = -3;
/// A string argument is longer than the fixed output buffer allows.
pub const EDS_ERR_STRING_TOO_LONG: i32 = -4;
/// Hash-chain verification failed (tampered, reordered, or truncated chain).
pub const EDS_ERR_CHAIN_INVALID: i32 = -5;
/// An unexpected Rust panic was caught at the FFI boundary.
pub const EDS_ERR_PANIC: i32 = -6;

// ── C-compatible record struct ────────────────────────────────────────────────

/// A tamper-evident audit record with a C-compatible layout.
///
/// All fields are value types — no heap allocation.  `device_id` and
/// `object_ref` are null-terminated strings stored in fixed-size byte arrays.
#[repr(C)]
pub struct EdsAuditRecord {
    /// Monotonically increasing record index for this device.
    pub sequence: u64,
    /// Unix epoch in milliseconds when the record was created.
    pub timestamp_ms: u64,
    /// BLAKE3 hash of the raw payload (32 bytes).
    pub payload_hash: [u8; 32],
    /// Ed25519 signature over `payload_hash` (64 bytes).
    pub signature: [u8; 64],
    /// Hash of the preceding record, or all-zeros for the first record (32 bytes).
    pub prev_record_hash: [u8; 32],
    /// Null-terminated device identifier (max 255 chars + NUL).
    pub device_id: [u8; 256],
    /// Null-terminated storage reference, e.g. `lift-01/check-1.bin`
    /// (max 511 chars + NUL).
    pub object_ref: [u8; 512],
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Copy `s` into `buf` as a null-terminated string.
/// Returns `EDS_ERR_STRING_TOO_LONG` if `s` does not fit (including the NUL).
fn copy_str_to_buf(s: &str, buf: &mut [u8]) -> i32 {
    if s.len() >= buf.len() {
        return EDS_ERR_STRING_TOO_LONG;
    }
    buf[..s.len()].copy_from_slice(s.as_bytes());
    buf[s.len()] = 0;
    EDS_OK
}

/// Read a null-terminated string from a fixed byte buffer.
fn read_str_from_buf(buf: &[u8]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

/// Convert an `EdsAuditRecord` reference to an `AuditRecord`.
fn to_audit_record(r: &EdsAuditRecord) -> AuditRecord {
    AuditRecord {
        device_id: read_str_from_buf(&r.device_id),
        sequence: r.sequence,
        timestamp_ms: r.timestamp_ms,
        payload_hash: r.payload_hash,
        signature: r.signature,
        prev_record_hash: r.prev_record_hash,
        object_ref: read_str_from_buf(&r.object_ref),
    }
}

// ── Public FFI functions ──────────────────────────────────────────────────────

/// Generate a fresh Ed25519 keypair using the OS CSPRNG.
///
/// # Parameters
/// - `private_key_out`: caller-allocated buffer for the 32-byte private key.
/// - `public_key_out`:  caller-allocated buffer for the 32-byte public key.
///
/// # Returns
/// `EDS_OK` on success, or a negative error code.
#[no_mangle]
pub unsafe extern "C" fn eds_keygen(
    private_key_out: *mut u8,
    public_key_out: *mut u8,
) -> i32 {
    std::panic::catch_unwind(|| {
        if private_key_out.is_null() || public_key_out.is_null() {
            return EDS_ERR_NULL_PTR;
        }
        let signing_key = SigningKey::generate(&mut OsRng);
        std::slice::from_raw_parts_mut(private_key_out, 32)
            .copy_from_slice(&signing_key.to_bytes());
        std::slice::from_raw_parts_mut(public_key_out, 32)
            .copy_from_slice(&signing_key.verifying_key().to_bytes());
        EDS_OK
    })
    .unwrap_or(EDS_ERR_PANIC)
}

/// Create a signed audit record.
///
/// Computes `BLAKE3(payload)`, signs the hash with the device private key, and
/// writes the resulting `EdsAuditRecord` to `out`.
///
/// # Parameters
/// - `device_id`:         null-terminated device identifier string.
/// - `sequence`:          monotonically increasing sequence number (start at 1).
/// - `timestamp_ms`:      Unix epoch in milliseconds.
/// - `payload`:           raw payload bytes.
/// - `payload_len`:       length of `payload` in bytes.
/// - `prev_record_hash`:  32-byte hash of the previous record, or NULL / all-zeros
///                        for the first record in a chain.
/// - `object_ref`:        null-terminated storage reference string.
/// - `private_key`:       32-byte Ed25519 private key.
/// - `out`:               caller-allocated `EdsAuditRecord` to fill.
///
/// # Returns
/// `EDS_OK` on success, or a negative error code.
#[no_mangle]
pub unsafe extern "C" fn eds_sign_record(
    device_id: *const c_char,
    sequence: u64,
    timestamp_ms: u64,
    payload: *const u8,
    payload_len: usize,
    prev_record_hash: *const u8,
    object_ref: *const c_char,
    private_key: *const u8,
    out: *mut EdsAuditRecord,
) -> i32 {
    std::panic::catch_unwind(|| {
        if device_id.is_null()
            || payload.is_null()
            || object_ref.is_null()
            || private_key.is_null()
            || out.is_null()
        {
            return EDS_ERR_NULL_PTR;
        }

        let device_id_str = match CStr::from_ptr(device_id).to_str() {
            Ok(s) => s,
            Err(_) => return EDS_ERR_INVALID_UTF8,
        };
        let object_ref_str = match CStr::from_ptr(object_ref).to_str() {
            Ok(s) => s,
            Err(_) => return EDS_ERR_INVALID_UTF8,
        };

        let payload_slice = std::slice::from_raw_parts(payload, payload_len);
        let key_bytes: [u8; 32] = match std::slice::from_raw_parts(private_key, 32).try_into() {
            Ok(b) => b,
            Err(_) => return EDS_ERR_INVALID_KEY,
        };
        let prev_hash: [u8; 32] = if prev_record_hash.is_null() {
            [0u8; 32]
        } else {
            match std::slice::from_raw_parts(prev_record_hash, 32).try_into() {
                Ok(b) => b,
                Err(_) => return EDS_ERR_INVALID_KEY,
            }
        };

        let signing_key = SigningKey::from_bytes(&key_bytes);
        let payload_hash = compute_payload_hash(payload_slice);
        let signature = sign_payload_hash(&signing_key, &payload_hash);

        let out_ref = &mut *out;
        out_ref.sequence = sequence;
        out_ref.timestamp_ms = timestamp_ms;
        out_ref.payload_hash = payload_hash;
        out_ref.signature = signature;
        out_ref.prev_record_hash = prev_hash;

        let rc = copy_str_to_buf(device_id_str, &mut out_ref.device_id);
        if rc != EDS_OK {
            return rc;
        }
        copy_str_to_buf(object_ref_str, &mut out_ref.object_ref)
    })
    .unwrap_or(EDS_ERR_PANIC)
}

/// Compute the record hash used as `prev_record_hash` for the next record.
///
/// The hash covers all fields of the record (BLAKE3 over its postcard encoding).
///
/// # Parameters
/// - `record`:   the record to hash.
/// - `hash_out`: caller-allocated 32-byte buffer for the result.
///
/// # Returns
/// `EDS_OK` on success, or a negative error code.
#[no_mangle]
pub unsafe extern "C" fn eds_record_hash(
    record: *const EdsAuditRecord,
    hash_out: *mut u8,
) -> i32 {
    std::panic::catch_unwind(|| {
        if record.is_null() || hash_out.is_null() {
            return EDS_ERR_NULL_PTR;
        }
        let audit_record = to_audit_record(&*record);
        let hash = audit_record.hash();
        std::slice::from_raw_parts_mut(hash_out, 32).copy_from_slice(&hash);
        EDS_OK
    })
    .unwrap_or(EDS_ERR_PANIC)
}

/// Verify the Ed25519 signature on a single record.
///
/// # Parameters
/// - `record`:     the record to verify.
/// - `public_key`: 32-byte Ed25519 public key of the signing device.
///
/// # Returns
/// `1` if the signature is valid, `0` if invalid, or a negative error code.
#[no_mangle]
pub unsafe extern "C" fn eds_verify_record(
    record: *const EdsAuditRecord,
    public_key: *const u8,
) -> i32 {
    std::panic::catch_unwind(|| {
        if record.is_null() || public_key.is_null() {
            return EDS_ERR_NULL_PTR;
        }
        let key_bytes: [u8; 32] = match std::slice::from_raw_parts(public_key, 32).try_into() {
            Ok(b) => b,
            Err(_) => return EDS_ERR_INVALID_KEY,
        };
        let verifying_key = match VerifyingKey::from_bytes(&key_bytes) {
            Ok(k) => k,
            Err(_) => return EDS_ERR_INVALID_KEY,
        };
        let rec = &*record;
        if verify_payload_signature(&verifying_key, &rec.payload_hash, &rec.signature) {
            1
        } else {
            0
        }
    })
    .unwrap_or(EDS_ERR_PANIC)
}

/// Verify that an array of records forms a valid BLAKE3 hash chain.
///
/// Checks that each record's `prev_record_hash` matches the hash of the
/// preceding record and that sequence numbers are strictly monotonically
/// increasing.
///
/// # Parameters
/// - `records`: pointer to an array of `count` consecutive `EdsAuditRecord`s.
/// - `count`:   number of records in the array.
///
/// # Returns
/// `EDS_OK` if the chain is valid, `EDS_ERR_CHAIN_INVALID` if not, or a
/// negative error code.
#[no_mangle]
pub unsafe extern "C" fn eds_verify_chain(
    records: *const EdsAuditRecord,
    count: usize,
) -> i32 {
    std::panic::catch_unwind(|| {
        if records.is_null() && count > 0 {
            return EDS_ERR_NULL_PTR;
        }
        let slice = std::slice::from_raw_parts(records, count);
        let audit_records: Vec<AuditRecord> = slice.iter().map(to_audit_record).collect();
        match verify_chain(&audit_records) {
            Ok(()) => EDS_OK,
            Err(_) => EDS_ERR_CHAIN_INVALID,
        }
    })
    .unwrap_or(EDS_ERR_PANIC)
}
