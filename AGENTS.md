# AGENTS Runbook

This file is the canonical runbook for executable procedures.

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
cargo test -p ledger-core
cargo test -p device-agent
cargo test -p ingest-api
cargo test -p audit-cli
```

Run ingest-api with S3-compatible backend feature enabled:

```bash
cargo test -p ingest-api --features s3
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

## CLI Usage

Build and show help:

```bash
cargo run -p audit-cli -- --help
```

Create a signed record and save it to `record1.json`:

```bash
cargo run -p audit-cli -- sign-record \
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
cargo run -p audit-cli -- verify-record \
  --record-file record1.json \
  --public-key-hex 8a88e3dd7409f195fd52db2d3cba5d72ca670bf1d94121bf3748801b40f6f5c0
```

Verify a whole chain from a JSON array file:

```bash
cargo run -p audit-cli -- verify-chain --records-file records.json
```

## Interactive Local Demo (PostgreSQL + MinIO + CLI)

This project includes an interactive local demo that:

- Starts PostgreSQL + MinIO backend services
- Generates and verifies a signed chain with `audit-cli`
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
docker exec -it immutable-trace-postgres psql -U trace -d trace_audit
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
cargo run -p audit-cli -- demo-lift-inspection \
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
cargo run -p audit-cli -- verify-chain --records-file lift_inspection_records.json
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
cargo run -p audit-cli -- verify-chain --records-file lift_inspection_records.json
```

Expected result: command exits with a non-zero code and prints an error such as `chain verification failed: invalid previous hash ...`.

### 3) Create and verify a single signed inspection event

Generate one signed event:

```bash
cargo run -p audit-cli -- sign-record \
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
cargo run -p audit-cli -- verify-record \
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
cargo run -p audit-cli -- verify-record \
  --record-file lift_single_record.json \
  --public-key-hex 8a88e3dd7409f195fd52db2d3cba5d72ca670bf1d94121bf3748801b40f6f5c0
```

Expected output:

```text
INVALID
```
