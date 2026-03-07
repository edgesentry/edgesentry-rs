# AGENTS Runbook

This file is the canonical runbook for executable procedures.

## Pull Request Conventions

When creating a pull request, always assign it to the user who authored the branch:

```bash
gh pr create --assignee "@me" --title "..." --body "..."
```

## Mandatory: Run Tests After Every Code Change

After **every** code change, run:

```bash
cargo test --workspace
```

Do not consider a change complete until all tests pass.

## Unit Tests

### Prerequisites (macOS)

Install Rust toolchain first:

```bash
brew install rustup-init
rustup-init -y
source "$HOME/.cargo/env"
rustup default stable
```

Install `cargo-deny` (required for OSS license checks):

```bash
cargo install cargo-deny
source "$HOME/.cargo/env"
cargo deny --version
```

Run all unit tests:

```bash
cargo test --workspace
```

Run tests for a specific crate:

```bash
cargo test -p edgesentry-rs
```

Run edgesentry-rs with S3-compatible backend feature enabled:

```bash
cargo test -p edgesentry-rs --features s3
```

Run unit tests + OSS license checks in one command:

```bash
./scripts/run_unit_and_license_check.sh
```

## Static Analysis and OSS License Check

Use the following checks before release.

### 1) Static analysis (`clippy`)

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

### 2) Dependency security advisory check (`cargo-audit`)

Install once:

```bash
cargo install cargo-audit
```

Run:

```bash
cargo audit
```

### 3) Commercial-use OSS license check (`cargo-deny`)

Install once:

```bash
cargo install cargo-deny
```

Run license check (policy in `deny.toml`):

```bash
cargo deny check licenses
```

Optional full dependency policy check:

```bash
cargo deny check advisories bans licenses sources
```

If this check fails, inspect violating crates and update dependencies or the policy only after legal/security review.

## Build and Release (Rust crates)

### Build release artifacts

```bash
cargo build --workspace --release
```

Build a specific crate only:

```bash
cargo build -p edgesentry-rs --release
```

### Publish to crates.io

1) Validate quality gates first:

```bash
./scripts/run_unit_and_license_check.sh
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

2) Login once:

```bash
cargo login <CRATES_IO_TOKEN>
```

3) Dry-run publish:

```bash
cargo publish --dry-run -p edgesentry-rs
```

4) Publish:

```bash
cargo publish -p edgesentry-rs
```

### GitHub Actions release automation (macOS / Windows / Linux)

This repository includes `.github/workflows/release.yml`.

- Trigger: push a tag like `v0.1.0`
- Quality gate: build, unit tests, license check, clippy
- Publish `edgesentry-rs` to crates.io
- Build `eds` binaries for Linux, macOS (x64 + arm64), and Windows
- Upload packaged binaries to GitHub Release assets

Note: `.github/workflows/ci.yml` runs `cargo publish --dry-run` for `edgesentry-rs`.

Required GitHub secret:

- `CRATES_IO_TOKEN`: crates.io API token used by `cargo publish`

### Automatic version increment after merge

This repository also includes `.github/workflows/auto-version-tag.yml`.

- Trigger: when `CI` succeeds on `main`
- Action: update `workspace.package.version` in `Cargo.toml` and create/push a `vX.Y.Z` tag
- Then: `release.yml` is triggered by that tag and performs the full release pipeline

Version bump rules (Conventional Commits):

- `fix:` -> patch bump (`x.y.z` -> `x.y.(z+1)`)
- `feat:` -> minor bump (`x.y.z` -> `x.(y+1).0`)
- `!` or `BREAKING CHANGE` -> major bump (`x.y.z` -> `(x+1).0.0`)

## CLI Usage

Build and show help:

```bash
cargo run -p edgesentry-rs -- --help
```

Create a signed record and save it to `record1.json`:

```bash
cargo run -p edgesentry-rs -- sign-record \
  --device-id lift-01 \
  --sequence 1 \
  --timestamp-ms 1700000000000 \
  --payload "door-open" \
  --object-ref "s3://bucket/lift-01/1.bin" \
  --private-key-hex 0101010101010101010101010101010101010101010101010101010101010101 \
  --out record1.json
```

Verify one record signature:

```bash
cargo run -p edgesentry-rs -- verify-record \
  --record-file record1.json \
  --public-key-hex 8a88e3dd7409f195fd52db2d3cba5d72ca670bf1d94121bf3748801b40f6f5c0
```

Verify a whole chain from a JSON array file:

```bash
cargo run -p edgesentry-rs -- verify-chain --records-file records.json
```

## Library Usage Example (without CLI)

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

## Interactive Local Demo (PostgreSQL + MinIO + CLI)

This project includes an interactive local demo that:

Note: unlike the library-only example, this demo **requires** PostgreSQL and MinIO.

- Starts PostgreSQL + MinIO backend services
- Generates and verifies a signed chain with `eds`
- Performs tampering and confirms verification failure
- Persists accepted records into PostgreSQL
- Prints audit records and operation logs from the DB
- Stops PostgreSQL + MinIO in the final step

Prerequisites:

- Docker / Docker Compose
- Rust toolchain (`cargo`)

Run end-to-end demo:

```bash
bash scripts/local_demo.sh
```

The script pauses after each step and waits for Enter (or `OK`) before proceeding.
At the end of the flow, it runs a shutdown step (`docker compose -f docker-compose.local.yml down`).

Manual inspection example:

```bash
docker exec -it edgesentry-rs-postgres psql -U trace -d trace_audit
```

Inside `psql`:

```sql
SELECT id, device_id, sequence, object_ref, ingested_at FROM audit_records ORDER BY sequence;
SELECT id, decision, device_id, sequence, message, created_at FROM operation_logs ORDER BY id;
```

MinIO endpoints:

- API: `http://localhost:9000`
- Console: `http://localhost:9001`
- Default credentials: `minioadmin / minioadmin`
- Bucket created by setup container: `bucket`

Manual stop local backend (only if you abort the script midway):

```bash
docker compose -f docker-compose.local.yml down
```

## Lift Inspection Scenario (CLI End-to-End)

This scenario simulates a remote lift inspection with three checks:

1. Door open/close cycle check
2. Vibration check
3. Emergency brake response check

### 1) Generate a full signed chain for one inspection session

```bash
cargo run -p edgesentry-rs -- demo-lift-inspection \
  --device-id lift-01 \
  --out-file lift_inspection_records.json
```

Expected output:

```text
DEMO_CREATED:lift_inspection_records.json
CHAIN_VALID
```

### 2) Verify chain integrity from file

```bash
cargo run -p edgesentry-rs -- verify-chain --records-file lift_inspection_records.json
```

Expected output:

```text
CHAIN_VALID
```

### 2.1) Tamper with the chain file and confirm detection

Modify the first record hash value in-place:

```bash
python3 - <<'PY'
import json

path = "lift_inspection_records.json"
with open(path, "r", encoding="utf-8") as f:
  records = json.load(f)

records[0]["payload_hash"][0] ^= 0x01

with open(path, "w", encoding="utf-8") as f:
  json.dump(records, f, indent=2)
print("tampered", path)
PY
```

Run chain verification again:

```bash
cargo run -p edgesentry-rs -- verify-chain --records-file lift_inspection_records.json
```

Expected result: command exits with a non-zero code and prints an error such as `chain verification failed: invalid previous hash ...`.

### 3) Create and verify a single signed inspection event

Generate one signed event:

```bash
cargo run -p edgesentry-rs -- sign-record \
  --device-id lift-01 \
  --sequence 1 \
  --timestamp-ms 1700000000000 \
  --payload "scenario=lift-inspection,check=door,status=ok" \
  --object-ref "s3://bucket/lift-01/door-check-1.bin" \
  --private-key-hex 0101010101010101010101010101010101010101010101010101010101010101 \
  --out lift_single_record.json
```

Verify signature:

```bash
cargo run -p edgesentry-rs -- verify-record \
  --record-file lift_single_record.json \
  --public-key-hex 8a88e3dd7409f195fd52db2d3cba5d72ca670bf1d94121bf3748801b40f6f5c0
```

Expected output:

```text
VALID
```

### 3.1) Tamper with a single record signature and confirm rejection

Modify one signature byte:

```bash
python3 - <<'PY'
import json

path = "lift_single_record.json"
with open(path, "r", encoding="utf-8") as f:
  record = json.load(f)

record["signature"][0] ^= 0x01

with open(path, "w", encoding="utf-8") as f:
  json.dump(record, f, indent=2)
print("tampered", path)
PY
```

Verify signature again:

```bash
cargo run -p edgesentry-rs -- verify-record \
  --record-file lift_single_record.json \
  --public-key-hex 8a88e3dd7409f195fd52db2d3cba5d72ca670bf1d94121bf3748801b40f6f5c0
```

Expected output:

```text
INVALID
```
