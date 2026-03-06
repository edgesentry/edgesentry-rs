# immutable-trace

This repository contains a tamper-evident audit log PoC built in Rust from IoT devices to cloud services.

## Motivation

In recent years, labor shortages have become a serious challenge in infrastructure operations. Labor-intensive industries such as construction are increasingly adopting IoT devices for remote inspections.

At the same time, if device spoofing, device takeover, or inspection data tampering occurs, trust in the entire system is fundamentally undermined. This makes continuous verification of both device authenticity and data integrity essential.

For public-infrastructure IoT deployments, Singapore's Cybersecurity Labelling Scheme (CLS) may require Level 3 or Level 4, which introduces hardware-level security requirements.

https://www.csa.gov.sg/our-programmes/certification-and-labelling-schemes/cybersecurity-labelling-scheme/about/

Because those hardware-dependent setups are often difficult to evaluate quickly in an early PoC phase, this repository focuses on sample code for tamper prevention and tamper-evident audit records.

## Crates

- `ledger-core`: Audit record types, hashing, signature verification, and chain verification
- `device-agent`: Device-side signed record generation
- `ingest-api`: Ingestion-time verification, deduplication, sequence validation, and persistence workflow for raw data / audit ledger / operation logs
- `audit-cli`: Command-line operations for signing and verifying audit records (`immutable-trace` package, `imt` binary)

## Concepts

For a glossary-style explanation of the core concepts in this repository, see [CONCEPTS.md](CONCEPTS.md).

## Device Side vs Cloud Side

This PoC assumes a public-infrastructure IoT deployment where field devices (for example, lift inspection devices) send inspection evidence to cloud services.

### Device side (resource-constrained edge)

The device-side responsibility is implemented by `device-agent` + `ledger-core`.

- Generate inspection event payloads (door check, vibration check, emergency brake check)
- Compute `payload_hash` (BLAKE3)
- Sign the hash using an Ed25519 private key
- Link each event to the previous record hash (`prev_record_hash`) so records form a chain
- Send only compact audit metadata plus object reference (`object_ref`) to keep edge-side cost low

### Cloud side (verification and trust enforcement)

The cloud-side responsibility is implemented by `ingest-api` + `ledger-core`.

- Verify that the device is known (`device_id` -> public key)
- Verify signature validity for each incoming record
- Enforce sequence monotonicity and reject duplicates
- Enforce hash-chain continuity (`prev_record_hash` must match previous record hash)
- Reject tampered, replayed, or reordered data before persistence

### Shared trust logic

`ledger-core` is shared by both sides to keep hashing and verification rules identical across edge and cloud boundaries.

Using Rust on both the device and cloud sides also enables a more unified implementation model. Core types, validation logic, and test cases can be reused across environments with minimal translation overhead. This reduces drift between edge and cloud behavior, lowers maintenance cost, and makes security-critical logic easier to review and verify end-to-end.

## Resource-Constrained Device Design

The device-side design is intentionally lightweight so it can be adapted to Cortex-M class environments.

- **Small cryptographic footprint:** records store fixed-size hashes (`[u8; 32]`) and signatures (`[u8; 64]`)
- **Minimal compute path:** hash and sign only; no heavy server-side validation logic on device
- **Compact wire format readiness:** record structure is deterministic and serializable (`serde` + `postcard` support in core)
- **Offload heavy work to cloud:** duplicate detection, sequence policy checks, and full-chain verification are cloud concerns
- **Tamper-evident by construction:** a one-byte modification breaks signature checks or chain continuity

## Concrete Design Flow

1. Device creates event payload `D`.
2. Device computes `H = hash(D)` and signs `H` -> signature `S`.
3. Device emits `AuditRecord { device_id, sequence, timestamp_ms, payload_hash=H, signature=S, prev_record_hash, object_ref }`.
4. Cloud verifies signature with registered public key.
5. Cloud verifies sequence and previous-hash link.
6. If any check fails, ingest is rejected; otherwise the record is accepted.

In short, the edge signs facts, and the cloud enforces continuity and authenticity.

## Operations

All execution procedures are centralized in [AGENTS.md](AGENTS.md):

- Unit test commands
- macOS prerequisites
- CLI usage
- Lift inspection end-to-end scenario
- Tampering and detection walkthrough

## Quality and License Check

Run workspace unit tests and commercial-use OSS license checks in one command:

```bash
./scripts/run_unit_and_license_check.sh
```

This script runs:

1. `cargo test --workspace`
2. `cargo test -p ingest-api --features s3`
3. `cargo deny check licenses` (policy from `deny.toml`)

## Interactive Local Demo

This repository includes an interactive end-to-end demo script that validates the tamper-evident workflow locally:

Note: unlike the library-only example, this demo **requires** PostgreSQL and MinIO.

1. Start backend services (PostgreSQL + MinIO)
2. Generate and verify a signed chain with `imt`
3. Tamper with a generated chain and confirm verification fails
4. Persist accepted records into PostgreSQL
5. Display audit records and operation logs from the database
6. Stop PostgreSQL + MinIO at the end of the flow

Run:

```bash
bash scripts/local_demo.sh
```

The script pauses at each step and waits for Enter (or `OK`) so results can be inspected interactively.
For full command details and manual inspection steps, see [AGENTS.md](AGENTS.md).

## Library Usage Example (Lift Inspection Scenario)

If you want to integrate the libraries directly (without using `imt`), run the example below.

Prerequisites:

- Rust toolchain (`cargo`)
- PostgreSQL / MinIO are **not required** for this example (it uses in-memory stores)

Run:

```bash
cargo run -p ingest-api --example lift_inspection_flow
```

Source:

- [crates/ingest-api/examples/lift_inspection_flow.rs](crates/ingest-api/examples/lift_inspection_flow.rs)

For the full scenario steps and expected behavior, see [AGENTS.md](AGENTS.md).

## S3 / MinIO Switching

`ingest-api` supports a switchable S3-compatible raw-data backend behind the `s3` feature.

- `S3Backend::AwsS3`: use AWS S3 (default AWS credential chain, or optional static key)
- `S3Backend::Minio`: use MinIO (custom endpoint + static access key/secret)

This is an S3-compatible object-storage design. The ingest layer is coded against a common raw-data storage abstraction, while concrete configuration selects AWS S3 or MinIO without changing ingest business logic.

Use these types from `ingest-api`:

- `S3ObjectStoreConfig::for_aws_s3(...)`
- `S3ObjectStoreConfig::for_minio(...)`
- `S3CompatibleRawDataStore::new(config)`

For build and test commands, see [AGENTS.md](AGENTS.md).

## License

This project is licensed under either of:

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT license](LICENSE-MIT)

at your option.
