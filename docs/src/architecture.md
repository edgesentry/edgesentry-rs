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
| `transport-mqtt` | `transport::mqtt::MqttIngestConfig` scaffold (full implementation pending) |

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

The `transport-mqtt` feature currently provides `MqttIngestConfig` — a configuration struct describing the broker, topic, client ID, and QoS level.  Full protocol implementation is planned in a follow-up issue.

```rust
use edgesentry_rs::transport::mqtt::{MqttIngestConfig, MqttQos};

let config = MqttIngestConfig {
    broker_host: "mqtt.example.com".into(),
    broker_port: 1883,
    topic: "devices/+/ingest".into(),
    client_id: "edgesentry-cloud".into(),
    qos: MqttQos::AtLeastOnce,
};
```
