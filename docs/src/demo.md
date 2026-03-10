# Interactive Local Demo

Note: unlike the library-only example, this demo **requires** PostgreSQL and MinIO.

## Three-role model

EdgeSentry-RS is designed around three distinct roles. Understanding which role each step belongs to is key to reading the demo output correctly.

| Role | Responsibility | In this demo |
|------|---------------|-------------|
| **Edge device** | Signs inspection records with an Ed25519 private key and emits them toward the cloud | Simulated by the `eds` CLI (`demo-lift-inspection`) |
| **Edge gateway** | Forwards signed records from the device to the cloud over HTTPS / MQTT; does not verify content | Not implemented in this demo — in production this is an industrial PC or 5G gateway between the sensor and the cloud |
| **Cloud backend** | Enforces `NetworkPolicy` (CLS-06), runs `IntegrityPolicyGate` (route identity → signature → sequence → hash-chain), and persists accepted records | PostgreSQL (audit ledger) + MinIO (raw payloads), driven by `demo-ingest` |

The demo script labels each step with its role so you can see where the trust boundary is crossed.

## What this demo does:

- Starts PostgreSQL + MinIO backend services
- Generates and verifies a signed chain with `eds`
- Performs tampering and confirms verification failure
- Ingests accepted records through `IngestService` (writes payloads to MinIO, writes metadata to PostgreSQL)
- Demonstrates rejection of tampered records through the same `IngestService`
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
