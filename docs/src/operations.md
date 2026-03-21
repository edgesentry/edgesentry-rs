# Operations Runbook

This page covers observability wiring, alert thresholds, and backup/restore procedures for a production EdgeSentry-RS deployment.

---

## Observability

### Structured logging with `tracing`

EdgeSentry-RS uses the [`tracing`](https://docs.rs/tracing) facade. No subscriber is bundled — deployers wire up the backend of their choice at application startup. The library emits zero overhead when no subscriber is registered.

**Recommended subscriber for production (JSON over stdout, ingested by Loki / CloudWatch):**

```toml
# Cargo.toml of the host application
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
```

```rust
use tracing_subscriber::{fmt, EnvFilter};

fn main() {
    fmt()
        .json()
        .with_env_filter(EnvFilter::from_default_env()) // RUST_LOG=edgesentry_rs=info
        .init();
    // ...
}
```

Set `RUST_LOG=edgesentry_rs=info` for production; `edgesentry_rs=debug` for incident investigation.

### Structured log events emitted by the library

All events include the module path as `target`. Key events:

| Level | Target | Event | Key fields |
|-------|--------|-------|-----------|
| `DEBUG` | `edgesentry_rs::agent` | `signing record` | `device_id`, `sequence`, `payload_bytes` |
| `DEBUG` | `edgesentry_rs::ingest::storage` | `ingest started` | `device_id`, `sequence`, `object_ref`, `payload_bytes` |
| `WARN`  | `edgesentry_rs::ingest::storage` | `payload hash mismatch — record rejected` | `device_id`, `sequence` |
| `WARN`  | `edgesentry_rs::ingest::storage` | `integrity policy rejected record` | `device_id`, `sequence`, `reason` |
| `ERROR` | `edgesentry_rs::ingest::storage` | `raw data store write failed` | `device_id`, `sequence`, `error` |
| `ERROR` | `edgesentry_rs::ingest::storage` | `audit ledger append failed` | `device_id`, `sequence`, `error` |
| `ERROR` | `edgesentry_rs::ingest::storage` | `operation log write failed` | `device_id`, `sequence`, `error` |
| `INFO`  | `edgesentry_rs::ingest::storage` | `record accepted` | `device_id`, `sequence`, `object_ref` |
| `DEBUG` | `edgesentry_rs::ingest::verify` | `signature verification failed` | `device_id`, `sequence` |
| `DEBUG` | `edgesentry_rs::ingest::verify` | `duplicate record rejected` | `device_id`, `sequence` |
| `DEBUG` | `edgesentry_rs::ingest::verify` | `sequence out of order` | `device_id`, `expected`, `actual` |
| `DEBUG` | `edgesentry_rs::ingest::verify` | `prev_record_hash mismatch — chain broken` | `device_id`, `sequence` |
| `DEBUG` | `edgesentry_rs::ingest::verify` | `record verified and accepted` | `device_id`, `sequence` |

### Recommended Prometheus metrics (derived from logs)

Use a log-to-metrics pipeline (e.g. Promtail + Loki, or Vector) to derive counters from structured log events:

| Metric | How to derive | Alert threshold |
|--------|--------------|-----------------|
| `edgesentry_ingest_accepted_total` | Count `INFO "record accepted"` events | — |
| `edgesentry_ingest_rejected_total{reason}` | Count `WARN` rejection events, label by `reason` field | > 10/min sustained → P1 alert |
| `edgesentry_ingest_error_total{component}` | Count `ERROR` storage failure events, label by `component` (raw_data_store / audit_ledger / operation_log) | Any occurrence → P0 alert |
| `edgesentry_chain_break_total` | Count `DEBUG "prev_record_hash mismatch"` events | Any occurrence → P0 alert |
| `edgesentry_signature_fail_total` | Count `DEBUG "signature verification failed"` events | > 5/min sustained → P1 alert |

### OpenTelemetry (tracing spans)

The `IngestService::ingest` method emits a `tracing` span. Wire it to an OTLP exporter for distributed tracing:

```toml
opentelemetry = "0.26"
opentelemetry-otlp = { version = "0.26", features = ["grpc-tonic"] }
tracing-opentelemetry = "0.27"
```

---

## Alert Definitions

| Alert | Condition | Severity | Response |
|-------|-----------|----------|----------|
| `IngestStorageError` | Any `ERROR`-level storage failure | P0 | Check DB/S3 connectivity; verify disk and credentials |
| `ChainBreak` | Any `prev_record_hash mismatch` event | P0 | Investigate tamper or replay; preserve logs before any restart |
| `HighRejectionRate` | Rejection rate > 10/min for 5 min | P1 | Check device firmware; look for misconfigured signing key rotation |
| `SignatureFailureSurge` | Signature failures > 5/min for 5 min | P1 | Possible key compromise or active spoofing attempt |
| `AuditLedgerLag` | Postgres `operation_logs` insert latency > 2 s p99 | P1 | Check DB query plan; autovacuum contention |

---

## Recovery Objectives

| Objective | Target | Basis |
|-----------|--------|-------|
| RTO (recovery time) | < 30 minutes | Time to restore Postgres from pg_basebackup + WAL replay |
| RPO (recovery point) | < 5 minutes | Continuous WAL archiving at 5-minute intervals |

---

## Backup Runbook

### PostgreSQL — audit ledger and operation log

**Prerequisites:** WAL archiving enabled (`archive_mode = on`, `archive_command` shipping to S3 or equivalent).

#### 1. Take a base backup

```bash
pg_basebackup \
  --host=<DB_HOST> \
  --username=<DB_USER> \
  --pgdata=/backup/pg_base_$(date +%Y%m%d_%H%M%S) \
  --format=tar \
  --gzip \
  --wal-method=stream \
  --checkpoint=fast \
  --progress
```

#### 2. Verify the backup

```bash
pg_restore --list /backup/pg_base_<timestamp>/base.tar.gz | head -20
```

#### 3. Archive WAL continuously

Ensure the `archive_command` in `postgresql.conf` ships WAL segments to durable storage (e.g. S3):

```
archive_command = 'aws s3 cp %p s3://<BUCKET>/wal/%f'
```

#### 4. Retention policy

| Backup type | Retention |
|-------------|-----------|
| Base backup | 30 days |
| WAL archive | 30 days |
| Logical dump (`pg_dump`) | 7 days (weekly) |

---

### S3 / MinIO — raw payload store

Enable **versioning** and **cross-region replication** on the bucket:

```bash
# Enable versioning
aws s3api put-bucket-versioning \
  --bucket <BUCKET> \
  --versioning-configuration Status=Enabled

# Enable replication (requires a destination bucket and IAM role configured separately)
aws s3api put-bucket-replication \
  --bucket <BUCKET> \
  --replication-configuration file://replication.json
```

Minimum replication target: one additional region. For CLS Level 3 evidence integrity, ensure object lock or versioning is enabled so payloads cannot be silently overwritten.

---

## Restore Runbook

### PostgreSQL — point-in-time recovery (PITR)

```bash
# 1. Stop the Postgres service
systemctl stop postgresql

# 2. Restore base backup
tar -xzf /backup/pg_base_<timestamp>/base.tar.gz -C /var/lib/postgresql/data/

# 3. Create recovery config
cat > /var/lib/postgresql/data/recovery.conf <<EOF
restore_command = 'aws s3 cp s3://<BUCKET>/wal/%f %p'
recovery_target_time = '<TARGET_TIMESTAMP>'
recovery_target_action = 'promote'
EOF

# 4. Start Postgres — it will replay WAL to the target time
systemctl start postgresql

# 5. Verify: query the last accepted sequence per device
psql -U <DB_USER> -d <DB_NAME> \
  -c "SELECT device_id, MAX(sequence) FROM audit_records GROUP BY device_id;"
```

#### Recovery verification checklist

- [ ] Last record sequence per device matches pre-incident snapshot
- [ ] Hash chain continuity verified: `eds verify-chain <exported-records.json>`
- [ ] Operation log shows no unexpected gaps (check timestamps around recovery target)
- [ ] Alert suppression lifted after verification completes

### S3 / MinIO — object restore

```bash
# Restore a specific object version
aws s3api get-object \
  --bucket <BUCKET> \
  --key <OBJECT_KEY> \
  --version-id <VERSION_ID> \
  <OUTPUT_FILE>
```

---

## Failure Drill Schedule

Run the following drills quarterly to verify runbook accuracy:

| Drill | Procedure | Pass criterion |
|-------|-----------|---------------|
| DB failover | Stop primary Postgres; promote replica | Ingest resumes in < 30 min |
| DB restore | PITR to 1 hour ago on staging | Chain continuity verified in < 30 min |
| S3 object recovery | Restore a deleted test object | Object byte-identical to original |
| Alert fire | Inject a bad signature via test harness | P1 alert fires within 2 min |
