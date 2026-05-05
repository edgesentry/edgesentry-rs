# SBOM and Vendor Disclosure Checklist

This page satisfies the IMDA IoT Cyber Security Guide lifecycle support evidence requirement for Singapore CLS Level 3 assessment. It covers the SBOM format, generation procedure, and vendor disclosure checklist responses for the five mandatory categories.

---

## Software Bill of Materials (SBOM)

### Format

EdgeSentry-RS publishes SBOMs in [CycloneDX](https://cyclonedx.org/) JSON format (spec version 1.3), generated from `Cargo.lock` at release time using [`cargo-cyclonedx`](https://crates.io/crates/cargo-cyclonedx).

### Published artifacts

Each GitHub Release includes two SBOM files as release assets. Download them from the Releases page:

```
https://github.com/edgesentry/edgesentry-rs/releases/tag/v<version>
```

| File | Scope |
|------|-------|
| `edgesentry-rs-<version>.cdx.json` | `edgesentry-rs` crate and all transitive dependencies |
| `edgesentry-bridge-<version>.cdx.json` | `edgesentry-bridge` C/C++ FFI crate and its dependencies |

For example, for v0.1.2:
- `https://github.com/edgesentry/edgesentry-rs/releases/download/v0.1.2/edgesentry-rs-0.1.2.cdx.json`
- `https://github.com/edgesentry/edgesentry-rs/releases/download/v0.1.2/edgesentry-bridge-0.1.2.cdx.json`

### Generating the SBOM locally

```bash
cargo install cargo-cyclonedx --locked
cargo cyclonedx --format json --all
# Output: crates/edgesentry-rs/edgesentry-rs.cdx.json
#         crates/edgesentry-bridge/edgesentry-bridge.cdx.json
```

### Inspecting dependency counts

Run after generating to see the current component count (changes with every dependency update):

```bash
cargo cyclonedx --format json --all
python3 -c "
import json
for f in ['crates/edgesentry-rs/edgesentry-rs.cdx.json',
          'crates/edgesentry-bridge/edgesentry-bridge.cdx.json']:
    bom = json.load(open(f))
    print(f\"{f}: {len(bom.get('components', []))} components\")
"
```

### Continuous supply-chain monitoring

- **`cargo-audit`** — run on every CI build and PR; checks all dependencies against the [RustSec Advisory Database](https://rustsec.org/)
- **`cargo-deny`** — enforces licence policy and bans on every CI build
- **Dependabot** — weekly automated dependency version update PRs

---

## Vendor Disclosure Checklist

The IMDA IoT Cyber Security Guide requires responses across five categories. The table below documents EdgeSentry-RS's position for each.

### 1. Encryption Support

| Item | Response |
|------|----------|
| Algorithms used | Ed25519 (signing), BLAKE3 (hashing) |
| Key length | Ed25519: 256-bit; BLAKE3 output: 256-bit |
| Random number generation | OS CSPRNG via `rand::OsRng` — no custom RNG |
| Transport encryption | Record-level: Ed25519 signature over payload hash. Native TLS transport is provided: `eds serve-tls --tls-cert / --tls-key` (rustls TLS 1.2/1.3, HTTP) and `eds serve-mqtt --tls-ca-cert` (MQTT over TLS). See [CLS-05 in the Traceability Matrix](compliance-matrix.md). |
| Key storage | Public-key registry in memory (`IntegrityPolicyGate`); private key files managed by the deployer. HSM-backed storage planned: [#54](https://github.com/edgesentry/edgesentry-rs/issues/54) |
| Implementation | `crates/edgesentry-rs/src/identity.rs`, `crates/edgesentry-rs/src/integrity.rs` |

### 2. Identification and Authentication

| Item | Response |
|------|----------|
| Device authentication method | Ed25519 asymmetric key pair: device signs each record with its private key; cloud verifies against the registered public key |
| Credential storage | Private key held exclusively on the device; public key registered on the cloud side via `IntegrityPolicyGate::register_device` |
| Default credentials | None — each device generates a unique keypair via `eds keygen` |
| Brute-force protection | Signature verification is a single constant-time operation; no credential-based login surface exists |
| Route identity enforcement | `cert_identity` parameter in `IngestService::ingest` — mismatch between TLS client certificate identity and `record.device_id` causes immediate rejection |
| Implementation | `crates/edgesentry-rs/src/identity.rs`, `crates/edgesentry-rs/src/ingest/policy.rs` |

### 3. Data Protection

| Item | Response |
|------|----------|
| Data in transit | Every `AuditRecord` carries an Ed25519 signature over its BLAKE3 payload hash — authenticity guaranteed at the record level regardless of transport |
| Data at rest | Raw payloads stored via `RawDataStore` (S3/MinIO); audit records via `AuditLedger` (PostgreSQL). Encryption at rest is the deployer's responsibility (S3 SSE, Postgres column encryption) |
| Personal data | `AuditRecord` contains no personal data fields by design — `object_ref` points to a storage key; the payload body is stored separately |
| Data minimisation | Audit metadata (`payload_hash`, `signature`, `prev_record_hash`) is separated from payload body — cloud stores only the hash chain; raw data stored independently via `object_ref` |
| Implementation | `crates/edgesentry-rs/src/record.rs`, `crates/edgesentry-rs/src/ingest/storage.rs` |

### 4. Network Protection

| Item | Response |
|------|----------|
| Unnecessary ports/services | Library only — no network service is opened by `edgesentry-rs`. Transport is the deployer's responsibility |
| Deny-by-default network policy | `NetworkPolicy` enforces an IP/CIDR allowlist; `check(source_ip)` is called before any cryptographic operation — all unlisted sources are rejected |
| DoS resilience | `NetworkPolicy` gate rejects unlisted sources before any cryptographic processing, limiting the attack surface. Full rate-limiting is a deployer concern |
| Implementation | `crates/edgesentry-rs/src/ingest/network_policy.rs` |
| CLS reference | CLS-06 / ETSI EN 303 645 §5.6 |

### 5. Lifecycle Support

| Item | Response |
|------|----------|
| Vulnerability reporting | GitHub private vulnerability reporting enabled. See [SECURITY.md](https://github.com/edgesentry/edgesentry-rs/blob/main/SECURITY.md) — SLA: acknowledge 3 business days; patch 30 days (critical/high), 90 days (medium/low) |
| SBOM availability | CycloneDX JSON published with every GitHub Release (see above) |
| Dependency advisory scanning | `cargo-audit` on every CI build + PR against RustSec Advisory DB |
| End-of-life policy | `edgesentry-rs` v0.x: current version supported. Security updates are patch releases |
| Software update integrity | `UpdateVerifier` checks BLAKE3 payload hash and Ed25519 publisher signature before any update is applied — see [CLS-03](compliance-matrix.md) |
| Supported versions | See [SECURITY.md](https://github.com/edgesentry/edgesentry-rs/blob/main/SECURITY.md#supported-versions) |
| CLS reference | CLS-02 / ETSI EN 303 645 §5.2 |

---

## Traceability

This document satisfies Milestone 1.4 in the [Roadmap](../audit/roadmap.md). For the full clause-by-clause compliance mapping see the [Compliance Traceability Matrix](compliance-matrix.md).
