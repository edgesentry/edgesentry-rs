# Architecture

## Device Side vs Cloud Side

This system assumes a public-infrastructure IoT deployment where field devices (for example, lift inspection devices) send inspection evidence to cloud services.

### Device side (resource-constrained edge)

The device-side responsibility is implemented by `edgesentry_rs::build_signed_record` and related functions.

- Generate inspection event payloads (door check, vibration check, emergency brake check)
- Compute `payload_hash` (BLAKE3)
- Sign the hash using an Ed25519 private key
- Link each event to the previous record hash (`prev_record_hash`) so records form a chain
- Send only compact audit metadata plus object reference (`object_ref`) to keep edge-side cost low

### Cloud side (verification and trust enforcement)

The cloud-side responsibility is implemented by `edgesentry_rs::ingest` and related modules.

- Gate incoming connections to approved IP addresses and CIDR ranges (`NetworkPolicy::check`) — deny-by-default
- Verify that the device is known (`device_id` -> public key)
- Verify signature validity for each incoming record
- Enforce sequence monotonicity and reject duplicates
- Enforce hash-chain continuity (`prev_record_hash` must match previous record hash)
- Reject tampered, replayed, or reordered data before persistence

### Shared trust logic

All hashing and verification rules live in the same `edgesentry-rs` crate, keeping logic identical across edge and cloud usage.

## Resource-Constrained Device Design

The device-side design is intentionally lightweight so it can be adapted to Cortex-M class environments.

- **Small cryptographic footprint:** records store fixed-size hashes (`[u8; 32]`) and signatures (`[u8; 64]`)
- **Minimal compute path:** hash and sign only; no heavy server-side validation logic on device
- **Compact wire format readiness:** record structure is deterministic and serializable (`serde` + `postcard` support in core)
- **Offload heavy work to cloud:** duplicate detection, sequence policy checks, and full-chain verification are cloud concerns
- **Tamper-evident by construction:** a one-byte modification breaks signature checks or chain continuity

## Concrete Design Flow

1. Device creates event payload `D`.
2. Device computes `H = hash(D)` and signs `H` → signature `S`.
3. Device emits `AuditRecord { device_id, sequence, timestamp_ms, payload_hash=H, signature=S, prev_record_hash, object_ref }`.
4. Cloud verifies signature with registered public key.
5. Cloud verifies sequence and previous-hash link.
6. If any check fails, ingest is rejected; otherwise the record is accepted.

In short, the edge signs facts, and the cloud enforces continuity and authenticity.

## Notarization Metadata Schema

For AI inference results to serve as legally admissible evidence (BCA/CONQUAS inspection reports, MPA ship certificates, MLIT near-visual-inspection equivalence), the audit record payload must capture five categories of provenance metadata in addition to the cryptographic chain. This is the target schema for the notarization connector.

| Category | Fields | Purpose |
|---|---|---|
| **Sensor** | `sensor_id`, `calibration_ts`, `firmware_version`, `sampling_rate` | Prove the measuring instrument was calibrated and operating within spec at capture time |
| **AI model** | `model_uuid`, `model_arch`, `weight_sha256`, `prompt_version` | Enable third-party reproduction of the same inference output from the same input (AI Verify Outcome 3.1 / 3.5) |
| **Compute environment** | `device_type`, `os_version`, `dependency_hashes`, `hw_temp_c` | Full runtime reproducibility; hardware temperature flags thermal throttling that could affect inference timing |
| **Context** | `ntp_ts`, `gps_lat_lon` (or indoor position), `input_data_hash` | Bind the record to a specific physical location and moment; `input_data_hash` prevents payload substitution |
| **Inference process** | `confidence_score`, `preprocessing_algo`, `guardrail_actions` | Support human-in-the-loop triage (AI Verify Outcome 4.5); low-confidence records can be routed for manual review |

These fields are stored in the `payload` object alongside the domain-specific detection data. The `payload_hash` in `AuditRecord` covers the entire payload, so any metadata field change invalidates the signature.

**ALCOA+ alignment:** The five categories map directly to the ALCOA+ data integrity framework required for regulatory submissions — Attributable (sensor/model identity), Legible (structured JSON), Contemporaneous (`ntp_ts`), Original (`input_data_hash`), Accurate (`weight_sha256`, `calibration_ts`), plus Complete, Consistent, Enduring, and Available (covered by the WORM storage connector).

## Ingest Service: Sync and Async Paths

`edgesentry-rs` provides two orchestration service types for cloud-side ingest, selectable by feature flag:

| Type | Feature flag | Thread model | Suitable for |
|------|-------------|-------------|--------------|
| `IngestService` | *(always available)* | Blocking / sync | Embedded, CLI tools, embedded runtimes |
| `AsyncIngestService` | `async-ingest` | `async/await` (tokio) | HTTP servers, async pipelines |

### Sync path (`IngestService`)

The synchronous service is the default and requires no additional features.  S3 writes (when `s3` feature is active) are performed by `block_on`-ing inside an embedded `tokio::runtime::Runtime`.  This is appropriate for single-threaded tools and embedded environments.

```rust
let mut svc = IngestService::new(policy, raw_store, ledger, op_log);
svc.register_device("lift-01", verifying_key);
svc.ingest(record, payload, None)?;
```

### Async path (`AsyncIngestService`)

Enable with `features = ["async-ingest"]`.  All storage calls use `.await` so the calling thread is never blocked, enabling high-concurrency pipelines.  The policy gate is wrapped in a `tokio::sync::Mutex` so the service can be shared across tasks via `Arc`.

```rust
let svc = Arc::new(AsyncIngestService::new(policy, raw_store, ledger, op_log));
svc.register_device("lift-01", verifying_key).await;
svc.ingest(record, payload, None).await?;
```

When `s3` and `async-ingest` are both active, `S3CompatibleRawDataStore` implements `AsyncRawDataStore` by calling the AWS SDK future directly — no embedded runtime needed.

### Feature flag summary

| Flag | What it adds |
|------|-------------|
| `async-ingest` | `AsyncRawDataStore`, `AsyncAuditLedger`, `AsyncOperationLogStore` traits; `AsyncIngestService`; in-memory async stores; `tokio` (sync + macros) |
| `s3` | `S3CompatibleRawDataStore` (sync); when combined with `async-ingest`, also implements `AsyncRawDataStore` |
| `postgres` | `PostgresAuditLedger`, `PostgresOperationLog` (sync) |
| `transport-http` | `transport::http::serve()` — axum-based `POST /api/v1/ingest` server; `eds serve` CLI subcommand |
| `transport-mqtt` | `transport::mqtt::serve_mqtt()` — async rumqttc event loop; subscribes to a topic, routes records through `AsyncIngestService`, publishes accept/reject responses |

## Transport Layer

The `transport` module provides network-facing ingest endpoints built on top of `AsyncIngestService`.

### HTTP (`transport-http` feature)

Enable with `features = ["transport-http"]`.  This brings in `axum 0.8` and exposes a single `POST /api/v1/ingest` endpoint.

#### Request / Response

| Field | Type | Description |
|-------|------|-------------|
| `record` | `AuditRecord` (JSON) | The signed audit record from the device |
| `raw_payload_hex` | `String` | Hex-encoded raw payload bytes |

| Status | Meaning |
|--------|---------|
| `202 Accepted` | Record passed all checks and was stored |
| `400 Bad Request` | `raw_payload_hex` is not valid hex |
| `403 Forbidden` | Client IP is not in the `NetworkPolicy` allowlist |
| `422 Unprocessable Entity` | Record failed signature, hash, or chain verification |

#### Usage

```rust
use edgesentry_rs::{
    AsyncIngestService, AsyncInMemoryRawDataStore, AsyncInMemoryAuditLedger,
    AsyncInMemoryOperationLog, IntegrityPolicyGate, NetworkPolicy,
};
use edgesentry_rs::transport::http::serve;

let mut policy = IntegrityPolicyGate::new();
policy.register_device("lift-01", verifying_key);

let mut network_policy = NetworkPolicy::new();
network_policy.allow_cidr("10.0.0.0/8").unwrap();

let service = AsyncIngestService::new(
    policy,
    AsyncInMemoryRawDataStore::default(),
    AsyncInMemoryAuditLedger::default(),
    AsyncInMemoryOperationLog::default(),
);

let addr = "0.0.0.0:8080".parse().unwrap();
serve(service, network_policy, addr).await?;
```

#### CLI

```sh
eds serve \
  --addr 0.0.0.0:8080 \
  --allowed-sources 10.0.0.0/8,127.0.0.1 \
  --device lift-01=<pubkey_hex>
```

### MQTT (`transport-mqtt` feature)

Enable with `features = ["transport-mqtt"]`.  This brings in `rumqttc` and exposes `serve_mqtt()` — a fully async event loop that connects to an MQTT broker, subscribes to a configurable ingest topic, and routes every incoming message through `AsyncIngestService`.

The message format is the same JSON envelope used by the HTTP transport:

```json
{ "record": { "device_id": "...", "sequence": 1, ... }, "raw_payload_hex": "deadbeef..." }
```

Accept / reject outcomes are published on `<topic>/response`:

```json
{ "device_id": "...", "sequence": 1, "status": "accepted" }
{ "device_id": "...", "sequence": 1, "status": "rejected", "error": "..." }
```

#### Usage

```rust
use edgesentry_rs::transport::mqtt::{MqttIngestConfig, serve_mqtt};
use edgesentry_rs::{
    AsyncIngestService, AsyncInMemoryRawDataStore, AsyncInMemoryAuditLedger,
    AsyncInMemoryOperationLog, IntegrityPolicyGate,
};

let service = AsyncIngestService::new(
    IntegrityPolicyGate::new(),
    AsyncInMemoryRawDataStore::default(),
    AsyncInMemoryAuditLedger::default(),
    AsyncInMemoryOperationLog::default(),
);

let config = MqttIngestConfig::new("mqtt.example.com", "devices/+/ingest", "edgesentry-cloud");
serve_mqtt(config, service).await?;
```

`serve_mqtt` runs until the broker connection is lost, returning `MqttServeError::EventLoop`.  Wrap the call in a retry loop for automatic reconnection.

#### Key behaviors

| Behavior | Detail |
|-----------|--------|
| Malformed JSON | Message is logged and discarded; event loop continues |
| Invalid hex payload | Message is logged and discarded; event loop continues |
| Ingest rejection | Response published on `<topic>/response` with `"status": "rejected"` |
| Response publish failure | Logged as a warning; does not stop the event loop |
