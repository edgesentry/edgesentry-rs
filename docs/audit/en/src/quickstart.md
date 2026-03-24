# Library Usage Example

Run the end-to-end lift inspection example implemented directly with library APIs:

Prerequisites:

- Rust toolchain (`cargo`)
- PostgreSQL / MinIO are **not required** for this example (it uses in-memory stores)

```bash
cargo run -p edgesentry-rs --example lift_inspection_flow
```

Scenario covered by the sample:

1. Register one lift device public key in `IntegrityPolicyGate`
2. Generate three signed inspection records with `build_signed_record`
3. Ingest all records via `IngestService` (accepted path)
4. Tamper one record (`payload_hash`) and confirm rejection
5. Print stored audit records and operation logs

What it demonstrates:

- Record signing with `edgesentry_rs::build_signed_record`
- Ingestion verification with `edgesentry_rs::ingest::IngestService`
- Tampering rejection (modified `payload_hash`)
- Audit records and operation-log output

Source:

- `crates/edgesentry-rs/examples/lift_inspection_flow.rs`

---

## Three-Role Distributed Demo

For a more realistic view of the edge-to-cloud flow, three separate examples can be run in sequence. Each example owns exactly one role:

| Example | Role | External deps |
|---------|------|--------------|
| `edge_device` | Signs records, writes `/tmp/eds_*.json` | None |
| `edge_gateway` | Routes records, no crypto verification | None |
| `cloud_backend` | NetworkPolicy + IngestService + storage | None (in-memory) or PostgreSQL + MinIO (`--features s3,postgres`) |

Run in order:

```bash
cargo run -p edgesentry-rs --example edge_device
cargo run -p edgesentry-rs --example edge_gateway
cargo run -p edgesentry-rs --example cloud_backend
```

Each example reads the output files of the previous one from `/tmp/`. The full sequence with real backends (requires Docker — see [Interactive Demo](demo.md)):

```bash
cargo run -p edgesentry-rs --example edge_device
cargo run -p edgesentry-rs --example edge_gateway
cargo run -p edgesentry-rs --features s3,postgres --example cloud_backend
```

What the sequence demonstrates:

- `edge_device` — device-side signing with `build_signed_record`; tampered copy written for rejection demo
- `edge_gateway` — gateway receives records but does NOT verify signatures (routing-only responsibility)
- `cloud_backend` — `NetworkPolicy::check` runs before every `IngestService::ingest`; accepted and rejected records both visible

Sources:

- `crates/edgesentry-rs/examples/edge_device.rs`
- `crates/edgesentry-rs/examples/edge_gateway.rs`
- `crates/edgesentry-rs/examples/cloud_backend.rs`

---

## S3 / MinIO Switching

`edgesentry-rs` supports a switchable S3-compatible raw-data backend behind the `s3` feature.

- `S3Backend::AwsS3`: use AWS S3 (default AWS credential chain, or optional static key)
- `S3Backend::Minio`: use MinIO (custom endpoint + static access key/secret)

The ingest layer is coded against a common raw-data storage abstraction, while concrete configuration selects AWS S3 or MinIO without changing ingest business logic.

Use these types from `edgesentry_rs`:

- `S3ObjectStoreConfig::for_aws_s3(...)`
- `S3ObjectStoreConfig::for_minio(...)`
- `S3CompatibleRawDataStore::new(config)`

Build and test with the S3 feature enabled:

```bash
cargo test -p edgesentry-rs --features s3
```

To run the S3 integration tests against a live MinIO instance, set the environment variables and run the dedicated test file:

```bash
TEST_S3_ENDPOINT=http://localhost:9000 \
TEST_S3_ACCESS_KEY=minioadmin \
TEST_S3_SECRET_KEY=minioadmin \
TEST_S3_BUCKET=bucket \
cargo test -p edgesentry-rs --features s3 --test integration -- --nocapture
```

Tests skip automatically when any of the four `TEST_S3_*` variables are unset.
