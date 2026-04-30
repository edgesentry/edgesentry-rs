# STRIDE Threat Model

This document is a formal threat-modelling artifact produced for Singapore CLS Level 3 assessment under SS 711:2025 **Rigour in Defence** and the IMDA IoT Cyber Security Guide threat-modelling checklist.  It covers all attack surfaces of the EdgeSentry-RS system: API, communication channel, and storage.

**Methodology:** STRIDE (Microsoft)
**Scope:** `edgesentry-rs` library and `edgesentry-bridge` FFI crate — device-side signing, cloud-side ingest, HTTP transport, operation log, and audit ledger.
**Assessor reference:** SS 711:2025 §4.2 Rigour in Defence; IMDA IoT Cyber Security Guide §3 Threat Modelling Checklist

---

## System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│  Field Device (edge)                                            │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  build_signed_record()                                     │ │
│  │  payload → BLAKE3 hash → Ed25519 sign → AuditRecord       │ │
│  └────────────────────────────────────────────────────────────┘ │
└────────────────────────────┬────────────────────────────────────┘
                             │ POST /api/v1/ingest (JSON over HTTPS)
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│  Cloud Ingest Layer                                             │
│  ┌────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │ NetworkPolicy  │  │ IntegrityPolicy │  │ AsyncIngest     │  │
│  │ IP/CIDR gate   │→ │ Gate            │→ │ Service         │  │
│  │ (deny-default) │  │ (signature +    │  │ (hash chain +   │  │
│  └────────────────┘  │  chain verify)  │  │  sequence)      │  │
│                      └─────────────────┘  └────────┬────────┘  │
│                                                     │           │
│            ┌────────────────────────────────────────┤           │
│            ▼                          ▼             ▼           │
│  ┌──────────────────┐  ┌─────────────────────┐  ┌──────────┐   │
│  │  Raw Data Store  │  │  Audit Ledger       │  │ Op. Log  │   │
│  │  (S3 / memory)   │  │  (Postgres / memory)│  │          │   │
│  └──────────────────┘  └─────────────────────┘  └──────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

---

## STRIDE Threat Analysis

### S — Spoofing (Device Identity)

**Threat:** An attacker impersonates a legitimate field device by forging the `device_id` field or replaying records signed by a compromised key.

**Attack surface:** `POST /api/v1/ingest` — `AuditRecord.device_id` and `AuditRecord.signature` fields.

| Sub-threat | Description |
|------------|-------------|
| S-1 | Attacker sends records with a valid `device_id` but self-generated Ed25519 key (unregistered) |
| S-2 | Attacker replays a previously captured, legitimately signed record |
| S-3 | Attacker sends records with a forged `device_id` that does not match the signing key |

**Mitigations:**

| ID | Mitigation | Code location |
|----|-----------|---------------|
| M-S-1 | Device public keys are pre-registered on the cloud side; any signature that does not verify against the registered key is rejected with `IngestError::UnknownDevice` | `ingest/policy.rs` `IntegrityPolicyGate::enforce()` |
| M-S-2 | Monotonic sequence numbers and `prev_record_hash` chain continuity are enforced; replayed records are detected as duplicate sequences | `ingest/verify.rs` `check_sequence()` |
| M-S-3 | Ed25519 signatures bind the payload hash to the private key; a forged `device_id` with the wrong key fails signature verification | `identity.rs` `verify_payload_signature()` |

**Residual risk:** If a device's private key is physically extracted, records can be forged with valid signatures.  Hardware-backed key storage (TPM/SE) is a device-layer control outside the scope of this library; it is noted in the [Roadmap](../audit/roadmap.md).

---

### T — Tampering (Audit Records)

**Threat:** An attacker modifies an audit record or its raw payload in transit or at rest.

**Attack surface:** Wire format (JSON body), raw data store (S3 objects), audit ledger (database rows).

| Sub-threat | Description |
|------------|-------------|
| T-1 | Attacker modifies `raw_payload_hex` in the HTTP request body |
| T-2 | Attacker modifies `AuditRecord.payload_hash` to match a different payload |
| T-3 | Attacker flips bytes in a stored S3 object after accepted ingest |
| T-4 | Attacker modifies `prev_record_hash` to break or redirect the chain |

**Mitigations:**

| ID | Mitigation | Code location |
|----|-----------|---------------|
| M-T-1 | On every ingest the cloud recomputes `BLAKE3(raw_payload)` and compares it to `record.payload_hash`; mismatch → `PayloadHashMismatch` rejection | `ingest/storage.rs` `IngestService::ingest()` |
| M-T-2 | `payload_hash` is covered by the Ed25519 signature; if the hash is changed the signature no longer verifies | `identity.rs` `verify_payload_signature()` |
| M-T-3 | Post-ingest tampering of stored objects is detectable by re-verifying the hash from the ledger against the object content; this is an operational control described in the [Operations Runbook](operations.md) |
| M-T-4 | `prev_record_hash` is validated against the previous accepted record's `hash()`; a break in continuity rejects all subsequent records | `ingest/verify.rs` `check_chain_link()` |

**Residual risk:** Tampering of stored objects after acceptance is a storage-layer concern.  Enabling S3 Object Lock (WORM) or database row-level checksums at the deployment layer eliminates this residual.

---

### R — Repudiation (Operation Logs)

**Threat:** A device or operator denies that a specific ingest event occurred, or claims a record was never sent / was rejected without evidence.

**Attack surface:** `OperationLog` entries written during ingest; audit ledger append operations.

| Sub-threat | Description |
|------------|-------------|
| R-1 | Device claims a record was never submitted |
| R-2 | Operator claims a record was rejected when it was accepted (or vice versa) |
| R-3 | Operation log entries are deleted or modified after the fact |

**Mitigations:**

| ID | Mitigation | Code location |
|----|-----------|---------------|
| M-R-1 | Every ingest attempt — accepted or rejected — writes an `OperationLogEntry` with `device_id`, `sequence`, `decision`, and `message`; the log is written before the ingest function returns | `ingest/storage.rs` `log_acceptance()` / `log_rejection()` |
| M-R-2 | `IngestDecision::Accepted` / `Rejected` is persisted to the operation log atomically with the decision; the record's signed hash serves as cryptographic proof of submission | `ingest/storage.rs` `OperationLogEntry` |
| M-R-3 | Append-only operation logs (Postgres `INSERT`-only pattern; no `DELETE`/`UPDATE` on log rows) prevent after-the-fact modification | `ingest/storage.rs` `PostgresOperationLog`; enforcement at the DB-user permission level |

**Residual risk:** The library provides the operation log data; protecting that data from privileged insider deletion requires database-level controls (role separation, audit logging at the DB layer).

---

### I — Information Disclosure (Payload Storage)

**Threat:** Sensitive inspection payload data is exposed to an unauthorised party.

**Attack surface:** HTTP request body (`raw_payload_hex`), raw data store (S3), audit ledger, operation log.

| Sub-threat | Description |
|------------|-------------|
| I-1 | Eavesdropping on the HTTP transport channel |
| I-2 | Unauthorised read access to S3 objects or Postgres rows |
| I-3 | Payload bytes appear in error messages or logs |

**Mitigations:**

| ID | Mitigation | Code location |
|----|-----------|---------------|
| M-I-1 | The HTTP transport is designed to run behind TLS termination (load balancer / Nginx / Cloudflare); raw payload is hex-encoded in the JSON body and must be carried over HTTPS | `transport/http.rs` — TLS is a deployment-layer control; noted in [Operations Runbook](operations.md) |
| M-I-2 | Raw payloads are stored by `object_ref` under the caller-specified key; access control is enforced by the storage layer (S3 bucket policy, Postgres GRANT); the library does not expose read APIs to unauthenticated callers | `ingest/storage.rs` `RawDataStore::put()` |
| M-I-3 | Error messages include `device_id` and `sequence` but never the raw payload bytes; `tracing` spans log `payload_bytes` length only | `ingest/storage.rs` `#[instrument(skip(raw_payload))]` |

**Residual risk:** Encryption at rest for S3 objects and Postgres rows is a deployment-layer control (S3 SSE-KMS, Postgres `pgcrypto` or TDE).  TLS 1.3 for the ingest HTTP endpoint is addressed in the [Roadmap](../audit/roadmap.md) (issue #73).

---

### D — Denial of Service (Network Policy)

**Threat:** An attacker floods the ingest endpoint to exhaust resources and prevent legitimate devices from submitting records.

**Attack surface:** `POST /api/v1/ingest` HTTP endpoint; `NetworkPolicy` check; `AsyncIngestService` tokio task pool.

| Sub-threat | Description |
|------------|-------------|
| D-1 | High-volume requests from untrusted IPs overwhelm the handler |
| D-2 | Large `raw_payload_hex` values exhaust memory |
| D-3 | Malformed JSON bodies consume parse time |

**Mitigations:**

| ID | Mitigation | Code location |
|----|-----------|---------------|
| M-D-1 | `NetworkPolicy` deny-by-default: all IPs and CIDR ranges are blocked unless explicitly allowlisted; unapproved source IPs receive `403 Forbidden` before any cryptographic work is performed | `ingest/network_policy.rs` `NetworkPolicy::check()`; `transport/http.rs` handler |
| M-D-2 | Axum's default request body size limit (2 MB) caps payload size; the `raw_payload_hex` field is bounded by the HTTP body limit | `transport/http.rs` — axum default body limit |
| M-D-3 | JSON deserialization errors return `400 Bad Request` immediately; no downstream processing occurs | `transport/http.rs` — axum `Json` extractor |

**Residual risk:** Rate limiting per source IP and per device is not yet implemented in the library layer; it should be added at the reverse proxy or API gateway layer in production deployments.  Issue #73 (TLS, P2) is the planned follow-up milestone.

---

### E — Elevation of Privilege (Ingest Gate)

**Threat:** An attacker bypasses the ingest validation gate to write arbitrary records to the ledger or raw data store.

**Attack surface:** `IntegrityPolicyGate`, `ingest_handler`, and the service registration API (`register_device`).

| Sub-threat | Description |
|------------|-------------|
| E-1 | Attacker calls `ingest` with a record for an unregistered device and succeeds |
| E-2 | Attacker submits a record with a valid sequence/chain for a device they do not control |
| E-3 | Attacker registers a malicious device by calling `register_device` directly |

**Mitigations:**

| ID | Mitigation | Code location |
|----|-----------|---------------|
| M-E-1 | `IntegrityPolicyGate::enforce()` is called unconditionally before any storage write; unknown devices fail with `IngestError::UnknownDevice` | `ingest/policy.rs` |
| M-E-2 | Signature verification uses the registered public key for `device_id`; a valid chain cannot be forged without the device's private key | `identity.rs` `verify_payload_signature()` |
| M-E-3 | `register_device` is a privileged operation called only by the application layer at startup; the HTTP ingest handler does not expose device registration over the network | `transport/http.rs` — no registration endpoint; `ingest/storage.rs` `AsyncIngestService::register_device()` |

**Residual risk:** If the application layer that calls `register_device` is compromised, arbitrary devices can be registered.  This is an operational security control: registration should be gated behind a separate privileged API with strong authentication.

---

## Binary Analysis Evidence

### `cargo audit` — Advisory Database Scan

Command and output captured at document generation time (advisory database commit: current):

```
cargo audit
```

**Result:** All detected advisories are pre-approved in `deny.toml` (see table below):

| Advisory | Crate | Version | Status | Reason |
|----------|-------|---------|--------|--------|
| RUSTSEC-2026-0049 | `rustls-webpki` | 0.101.7 | Ignored ([#125](https://github.com/edgesentry/edgesentry-rs/issues/125)) | Pinned by `aws-smithy-http-client` legacy `hyper-rustls 0.24` → `rustls 0.21` chain; no 0.101.x patch exists. The `0.103.x` instance in the tree is updated to 0.103.10. |
| RUSTSEC-2026-0049 | `rustls-webpki` | 0.102.8 | Ignored ([#166](https://github.com/edgesentry/edgesentry-rs/issues/166)) | Pinned by `rumqttc 0.25` → `rustls 0.22` chain; fix requires rumqttc to adopt rustls 0.23+. No CRL revocation calls in the codebase; unexploitable as-is. |

All remaining scanned crate dependencies: **no known CVEs**.

To reproduce:

```bash
cargo install cargo-audit --locked
cargo audit
```

### `cargo deny check` — Policy Enforcement

Command:

```bash
cargo deny check
```

**Result:** `advisories ok, bans ok, licenses ok, sources ok`

The `deny.toml` policy enforces:
- Advisories: all vulnerabilities denied by default except explicitly ignored entries with documented reasons
- Bans: multiple crate versions warned; wildcard dependencies warned
- Licenses: only MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, Unicode-3.0, CC0-1.0, Zlib permitted; one exception: `cbindgen` (MPL-2.0, build-only header generator — copyleft does not extend to generated artifacts or source)
- Sources: only `crates.io` and trusted git sources

To reproduce:

```bash
cargo install cargo-deny --locked
cargo deny check
```

---

## Threat-to-Mitigation Traceability Summary

| STRIDE Category | Threat ID | Mitigation ID | Source File | Status |
|-----------------|-----------|---------------|-------------|--------|
| Spoofing | S-1 | M-S-1 | `ingest/policy.rs` | ✅ |
| Spoofing | S-2 | M-S-2 | `ingest/verify.rs` | ✅ |
| Spoofing | S-3 | M-S-3 | `identity.rs` | ✅ |
| Tampering | T-1 | M-T-1 | `ingest/storage.rs` | ✅ |
| Tampering | T-2 | M-T-2 | `identity.rs` | ✅ |
| Tampering | T-3 | M-T-3 | Operational control | ⚠️ Deployment |
| Tampering | T-4 | M-T-4 | `ingest/verify.rs` | ✅ |
| Repudiation | R-1 | M-R-1 | `ingest/storage.rs` | ✅ |
| Repudiation | R-2 | M-R-2 | `ingest/storage.rs` | ✅ |
| Repudiation | R-3 | M-R-3 | DB permission layer | ⚠️ Deployment |
| Information Disclosure | I-1 | M-I-1 | Deployment (TLS) | ⚠️ [#73](https://github.com/edgesentry/edgesentry-rs/issues/73) |
| Information Disclosure | I-2 | M-I-2 | Storage access control | ⚠️ Deployment |
| Information Disclosure | I-3 | M-I-3 | `ingest/storage.rs` | ✅ |
| Denial of Service | D-1 | M-D-1 | `ingest/network_policy.rs`, `transport/http.rs` | ✅ |
| Denial of Service | D-2 | M-D-2 | `transport/http.rs` (axum body limit) | ✅ |
| Denial of Service | D-3 | M-D-3 | `transport/http.rs` | ✅ |
| Elevation of Privilege | E-1 | M-E-1 | `ingest/policy.rs` | ✅ |
| Elevation of Privilege | E-2 | M-E-2 | `identity.rs` | ✅ |
| Elevation of Privilege | E-3 | M-E-3 | `transport/http.rs` | ✅ |

**Legend:** ✅ Implemented in library code — ⚠️ Deployment-layer control (outside library scope)
