# Step 7 - Seal

Sign each record and chain it to the previous one using BLAKE3 + Ed25519. A tampered record
breaks the chain, which `eds audit verify-chain` detects immediately.

The `edgesentry-audit` crate is the implementation; see its dedicated book for full detail
on key management, deployment, and threat model. This page covers the CLI commands used
in the Phase 1-3 pipeline demos.

## Demo audit chain

Generate a pre-built lift-inspection chain for demo purposes:

```
eds audit demo-lift-inspection
  --device-id <ID>
  --private-key-hex <HEX>
  --out-file <FILE>
  [--start-timestamp-ms <MS>]
  [--object-prefix <PREFIX>]
  [--payloads-file <FILE>]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--device-id` | `lift-01` | Device identifier embedded in each record |
| `--private-key-hex` | `0101...01` | Ed25519 private key as 64 hex characters |
| `--out-file` | `lift_inspection_records.json` | Output JSON array of AuditRecord |
| `--start-timestamp-ms` | `1700000000000` | Timestamp of the first record |
| `--payloads-file` | | Optional: write raw payloads as hex strings (for demo-ingest) |

The demo key (`0101...01` repeated 32 bytes) is for local testing only. Generate a real
keypair with `eds audit keygen` before any production deployment.

## Verify a chain

```
eds audit verify-chain --records-file <FILE>
```

Reads the JSON array of AuditRecord, recomputes each BLAKE3 hash, verifies each Ed25519
signature, and confirms that `prev_hash` in record N matches the hash of record N-1. Exits
0 on success; exits 1 with a specific error message on any failure.

## Key management

```bash
# Generate a new Ed25519 keypair
eds audit keygen [--out <FILE>]

# Derive public key from an existing private key
eds audit inspect-key --private-key-hex <HEX> [--out <FILE>]
```

Store private keys in a secrets manager or hardware security module for production use.
The public key can be distributed freely -- it is only used for verification.

## AuditRecord structure

Each record contains:

```json
{
  "sequence": 1,
  "device_id": "demo-edge-01",
  "timestamp_ms": 1700000001000,
  "payload_hash": "<BLAKE3 hex of the payload bytes>",
  "prev_hash": "<BLAKE3 hex of the previous record>",
  "signature": "<Ed25519 signature hex over hash of all above fields>"
}
```

The chain starts with `prev_hash` all zeros for the first record. Any insertion, deletion,
or modification of any field invalidates all signatures from that record onward.
