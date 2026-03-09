# Interactive Local Demo

Note: unlike the library-only example, this demo **requires** PostgreSQL and MinIO.

This demo:

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
