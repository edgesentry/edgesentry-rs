#ifndef EDGESENTRY_BRIDGE_H
#define EDGESENTRY_BRIDGE_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <stdint.h>
#include <stddef.h>

/**
 * Operation completed successfully.
 */
#define EDS_OK 0

/**
 * A required pointer argument was NULL.
 */
#define EDS_ERR_NULL_PTR -1

/**
 * A string argument contains invalid UTF-8.
 */
#define EDS_ERR_INVALID_UTF8 -2

/**
 * A key or hash byte buffer is invalid or has the wrong length.
 */
#define EDS_ERR_INVALID_KEY -3

/**
 * A string argument is longer than the fixed output buffer allows.
 */
#define EDS_ERR_STRING_TOO_LONG -4

/**
 * Hash-chain verification failed (tampered, reordered, or truncated chain).
 */
#define EDS_ERR_CHAIN_INVALID -5

/**
 * An unexpected Rust panic was caught at the FFI boundary.
 */
#define EDS_ERR_PANIC -6

/**
 * A tamper-evident audit record with a C-compatible layout.
 *
 * All fields are value types — no heap allocation.  `device_id` and
 * `object_ref` are null-terminated strings stored in fixed-size byte arrays.
 */
typedef struct EdsAuditRecord {
    /**
     * Monotonically increasing record index for this device.
     */
    uint64_t sequence;
    /**
     * Unix epoch in milliseconds when the record was created.
     */
    uint64_t timestamp_ms;
    /**
     * BLAKE3 hash of the raw payload (32 bytes).
     */
    uint8_t payload_hash[32];
    /**
     * Ed25519 signature over `payload_hash` (64 bytes).
     */
    uint8_t signature[64];
    /**
     * Hash of the preceding record, or all-zeros for the first record (32 bytes).
     */
    uint8_t prev_record_hash[32];
    /**
     * Null-terminated device identifier (max 255 chars + NUL).
     */
    uint8_t device_id[256];
    /**
     * Null-terminated storage reference, e.g. `lift-01/check-1.bin`
     * (max 511 chars + NUL).
     */
    uint8_t object_ref[512];
} EdsAuditRecord;

/**
 * Generate a fresh Ed25519 keypair using the OS CSPRNG.
 *
 * # Safety
 *
 * `private_key_out` and `public_key_out` must each be non-null and point to
 * at least 32 bytes of valid writable memory.  Passing NULL returns
 * `EDS_ERR_NULL_PTR` without writing anything.
 *
 * # Parameters
 * - `private_key_out`: caller-allocated buffer for the 32-byte private key.
 * - `public_key_out`:  caller-allocated buffer for the 32-byte public key.
 *
 * # Returns
 * `EDS_OK` on success, or a negative error code.
 */
int32_t eds_keygen(uint8_t *private_key_out, uint8_t *public_key_out);

/**
 * Create a signed audit record.
 *
 * Computes `BLAKE3(payload)`, signs the hash with the device private key, and
 * writes the resulting `EdsAuditRecord` to `out`.
 *
 * # Safety
 *
 * - `device_id` and `object_ref` must be non-null, null-terminated UTF-8 strings.
 * - `payload` must be non-null and valid for `payload_len` bytes.
 * - `private_key` must be non-null and point to exactly 32 bytes.
 * - `prev_record_hash`, if non-null, must point to exactly 32 bytes.
 * - `out` must be non-null and point to a valid writable `EdsAuditRecord`.
 *
 * # Parameters
 * - `device_id`:        null-terminated device identifier string.
 * - `sequence`:         monotonically increasing sequence number (start at 1).
 * - `timestamp_ms`:     Unix epoch in milliseconds.
 * - `payload`:          raw payload bytes.
 * - `payload_len`:      length of `payload` in bytes.
 * - `prev_record_hash`: 32-byte hash of the previous record, or NULL for the first record.
 * - `object_ref`:       null-terminated storage reference string.
 * - `private_key`:      32-byte Ed25519 private key.
 * - `out`:              caller-allocated `EdsAuditRecord` to fill.
 *
 * # Returns
 * `EDS_OK` on success, or a negative error code.
 */
int32_t eds_sign_record(const char *device_id,
                        uint64_t sequence,
                        uint64_t timestamp_ms,
                        const uint8_t *payload,
                        uintptr_t payload_len,
                        const uint8_t *prev_record_hash,
                        const char *object_ref,
                        const uint8_t *private_key,
                        struct EdsAuditRecord *out);

/**
 * Compute the record hash used as `prev_record_hash` for the next record.
 *
 * The hash covers all fields of the record (BLAKE3 over its postcard encoding).
 *
 * # Safety
 *
 * - `record` must be non-null and point to a valid `EdsAuditRecord`.
 * - `hash_out` must be non-null and point to at least 32 bytes of writable memory.
 *
 * # Parameters
 * - `record`:   the record to hash.
 * - `hash_out`: caller-allocated 32-byte buffer for the result.
 *
 * # Returns
 * `EDS_OK` on success, or a negative error code.
 */
int32_t eds_record_hash(const struct EdsAuditRecord *record, uint8_t *hash_out);

/**
 * Verify the Ed25519 signature on a single record.
 *
 * # Safety
 *
 * - `record` must be non-null and point to a valid `EdsAuditRecord`.
 * - `public_key` must be non-null and point to exactly 32 bytes.
 *
 * # Parameters
 * - `record`:     the record to verify.
 * - `public_key`: 32-byte Ed25519 public key of the signing device.
 *
 * # Returns
 * `1` if the signature is valid, `0` if invalid, or a negative error code.
 */
int32_t eds_verify_record(const struct EdsAuditRecord *record, const uint8_t *public_key);

/**
 * Verify that an array of records forms a valid BLAKE3 hash chain.
 *
 * Checks that each record's `prev_record_hash` matches the hash of the
 * preceding record and that sequence numbers are strictly monotonically
 * increasing.
 *
 * # Safety
 *
 * - `records` must be non-null and point to `count` consecutive, valid
 *   `EdsAuditRecord` values.  Passing NULL with `count == 0` is safe and
 *   returns `EDS_OK`.
 *
 * # Parameters
 * - `records`: pointer to an array of `count` consecutive `EdsAuditRecord`s.
 * - `count`:   number of records in the array.
 *
 * # Returns
 * `EDS_OK` if the chain is valid, `EDS_ERR_CHAIN_INVALID` if not, or a
 * negative error code.
 */
int32_t eds_verify_chain(const struct EdsAuditRecord *records, uintptr_t count);

#endif  /* EDGESENTRY_BRIDGE_H */
