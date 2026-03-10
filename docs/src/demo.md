# Interactive Local Demo

Note: unlike the library-only example, this demo **requires** PostgreSQL and MinIO.

## Three-role model

EdgeSentry-RS is designed around three distinct roles. Understanding which role each step belongs to is key to reading the demo output correctly.

| Role | Responsibility | In this demo |
|------|---------------|-------------|
| **Edge device** | Signs inspection records with an Ed25519 private key and emits them toward the cloud | `examples/edge_device.rs` |
| **Edge gateway** | Forwards signed records from the device to the cloud over HTTPS / MQTT; does not verify content | `examples/edge_gateway.rs` — HTTP transport is out of scope; files on disk simulate the transport |
| **Cloud backend** | Enforces `NetworkPolicy` (CLS-06), runs `IntegrityPolicyGate` (route identity → signature → sequence → hash-chain), and persists accepted records | `examples/cloud_backend.rs` with `--features s3,postgres` |

## What this demo does

The script starts Docker services and then runs the three role examples in sequence:

| Step | Role | What happens |
|------|------|-------------|
| 1–3 | Infrastructure | Start PostgreSQL + MinIO via Docker Compose; wait for health checks |
| 4 | Edge device | `edge_device` — sign 3 records, write `/tmp/eds_*.json` |
| 5 | Edge gateway | `edge_gateway` — read device output, forward unchanged to `/tmp/eds_fwd_*.json` |
| 6 | Cloud backend | `cloud_backend` — `NetworkPolicy` check → `IngestService` → PostgreSQL + MinIO; also shows tamper rejection |
| 7 | Cloud backend | Query persisted audit records and operation log from PostgreSQL |
| 8 | Infrastructure | Stop Docker services |

Prerequisites:

- Docker / Docker Compose
- Rust toolchain (`cargo`)

Run end-to-end demo:

```bash
bash scripts/local_demo.sh
```

The script pauses after each step and waits for Enter (or `OK`) before proceeding.
At the end of the flow, it runs a shutdown step (`docker compose -f docker-compose.local.yml down`).

## Running individual role examples

Each example can also be run standalone without Docker (using in-memory storage for the cloud backend):

```bash
# Step 1: edge device signs records
cargo run -p edgesentry-rs --example edge_device

# Step 2: edge gateway forwards records
cargo run -p edgesentry-rs --example edge_gateway

# Step 3a: cloud backend (in-memory — no Docker required)
cargo run -p edgesentry-rs --example cloud_backend

# Step 3b: cloud backend (PostgreSQL + MinIO — requires Docker)
cargo run -p edgesentry-rs --features s3,postgres --example cloud_backend
```

Each example reads the output files of the previous one from `/tmp/`. Run them in order.

## Manual inspection

Connect to PostgreSQL after step 6:

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
