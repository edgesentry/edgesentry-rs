# C/C++ FFI Bridge

`edgesentry-bridge` is a separate Rust crate that exposes Ed25519 signing and
BLAKE3 hash-chain verification as a stable C ABI.  C and C++ firmware or
gateways can call the same security logic as the Rust library without a full
rewrite.  Downstream languages (including Python) must reach the same ABI or
the `eds audit` CLI — see [Canonicalization contract](#canonicalization-contract).

---

## Canonicalization contract

`AuditRecord::hash()` is defined as:

```text
blake3(postcard(record))
```

(see `crates/edgesentry-audit/src/record.rs`).  postcard is schema-driven and
**not** self-describing: field order, varint integer encoding, fixed-size
arrays without length prefixes, and `serialize_bytes` length prefixes must
match byte-for-byte.

**Downstream implementations MUST NOT reimplement postcard** (or otherwise
recompute the record hash outside Rust).  A subtly wrong reimplementation
still succeeds on write and only fails at verification — the worst failure
mode for an audit chain.

Canonical hashing, signing, and verification must go through one of:

| Path | Mechanism | Typical use |
|------|-----------|-------------|
| **Write (in-process)** | C ABI: `eds_sign_record`, `eds_record_hash`, … | Hot path from C/C++ or `ctypes` |
| **Verify (out-of-process)** | `eds audit verify-chain --records-file <path>` | Independent re-check; verifier is a **separate binary** from the writer |

Keeping write and verify in different binaries is intentional: the same Rust
implementation is exercised on two process boundaries, so chain validity is
not a self-check inside a single address space.

**Fallback:** if the shared library cannot be loaded, both write and verify
may use the CLI (`eds audit sign-record` / `eds audit verify-chain`).  See
`tools/seal_events.py` for a subprocess write example.  Prefer the FFI write
path when available.

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

On newer macOS, if `ctypes.CDLL` / `dlopen` fails with a mis-aligned
LINKEDIT error on the rustc-produced `.dylib`, re-link from the staticlib:

```bash
cc -dynamiclib -o target/release/libedgesentry_bridge.dylib \
  -Wl,-force_load,target/release/libedgesentry_bridge.a \
  -framework Security -framework CoreFoundation \
  -lSystem -liconv \
  -install_name libedgesentry_bridge.dylib
```

### Prebuilt release assets

GitHub Releases ship sibling archives alongside `eds` for the same
Unix targets used by the CLI matrix:

| Target | Archive |
|--------|---------|
| `x86_64-unknown-linux-gnu` | `libedgesentry_bridge-{tag}-{target}.tar.gz` (`.so` + header) |
| `aarch64-apple-darwin` | `libedgesentry_bridge-{tag}-{target}.tar.gz` (`.dylib` + header) |

Each archive contains the shared library and `edgesentry_bridge.h` at the
top level. On macOS, release builds re-link the cdylib with Apple `ld` from
the staticlib so `dlopen` succeeds on newer macOS (rustc-produced dylibs can
fail with a mis-aligned LINKEDIT string pool).

Example (Linux CI):

```bash
TAG=vX.Y.Z
TARGET=x86_64-unknown-linux-gnu
curl -fsSL \
  "https://github.com/edgesentry/edgesentry-rs/releases/download/${TAG}/libedgesentry_bridge-${TAG}-${TARGET}.tar.gz" \
  | tar -xz
python3 -c "import ctypes; ctypes.CDLL('./libedgesentry_bridge.so'); print('ok')"
```

Point consumers at the extracted library with `EDS_BRIDGE_LIB` (or pass the
path to `ctypes.CDLL` directly). Windows DLL assets are not published.

### Cross-compiling for aarch64 (Linux)

For edge hosts (e.g. Raspberry Pi) that need `libedgesentry_bridge.so` and
the `eds` CLI on `aarch64-unknown-linux-gnu` — this target is **not**
shipped as a GitHub Release asset; build locally or in CI:

```bash
# Install the target once (rustup)
rustup target add aarch64-unknown-linux-gnu

# eds CLI
cargo build -p eds --release --target aarch64-unknown-linux-gnu

# bridge shared library
cargo build -p edgesentry-bridge --release --target aarch64-unknown-linux-gnu
# → target/aarch64-unknown-linux-gnu/release/libedgesentry_bridge.so
# → target/aarch64-unknown-linux-gnu/release/eds
```

A linker for the target (for example `aarch64-linux-gnu-gcc` via
`cross` or a distro cross-toolchain) must be available. CI currently
cross-builds the `eds` binary only for this target.

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
| `EDS_ERR_HASH_MISMATCH` | `-7` | Payload hash does not match expected value |
| `EDS_ERR_BAD_SIGNATURE` | `-8` | Ed25519 signature is invalid |

After any call that returns a negative error code, call `eds_last_error_message()` to retrieve a human-readable description of the failure.

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

/* Verify a software update before installation (CLS-03 / STAR-2 R2.2).
   Checks BLAKE3(payload) == payload_hash, then verifies the Ed25519
   publisher signature over payload_hash.
   payload_hash must point to 32 bytes; signature to 64 bytes;
   publisher_key to 32 bytes.
   Returns EDS_OK, EDS_ERR_HASH_MISMATCH, EDS_ERR_BAD_SIGNATURE, or
   EDS_ERR_INVALID_KEY / EDS_ERR_NULL_PTR on bad inputs. */
int32_t eds_verify_update(const uint8_t *payload,
                          size_t         payload_len,
                          const uint8_t *payload_hash,
                          const uint8_t *signature,
                          const uint8_t *publisher_key);

/* Return a thread-local human-readable description of the last error.
   The pointer is valid until the next eds_* call on this thread.
   Returns "" when no error has occurred.  Never returns NULL. */
const char *eds_last_error_message(void);
```

---

## Minimal C example

```c
#include "edgesentry_bridge.h"
#include <string.h>
#include <assert.h>

int main(void) {
    uint8_t priv_key[32], pub_key[32];
    if (eds_keygen(priv_key, pub_key) != EDS_OK) {
        fprintf(stderr, "keygen failed: %s\n", eds_last_error_message());
        return 1;
    }

    const char *payload = "check=door,status=ok";
    EdsAuditRecord rec;
    memset(&rec, 0, sizeof(rec));

    int rc = eds_sign_record("lift-01", 1, 1700000000000ULL,
                             (const uint8_t *)payload, strlen(payload),
                             NULL,              /* zero hash — first record */
                             "lift-01/1.bin",
                             priv_key, &rec);
    if (rc != EDS_OK) {
        fprintf(stderr, "sign_record failed: %s\n", eds_last_error_message());
        return 1;
    }

    assert(eds_verify_record(&rec, pub_key) == 1);
    return 0;
}
```

See the full example in
`crates/edgesentry-bridge/examples/c_integration/main.c`.

---

## Minimal Python (`ctypes`) example

Build the library first (`cargo build -p edgesentry-bridge --release`).
Do **not** hash or sign records in pure Python — load the shared library.

```python
#!/usr/bin/env python3
"""Minimal ctypes consumer of libedgesentry_bridge (canonical write path)."""

from __future__ import annotations

import ctypes
import sys
from pathlib import Path

EDS_OK = 0


class EdsAuditRecord(ctypes.Structure):
    _fields_ = [
        ("sequence", ctypes.c_uint64),
        ("timestamp_ms", ctypes.c_uint64),
        ("payload_hash", ctypes.c_uint8 * 32),
        ("signature", ctypes.c_uint8 * 64),
        ("prev_record_hash", ctypes.c_uint8 * 32),
        ("device_id", ctypes.c_uint8 * 256),
        ("object_ref", ctypes.c_uint8 * 512),
    ]


def load_bridge(release_dir: Path) -> ctypes.CDLL:
    if sys.platform == "darwin":
        name = "libedgesentry_bridge.dylib"
    else:
        name = "libedgesentry_bridge.so"
    lib = ctypes.CDLL(str(release_dir / name))

    lib.eds_last_error_message.restype = ctypes.c_char_p
    lib.eds_last_error_message.argtypes = []

    lib.eds_keygen.restype = ctypes.c_int32
    lib.eds_keygen.argtypes = [
        ctypes.POINTER(ctypes.c_uint8),
        ctypes.POINTER(ctypes.c_uint8),
    ]

    lib.eds_sign_record.restype = ctypes.c_int32
    lib.eds_sign_record.argtypes = [
        ctypes.c_char_p,
        ctypes.c_uint64,
        ctypes.c_uint64,
        ctypes.POINTER(ctypes.c_uint8),
        ctypes.c_size_t,
        ctypes.POINTER(ctypes.c_uint8),
        ctypes.c_char_p,
        ctypes.POINTER(ctypes.c_uint8),
        ctypes.POINTER(EdsAuditRecord),
    ]

    lib.eds_record_hash.restype = ctypes.c_int32
    lib.eds_record_hash.argtypes = [
        ctypes.POINTER(EdsAuditRecord),
        ctypes.POINTER(ctypes.c_uint8),
    ]

    lib.eds_verify_chain.restype = ctypes.c_int32
    lib.eds_verify_chain.argtypes = [
        ctypes.POINTER(EdsAuditRecord),
        ctypes.c_size_t,
    ]
    return lib


def check(lib: ctypes.CDLL, rc: int, what: str) -> None:
    if rc < 0:
        msg = lib.eds_last_error_message() or b""
        raise RuntimeError(f"{what} failed ({rc}): {msg.decode()}")


def main() -> int:
    release = Path("target/release")  # adjust if needed
    lib = load_bridge(release)

    priv = (ctypes.c_uint8 * 32)()
    pub = (ctypes.c_uint8 * 32)()
    check(lib, lib.eds_keygen(priv, pub), "eds_keygen")

    payload = b"check=door,status=ok"
    rec = EdsAuditRecord()
    check(
        lib,
        lib.eds_sign_record(
            b"lift-01",
            1,
            1_700_000_000_000,
            (ctypes.c_uint8 * len(payload)).from_buffer_copy(payload),
            len(payload),
            None,  # first record → zero prev hash
            b"lift-01/1.bin",
            priv,
            ctypes.byref(rec),
        ),
        "eds_sign_record",
    )

    nxt = (ctypes.c_uint8 * 32)()
    check(lib, lib.eds_record_hash(ctypes.byref(rec), nxt), "eds_record_hash")

    records = (EdsAuditRecord * 1)(rec)
    check(lib, lib.eds_verify_chain(records, 1), "eds_verify_chain")
    print("FFI chain OK; next prev_record_hash =", bytes(nxt).hex())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
```

**Recommended independent verification:** persist an `AuditRecord` JSON array
(for example via `eds audit sign-record` / `sign-document`, or by exporting
records your app already wrote through the Rust types) and re-check
out-of-process:

```bash
eds audit verify-chain --records-file /path/to/records.json
# prints CHAIN_VALID on success
```

In-process `eds_verify_chain` is fine for unit tests; production demos that
claim independent re-verification should use the CLI against a separate
`eds` binary.

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
array.  The planned HSM integration ([#54](https://github.com/edgesentry/edgesentry-rs/issues/54))
will delegate the `eds_sign_record` operation to an HSM-backed provider
without exposing key bytes to the caller.
