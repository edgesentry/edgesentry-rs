# Compliance Traceability Matrix

This page maps each Singapore CLS / iM8 clause and corresponding ETSI EN 303 645 provision to the source code that satisfies it. Japan JC-STAR cross-references and SS 711:2025 design principle alignment are included for each row.

Legend:

- ✅ Implemented
- ⚠️ Partial
- 🔲 Planned
- ➖ Not in scope

## SS 711:2025 Design Principles Coverage

Singapore's national IoT standard SS 711:2025 defines four principles. See the [Roadmap](../audit/roadmap.md) for the full module mapping.

| Principle | SS 711:2025 Requirement | Status |
|-----------|------------------------|--------|
| Secure by Default | Unique device identity, signed OTA updates | ✅ `identity.rs`, `update.rs` |
| Rigour in Defence | STRIDE threat model, tamper detection | ✅ Hash chain (`integrity.rs`) + [STRIDE threat model](threat-model.md) |
| Accountability | Audit trail, operation logs, RBAC design | ✅ `ingest/` (AuditLedger, OperationLog) |
| Resiliency | Deny-by-default networking, DoS protection | ✅ `ingest/network_policy.rs` |

---

---

## CLS Level 3 / ETSI EN 303 645 — Core Requirements

### CLS-01 / §5.1 — No universal default passwords

| Item | Detail |
|------|--------|
| JC-STAR | STAR-1 R3.1 |
| Requirement | Devices must not use universal default credentials |
| Status | ➖ Out of scope — this project implements software audit records, not device credential management |

---

### CLS-02 / §5.2 — Implement a means to manage reports of vulnerabilities

| Item | Detail |
|------|--------|
| JC-STAR | STAR-1 R4.1 |
| Requirement | A published, actionable vulnerability reporting channel with defined SLAs |
| Status | ✅ Implemented |
| Implementation | [`SECURITY.md`](https://github.com/edgesentry/edgesentry-rs/blob/main/SECURITY.md) — published disclosure policy with supported versions, private reporting via GitHub advisory, acknowledgement SLA (3 business days), patch SLA (30 days critical/high; 90 days medium/low), and defined in/out-of-scope |
| Implementation | GitHub private vulnerability reporting enabled — reporters use the [Security Advisories](https://github.com/edgesentry/edgesentry-rs/security/advisories/new) form |

---

### CLS-03 / §5.3 — Keep software updated

| Item | Detail |
|------|--------|
| JC-STAR | STAR-2 R2.2 |
| Requirement | Software update packages must be signed and verified before installation |
| Status | ✅ Implemented |
| Implementation | `UpdateVerifier::verify` checks BLAKE3 payload hash then Ed25519 publisher signature before allowing installation; failed checks are logged as `UpdateVerifyDecision::Rejected` in `UpdateVerificationLog` ([`src/update.rs`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/update.rs)) |
| Tests | `tests/unit/update_tests.rs` — covers accepted path, tampered payload, invalid signature, unknown publisher, multi-publisher isolation |

---

### CLS-04 / §5.4 — Securely store sensitive security parameters

| Item | Detail |
|------|--------|
| JC-STAR | STAR-1 R1.2 |
| Requirement | Private keys must be stored securely; a key registration process must exist |
| Status | ✅ Implemented |
| Implementation | Public key registry: `IntegrityPolicyGate::register_device` ([`src/ingest/policy.rs:20`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/ingest/policy.rs#L20)) |
| Implementation | Key generation CLI: `eds keygen` ([`src/lib.rs — generate_keypair`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/lib.rs)) |
| Implementation | Key inspection CLI: `eds inspect-key` ([`src/lib.rs — inspect_key`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/lib.rs)) |
| Implementation | Provisioning and rotation guidance: [Key Management](../audit/key_management.md) |
| Note | HSM-backed key storage (CLS Level 4) is planned in [#54](https://github.com/edgesentry/edgesentry-rs/issues/54) |

---

### CLS-05 / §5.5 — Communicate securely

| Item | Detail |
|------|--------|
| JC-STAR | STAR-1 R1.1 |
| Requirement | Data must be transmitted with authenticity guarantees |
| Status | ✅ Implemented |
| Implementation — record authenticity | Every `AuditRecord` carries an Ed25519 signature over its BLAKE3 payload hash — `build_signed_record` ([`src/agent.rs`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/agent.rs)), `sign_payload_hash` ([`src/identity.rs:12`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/identity.rs#L12)) |
| Implementation — channel confidentiality (HTTP) | `transport-tls` feature: `serve_tls()` with rustls TLS 1.2/1.3, IP allowlist enforced before handshake, `eds serve-tls --tls-cert / --tls-key` CLI — closed [#176](https://github.com/edgesentry/edgesentry-rs/issues/176) ([`src/transport/tls.rs`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/transport/tls.rs)) |
| Implementation — channel confidentiality (MQTT) | `transport-mqtt-tls` feature: `MqttTlsConfig` with CA cert path, rustls `ClientConfig` via `rumqttc::TlsConfiguration::Rustls`, `eds serve-mqtt --tls-ca-cert` CLI — closed [#180](https://github.com/edgesentry/edgesentry-rs/issues/180) ([`src/transport/mqtt.rs`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/transport/mqtt.rs)) |

---

### CLS-06 / §5.6 — Minimise exposed attack surfaces

| Item | Detail |
|------|--------|
| JC-STAR | STAR-1 R3.2 |
| Requirement | Only necessary interfaces and services should be exposed |
| Status | ✅ Implemented |
| Implementation — IP allowlist | `NetworkPolicy` provides deny-by-default IP/CIDR allowlist enforcement ([`src/ingest/network_policy.rs`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/ingest/network_policy.rs)) |
| Implementation — HTTP transport | `ingest_handler` enforces `NetworkPolicy::check(source_ip)` before any crypto verification; returns `403 Forbidden` for unlisted sources ([`src/transport/http.rs`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/transport/http.rs)) |
| Implementation — MQTT transport | `serve_mqtt` exposes a single subscribe-only topic; no administrative interface; broker-level ACLs recommended ([`src/transport/mqtt.rs`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/transport/mqtt.rs)) |
| Note | Network-level controls (VPN, firewall rules) remain the deployer's responsibility |

---

### CLS-07 / §5.7 — Ensure software integrity

| Item | Detail |
|------|--------|
| JC-STAR | STAR-1 R1.3 |
| Requirement | The device must verify the integrity of software and data |
| Status | ✅ Implemented |
| Implementation — payload hash | BLAKE3 hash over raw payload: `compute_payload_hash` ([`src/integrity.rs:12`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/integrity.rs#L12)) |
| Implementation — hash chain | `prev_record_hash` links each record to its predecessor; insertion/deletion detected by `verify_chain` ([`src/integrity.rs:35`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/integrity.rs#L35)) |
| Tests | `tampered_lift_demo_chain_is_detected` ([`src/lib.rs:338`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/lib.rs#L338)) |

---

### CLS-08 / §5.8 — Ensure that personal data is secure

| Item | Detail |
|------|--------|
| JC-STAR | STAR-2 R4.1 |
| Requirement | Personal data transmitted or stored must be protected |
| Status | ➖ Out of scope — audit records do not contain personal data in the current implementation |

---

### CLS-09 / §5.9 — Make systems resilient to outages

| Item | Detail |
|------|--------|
| JC-STAR | STAR-2 R3.2 |
| Requirement | The device should remain operational and recover gracefully |
| Status | ⚠️ Partial |
| Implementation | `OfflineBuffer<S>` accumulates signed records during connectivity loss and replays them in insertion order via `flush` when the link recovers. Duplicate records from replay are treated as already-accepted and do not cause failures ([`src/buffer/mod.rs`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/buffer/mod.rs)) |
| Implementation | Pluggable `BufferStore` trait — volatile `InMemoryBufferStore` (default) and durable `SqliteBufferStore` behind the `buffer-sqlite` feature flag |
| Gap | Full HA (active–active failover, network-level redundancy) remains the deployer's responsibility |

---

### CLS-10 / §5.10 — Examine system telemetry data

| Item | Detail |
|------|--------|
| JC-STAR | STAR-2 R3.1 |
| Requirement | Security-relevant events must be logged and replay/reorder attacks must be detected |
| Status | ✅ Implemented |
| Implementation — sequence | Strict monotonic `sequence` per device; duplicates and out-of-order records rejected by `IngestState::verify_and_accept` ([`src/ingest/verify.rs:45`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/ingest/verify.rs#L45)) |
| Implementation — audit trail | Accept/reject decisions persisted via `IngestService` and `AuditLedger` ([`src/ingest/storage.rs`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/ingest/storage.rs)) |

---

### CLS-11 / §5.11 — Make it easy for users to delete user data

| Item | Detail |
|------|--------|
| JC-STAR | — |
| Requirement | Users should be able to delete personal data |
| Status | ➖ Out of scope |

---

## CLS Level 4 — Additional Requirements

### CLS Level 4 — Hardware Security Module (HSM)

| Item | Detail |
|------|--------|
| JC-STAR | STAR-2 R1.4 |
| Requirement | Private keys must be stored and used inside an HSM |
| Status | 🔲 Planned |
| Gap | HSM-backed key storage planned for Phase 3 (IEC 62443-4-2 / CII/OT). See [#54](https://github.com/edgesentry/edgesentry-rs/issues/54) and [#98](https://github.com/edgesentry/edgesentry-rs/issues/98) |

---

## JC-STAR Additional Requirements

### STAR-1 R2.1 — Replay and reorder prevention

| Item | Detail |
|------|--------|
| CLS | CLS-10 |
| Requirement | Replay attacks must be detected and rejected |
| Status | ✅ Implemented |
| Implementation | `seen` HashSet in `IngestState` rejects duplicate `(device_id, sequence)` pairs ([`src/ingest/verify.rs:56`](https://github.com/edgesentry/edgesentry-rs/blob/main/crates/edgesentry-rs/src/ingest/verify.rs#L56)) |

---

## Coverage Summary

| Level | Total clauses | ✅ Implemented | ⚠️ Partial | 🔲 Planned | ➖ Out of scope |
|-------|-------------|--------------|-----------|-----------|----------------|
| CLS Level 3 | 11 | 6 | 2 | 0 | 3 |
| CLS Level 4 | 1 | 0 | 0 | 1 | 0 |
| JC-STAR additions | 1 | 1 | 0 | 0 | 0 |

> **Note:** "Out of scope" clauses cover device-level concerns (passwords, network interfaces, personal data) that are the responsibility of the deployer, not the audit-record library.
