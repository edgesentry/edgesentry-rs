# Concepts in edgesentry-rs

This document summarizes the core concepts used in this repository.

## 1. Tamper-evident design

The primary goal is not "perfect tamper prevention," but "reliable tamper detection."

- Compute a hash from the original payload
- Sign the hash with a device private key
- Link records through a hash chain

Together, these mechanisms detect tampering, spoofing, and record reordering.

## 2. AuditRecord

The basic unit of evidence is `AuditRecord`. Key fields:

- `device_id`: source device identity
- `sequence`: monotonically increasing sequence number
- `timestamp_ms`: event timestamp
- `payload_hash`: hash of raw payload data
- `signature`: signature over `payload_hash`
- `prev_record_hash`: hash of the previous audit record
- `object_ref`: reference to raw payload storage (for example, `s3://...`)

## 3. Hash and signature

### 3.1 Hash (integrity)

- Purpose: fingerprint of payload content
- Property: even a 1-byte payload change produces a different hash

### 3.2 Signature (authenticity)

- Purpose: prove the payload hash was produced by a trusted device key
- Verification: validate with the registered device public key

## 4. Hash chain continuity

Records are linked by `prev_record_hash`.

- First record: `prev_record_hash = zero_hash`
- Subsequent records: must match the previous record's `hash()`

This detects insertion, deletion, and substitution inside the chain.

## 5. Sequence policy

`sequence` must increase per device as 1, 2, 3, ...

- Duplicate sequence values are rejected
- Gaps or out-of-order sequences are rejected

## 6. Ingest-time verification

`edgesentry_rs::ingest` is responsible for completing trust checks before persistence.

The `IntegrityPolicyGate` is the explicit P0 gate that enforces all integrity rules before a record is allowed through to storage. It runs in order:

1. **Route identity** — `cert_identity` must match `record.device_id` when present
2. **Signature** — payload hash must be signed by the registered device key
3. **Sequence** — must be strictly monotonic and non-duplicate per device
4. **Previous-record hash** — must chain from the last accepted record's hash

`IngestService` additionally checks that the raw payload matches `payload_hash`. This check runs after the policy gate — the policy gate enforces identity, signature, sequence, and chain continuity first, then the payload hash is verified before the record is persisted.

## 7. Storage model

On accepted ingest, the system stores:

- Raw data (payload body)
- Audit ledger (audit record stream)
- Operation log (accept/reject decisions)

This separation keeps evidence metadata and payload storage independently manageable.

## 8. Demo modes

### 8.1 Library example (no DB/MinIO required)

- Run: `cargo run -p edgesentry-rs --example lift_inspection_flow`
- Uses in-memory stores
- Fast path to verify signing, ingest verification, and tamper rejection

### 8.2 Interactive local demo (DB/MinIO required)

- Run: `bash scripts/local_demo.sh`
- End-to-end flow with PostgreSQL + MinIO + CLI
- Shows persisted audit records and operation logs

## 9. Trust boundary

- Device side: signs facts and emits compact audit metadata
- Cloud side: enforces strict verification rules before accepting data

This split keeps edge and cloud responsibilities clear and auditable.

## 10. Quality and release concepts

- Static analysis: `clippy`
- OSS license policy validation: `cargo-deny`
- Release readiness: CI + release workflows
- Tag-driven release: `vX.Y.Z`

See [Contributing](contributing.md) and [Build and Release](release.md) for executable procedures.
