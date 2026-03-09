# C/C++ FFI Bridge

`edgesentry-bridge` is a separate Rust crate that exposes Ed25519 signing and
BLAKE3 hash-chain verification as a stable C ABI.  C and C++ firmware or
gateways can call the same security logic as the Rust library without a full
rewrite.

---

## Building the library

```bash
cargo build -p edgesentry-bridge --release
```

This produces:

| Platform | File |
|----------|------|
| macOS | `target/release/libedgesentry_bridge.dylib` and `.a` |
| Linux | `target/release/libedgesentry_bridge.so` and `.a` |

The header `crates/edgesentry-bridge/include/edgesentry_bridge.h` is
regenerated automatically by `build.rs` using `cbindgen`.

---

## Linking from C/C++

**macOS:**

```bash
cc -o my_app main.c \
   -I path/to/edgesentry-bridge/include \
   -L path/to/target/release \
   -ledgesentry_bridge \
   -framework Security -framework CoreFoundation
```

**Linux:**

```bash
cc -o my_app main.c \
   -I path/to/edgesentry-bridge/include \
   -L path/to/target/release \
   -ledgesentry_bridge \
   -lpthread -ldl
```

A ready-made `Makefile` is provided in
`crates/edgesentry-bridge/examples/c_integration/`.

---

## API reference

### Error codes

| Constant | Value | Meaning |
|----------|-------|---------|
| `EDS_OK` | `0` | Success |
| `EDS_ERR_NULL_PTR` | `-1` | A required pointer was NULL |
| `EDS_ERR_INVALID_UTF8` | `-2` | String argument is not valid UTF-8 |
| `EDS_ERR_INVALID_KEY` | `-3` | Key or hash buffer is invalid |
| `EDS_ERR_STRING_TOO_LONG` | `-4` | String exceeds fixed buffer size |
| `EDS_ERR_CHAIN_INVALID` | `-5` | Hash-chain verification failed |
| `EDS_ERR_PANIC` | `-6` | Unexpected internal error |

### Record struct

```c
typedef struct {
    uint64_t sequence;           /* monotonic record index (starts at 1) */
    uint64_t timestamp_ms;       /* Unix epoch in milliseconds           */
    uint8_t  payload_hash[32];   /* BLAKE3 hash of the raw payload        */
    uint8_t  signature[64];      /* Ed25519 signature over payload_hash   */
    uint8_t  prev_record_hash[32]; /* hash of preceding record (zero for first) */
    uint8_t  device_id[256];     /* null-terminated device identifier     */
    uint8_t  object_ref[512];    /* null-terminated storage reference     */
} EdsAuditRecord;
```

`EdsAuditRecord` is **caller-allocated**.  Rust never calls `malloc` or
returns a heap pointer — no `_free` function is needed.

### Functions

```c
/* Generate an Ed25519 keypair via OS CSPRNG.
   private_key_out and public_key_out must each point to 32 bytes. */
int32_t eds_keygen(uint8_t *private_key_out, uint8_t *public_key_out);

/* Hash payload with BLAKE3, sign with Ed25519, fill *out.
   Pass NULL for prev_record_hash to use the zero hash (first record). */
int32_t eds_sign_record(const char    *device_id,
                        uint64_t       sequence,
                        uint64_t       timestamp_ms,
                        const uint8_t *payload,
                        size_t         payload_len,
                        const uint8_t *prev_record_hash,
                        const char    *object_ref,
                        const uint8_t *private_key,
                        EdsAuditRecord *out);

/* Compute the per-record hash (used as prev_record_hash for the next record).
   hash_out must point to 32 bytes. */
int32_t eds_record_hash(const EdsAuditRecord *record, uint8_t *hash_out);

/* Verify Ed25519 signature. Returns 1 valid, 0 invalid, negative on error. */
int32_t eds_verify_record(const EdsAuditRecord *record,
                          const uint8_t *public_key);

/* Verify the entire hash chain. Returns EDS_OK or EDS_ERR_CHAIN_INVALID. */
int32_t eds_verify_chain(const EdsAuditRecord *records, size_t count);
```

---

## Minimal C example

```c
#include "edgesentry_bridge.h"
#include <string.h>
#include <assert.h>

int main(void) {
    uint8_t priv_key[32], pub_key[32];
    assert(eds_keygen(priv_key, pub_key) == EDS_OK);

    const char *payload = "check=door,status=ok";
    EdsAuditRecord rec;
    memset(&rec, 0, sizeof(rec));

    assert(eds_sign_record("lift-01", 1, 1700000000000ULL,
                           (const uint8_t *)payload, strlen(payload),
                           NULL,              /* zero hash — first record */
                           "lift-01/1.bin",
                           priv_key, &rec) == EDS_OK);

    assert(eds_verify_record(&rec, pub_key) == 1);
    return 0;
}
```

See the full example in
`crates/edgesentry-bridge/examples/c_integration/main.c`.

---

## Memory safety conventions

| Rule | Detail |
|------|--------|
| No heap allocation | `EdsAuditRecord` is caller-allocated; Rust never calls `malloc` |
| NULL-checked | Every pointer argument is checked; `EDS_ERR_NULL_PTR` returned on failure |
| Fixed-size strings | `device_id` max 255 chars; `object_ref` max 511 chars — truncated inputs return `EDS_ERR_STRING_TOO_LONG` |
| Panic safety | `std::panic::catch_unwind` wraps every FFI function; a Rust panic returns `EDS_ERR_PANIC` instead of unwinding across the C boundary |
| Key sizes | `private_key` and `public_key` must point to exactly 32 bytes; hash buffers to 32 bytes; signature buffer to 64 bytes |

---

## HSM path

For CLS Level 4, the private key should never exist as an extractable byte
array.  The planned HSM integration ([#54](https://github.com/yohei1126/edgesentry-rs/issues/54))
will delegate the `eds_sign_record` operation to an HSM-backed provider
without exposing key bytes to the caller.
