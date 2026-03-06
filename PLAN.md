# PLAN — Lift RM&D (Remote Monitoring & Diagnostics) Tamper Evidence

## 1. Scope (Strict)

This document describes a flexible architecture focused **only** on:

- **Tamper prevention / tamper detection for lift-related data** collected from industrial IoT devices.
- **Tamper-evident audit records** that make unauthorized modification, deletion, insertion, replay, or reordering detectable.
- **Network-path / network-handling evidence** (captured at a gateway observation point) so later policies can assess whether data traveled through an unacceptable network.

Out of scope for this plan:

- Predictive maintenance algorithms and data science pipelines.
- Alert routing/notification systems.
- Regulatory process details (trial procedures, certification paperwork).
- Complex cloud architectures (streaming platforms, distributed consensus, blockchain).

## 2. Default Deployment Profile (Recommended Starting Scenario)

This default profile targets a common retrofit scenario:

- Controller/vendor interfaces may be unavailable or risky to integrate.
- The building network may be unavailable or hard to access.
- Safety and responsibility boundaries strongly prefer **read-only** acquisition.

### 2.1 Components

**(A) Device Agent (MCU / RTOS, resource-constrained)**

Responsibilities:

- Collect sensor payloads (e.g., vibration/acceleration, temperature, current clamp).
- Produce **signed, chained, tamper-evident records**:
  - `payload_hash = hash(payload)` (e.g., BLAKE3)
  - `signature = sign(payload_hash)` (e.g., Ed25519)
  - `prev_record_hash` links records into a per-device chain
  - `sequence` strictly increases per device
- Send records and (optionally) raw payloads to a gateway.

Default device → gateway uplink: **BLE (GATT)**  
Alternative uplinks (swap-in): RS‑485, Ethernet, Wi‑Fi.

**(B) Site Gateway (Small Linux, observation point)**

Responsibilities:

- Receive device records/payloads over one or more uplinks.
- Provide **store-and-forward** buffering to handle intermittent connectivity.
- Generate **Gateway Witness** evidence for each accepted device record.
- Sign the witness using a gateway key so network-handling evidence is itself tamper-evident.
- Forward to cloud over a minimal protocol.

Default gateway → cloud transport: **HTTP + mTLS**  
Default backhaul: **LTE/5G** (avoid dependence on building LAN)  
Alternative backhauls (swap-in): Ethernet, Wi‑Fi.

**(C) Minimal Cloud Ingest + Storage**

Responsibilities:

- Verify device signatures and device hash-chain integrity.
- Verify gateway witness signatures.
- Enforce sequence monotonicity and deduplication.
- Persist evidence:
  - Raw payloads and large artifacts → S3-compatible object storage (e.g., MinIO)
  - Audit ledger records + witness records → PostgreSQL
  - Operation decision logs (accept/reject + reasons) → PostgreSQL

## 3. Evidence Data Model

### 3.1 Device Audit Record

A compact signed record that references raw payload stored elsewhere.

Minimum conceptual fields:

- `device_id`
- `sequence`
- `timestamp_ms` (device time; may drift)
- `payload_hash`
- `signature` (over `payload_hash`)
- `prev_record_hash`
- `object_ref` (reference to raw payload in S3-compatible store)

### 3.2 Gateway Witness Record (Network/Handling Evidence)

A signed evidence record produced by the gateway.

Key requirements:

- Must be **cryptographically bound** to a specific device record (by hash/reference).
- Must be **signed by the gateway** (gateway public key known to cloud).
- Must capture enough metadata to support future “untrusted network” definitions.

Suggested fields (example):

- `gateway_id`
- `gateway_sequence` (monotonic per gateway)
- `received_at_ms` (gateway time)
- `device_id`, `device_sequence`
- `device_record_hash` (hash of canonical device record bytes)
- `uplink_type` (BLE / RS‑485 / Ethernet / Wi‑Fi / etc.)
- `uplink_metadata` (optional structured; e.g., RSSI, port, bus id)
- `backhaul_type` (LTE/5G / Ethernet / Wi‑Fi)
- `cloud_endpoint` (logical name)
- `tls_summary` (version, cipher, server cert fingerprint/pin result, success/failure)
- `policy_snapshot_id` (optional reference to gateway policy version)
- `witness_prev_hash` (optional: witness chain for gateway events)
- `witness_signature`

Guidance:

- If “untrusted network” policies are not defined yet, **collect evidence first** and evaluate later.
- Avoid collecting unnecessary sensitive information; keep metadata minimal and justified.

## 4. Verification Rules (Tamper Detection)

### 4.1 Device-side (lightweight)

- Hash the raw payload.
- Sign the payload hash.
- Link records with `prev_record_hash` and increment `sequence`.

### 4.2 Cloud-side enforcement (strict)

Reject ingestion if any check fails:

- Unknown device ID (no registered public key).
- Invalid device signature.
- Duplicate record for `(device_id, sequence)`.
- Invalid sequence progression (non-monotonic or gaps if policy disallows gaps).
- Broken per-device hash-chain (`prev_record_hash` mismatch).
- Payload hash mismatch (raw payload does not match `payload_hash`).
- Missing or invalid gateway witness (in deployments that require it).
- Invalid gateway witness signature or witness not bound to the device record.

## 5. Pluggability / Future Variants (Keep It Flexible)

All acquisition and transport must be implemented as replaceable adapters.

### 5.1 Uplink adapters (Device → Gateway)

- BLE (default)
- RS‑485 / serial
- Ethernet
- Wi‑Fi

The gateway normalizes all uplinks into a single internal representation:

- `DeviceRecord + RawPayload`

### 5.2 Transport adapters (Gateway → Cloud)

- HTTP + mTLS (default minimal cloud)
- MQTT + TLS (optional)

The cloud verification + persistence rules remain stable regardless of transport.

### 5.3 Key management and hardware security (future)

This plan assumes software keys initially. Later improvements may include:

- Hardware-backed key storage on devices/gateways (secure element / TPM).
- Key lifecycle operations (rotation, revocation).
- Remote attestation to counter hardware spoofing or takeover.

## 6. Minimal Operational Guidance (Evidence Integrity)

To keep records tamper-evident in practice:

- Treat PostgreSQL ledger tables as **append-only** (no UPDATE/DELETE for normal roles).
- Separate privileges for ingestion, query, and administration.
- Keep periodic signed backups of both PostgreSQL and the S3-compatible object store.

---

## 7. Future Profile (Not Required Now): Near-shore Vessels Using VDES (Ship → Shore Office)

This section is future-looking. It keeps the **same evidence model** (hash + signature + chain + sequence)
but uses a different transport and different network-handling evidence.

Scope remains strict and unchanged:
- tamper prevention/detection for data
- device identity authenticity
- network-handling/path evidence via signed witness records

High-level adaptation:
- Replace “Gateway → Cloud transport adapter” with a **VDES transport adapter**.
- Use store-and-forward and (if required) fragmentation/reassembly while preserving verifiability:
  - deduplicate by `(device_id, sequence)`
  - enforce hash-chain continuity
- Capture VDES-specific handling evidence in signed witness records at observation points (where available):
  - onboard comms gateway (recommended)
  - shore receiver/gateway (recommended)

Witness records SHOULD bind to the device record via `device_record_hash` so that any modification to either side is detectable.

And commit message: Restore detailed elevator PLAN and append future VDES profile.