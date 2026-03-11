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

## 6. Software update integrity

Before a device applies any firmware or software update, the update package must pass two checks via `edgesentry_rs::update::UpdateVerifier`:

1. **Payload hash** — `BLAKE3(raw_payload)` must match the hash embedded in the `SoftwareUpdate` manifest
2. **Publisher signature** — the Ed25519 signature over that hash must verify against a registered trusted publisher key

Every attempt (accepted or rejected) is appended to `UpdateVerificationLog` for auditing. This satisfies CLS-03 / ETSI EN 303 645 §5.3 / JC-STAR STAR-2 R2.2.

## 7. Network policy (deny-by-default)

`edgesentry_rs::ingest::NetworkPolicy` enforces a deny-by-default IP/CIDR allowlist for incoming connections. Callers call `NetworkPolicy::check(source_ip)` **before** passing a record to `IngestService`. Connections from unlisted addresses are rejected without reaching any cryptographic check.

Rules are additive: `allow_ip(addr)` for exact matches and `allow_cidr("10.0.0.0/8")` for CIDR blocks (IPv4 and IPv6). An empty policy denies everything.

## 8. Ingest-time verification

`edgesentry_rs::ingest` is responsible for completing trust checks before persistence.

The full check order when ingesting a record is:

1. **Network gate** — `NetworkPolicy::check(source_ip)` denies unlisted sources before any crypto runs
2. **Payload hash** — `IngestService` verifies raw payload matches `record.payload_hash`
3. **Route identity** — `cert_identity` must match `record.device_id` when present
4. **Signature** — payload hash must be signed by the registered device key
5. **Sequence** — must be strictly monotonic and non-duplicate per device
6. **Previous-record hash** — must chain from the last accepted record's hash

Steps 3–6 are enforced by `IntegrityPolicyGate`; step 2 by `IngestService` before invoking the gate.

## 9. Storage model

On accepted ingest, the system stores:

- Raw data (payload body)
- Audit ledger (audit record stream)
- Operation log (accept/reject decisions)

This separation keeps evidence metadata and payload storage independently manageable.

## 10. Demo modes

### 10.1 Library example (no DB/MinIO required)

- Run: `cargo run -p edgesentry-rs --example lift_inspection_flow`
- Uses in-memory stores
- Fast path to verify signing, ingest verification, and tamper rejection

### 10.2 Interactive local demo (DB/MinIO required)

- Run: `bash scripts/local_demo.sh`
- End-to-end flow with PostgreSQL + MinIO + CLI
- Shows persisted audit records and operation logs

## 11. Trust boundary

- Device side: signs facts and emits compact audit metadata
- Cloud side: enforces strict verification rules before accepting data

This split keeps edge and cloud responsibilities clear and auditable.

## 12. Quality and release concepts

- Static analysis: `clippy`
- OSS license policy validation: `cargo-deny`
- Advisory scanning: `cargo-audit` (CVE checks against RustSec advisory DB)
- Release readiness: CI + release workflows
- Tag-driven release: `vX.Y.Z`

See [Contributing](contributing.md) and [Build and Release](release.md) for executable procedures.

## 13. STRIDE threat model

SS 711:2025 and the IMDA IoT Cyber Security Guide require recorded STRIDE-based threat model artifacts for CLS Level 3 assessment. The six threat categories map to EdgeSentry-RS attack surfaces as follows:

| Threat | Attack surface | Mitigation |
|--------|---------------|------------|
| **S**poofing | Device identity | Ed25519 signature — only the registered public key can verify a record |
| **T**ampering | Audit records, payload storage | BLAKE3 hash chain — any modification breaks chain continuity |
| **R**epudiation | Ingest decisions | `OperationLog` records every accept/reject decision with reason |
| **I**nformation Disclosure | Raw payload storage | `object_ref` separation keeps payload body out of the audit metadata stream |
| **D**enial of Service | Ingest endpoint | `NetworkPolicy` deny-by-default rejects unlisted sources before any crypto runs |
| **E**levation of Privilege | Ingest gate | `IntegrityPolicyGate` verifies device registration and signature before accepting data |

Producing the formal design artifact for CLS Level 3 assessment is tracked in [#93](https://github.com/yohei1126/edgesentry-rs/issues/93).

## 14. SBOM (Software Bill of Materials)

A Software Bill of Materials lists all software components and their versions used in a product. The IMDA IoT Cyber Security Guide requires SBOM availability as part of the lifecycle support category in the vendor disclosure checklist — a mandatory CLS Level 3 evidence artifact.

For Rust projects, SBOM is generated from `Cargo.lock` using tools such as `cargo-sbom` or `cargo-cyclonedx`, producing a machine-readable inventory of all crates and their transitive dependencies.

Generating and publishing the SBOM alongside the vendor disclosure checklist is tracked in [#92](https://github.com/yohei1126/edgesentry-rs/issues/92).
