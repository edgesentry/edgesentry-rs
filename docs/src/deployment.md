# Production Deployment Guide

This guide covers moving from the local Docker Compose demo to a production-grade deployment of `eds serve` (HTTP/TLS) and `eds serve-mqtt`. For the local quickstart, see [Interactive Demo](demo.md). For observability, alerting, and backup/restore procedures, see [Operations Runbook](operations.md).

---

## Prerequisites

| Component | Minimum version | Notes |
|-----------|----------------|-------|
| edgesentry-rs binary | current `main` | Built with `--features transport-http,transport-tls` for HTTPS; add `transport-mqtt` for MQTT |
| PostgreSQL | 14 | Audit ledger and operation log |
| S3-compatible store | — | AWS S3, MinIO ≥ RELEASE.2023, or Cloudflare R2 |
| (Optional) MQTT broker | Mosquitto ≥ 2.0 | Required only for `eds serve-mqtt` |

---

## 1 — TLS Certificate Management

### 1.1 Provisioning with Let's Encrypt (recommended)

```bash
# Install certbot
apt install certbot

# Issue a certificate for the ingest endpoint
certbot certonly --standalone \
  -d ingest.example.com \
  --agree-tos --non-interactive \
  -m ops@example.com

# Certificates are written to:
#   /etc/letsencrypt/live/ingest.example.com/fullchain.pem  (cert + chain)
#   /etc/letsencrypt/live/ingest.example.com/privkey.pem    (private key)
```

### 1.2 Starting `eds serve` with TLS

```bash
eds serve \
  --addr 0.0.0.0:8443 \
  --tls-cert /etc/letsencrypt/live/ingest.example.com/fullchain.pem \
  --tls-key  /etc/letsencrypt/live/ingest.example.com/privkey.pem \
  --allowed-sources 10.0.0.0/8 \
  --device lift-01=<PUBLIC_KEY_HEX>
```

`eds serve` enforces TLS 1.2 minimum and TLS 1.3 preferred via rustls. No extra configuration is needed.

### 1.3 Certificate rotation (zero-downtime)

`eds serve` reads the certificate files at startup only. For rotation without downtime:

```bash
# 1. Renew the certificate
certbot renew --quiet

# 2. Send SIGTERM to the running process (systemd handles restart)
systemctl reload edgesentry
# — or, without systemd —
kill -TERM $(pidof eds)
# Process exits cleanly; supervisor / systemd restarts it and picks up the new cert
```

Add a cron/systemd timer to automate renewal:

```ini
# /etc/systemd/system/certbot.timer
[Timer]
OnCalendar=weekly
Persistent=true

[Install]
WantedBy=timers.target
```

```bash
systemctl enable --now certbot.timer
```

### 1.4 Self-signed certificates (internal / air-gapped deployments)

```bash
# Generate a 10-year self-signed certificate
openssl req -x509 -newkey rsa:4096 -sha256 -days 3650 \
  -nodes -keyout server.key -out server.crt \
  -subj "/CN=ingest.internal" \
  -addext "subjectAltName=IP:10.0.1.5,DNS:ingest.internal"
```

Distribute `server.crt` to all edge devices as the trusted CA.

---

## 2 — PostgreSQL: Schema, Indexes, and Connection Sizing

### 2.1 Schema migration

The schema is in [`db/init/001_schema.sql`](https://github.com/edgesentry/edgesentry-rs/blob/main/db/init/001_schema.sql). Apply it against your production database:

```bash
psql "$DATABASE_URL" -f db/init/001_schema.sql
```

The schema is idempotent (`CREATE TABLE IF NOT EXISTS`) and safe to re-run.

### 2.2 Recommended indexes

The base schema ships with a `UNIQUE (device_id, sequence)` constraint which doubles as a B-tree index and rejects replay attacks at the database level. Add the following indexes for common query patterns:

```sql
-- Fast lookup of the latest record per device (chain-head queries)
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_audit_device_seq
    ON audit_records (device_id, sequence DESC);

-- Time-range queries for compliance reporting
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_audit_ingested_at
    ON audit_records (ingested_at);

-- Operation log filtering by decision type
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_oplog_decision_device
    ON operation_logs (decision, device_id, created_at DESC);
```

`CONCURRENTLY` means these can be created without locking the table in production.

### 2.3 Connection pool sizing

`PostgresAuditLedger` and `PostgresOperationLog` each open one synchronous connection via the `postgres` crate. For multi-node deployments (see §5) each `eds` process holds two connections. Set `max_connections` in `postgresql.conf` to accommodate:

```
max_connections = 2 × <number of eds instances> + 10   # headroom for psql, monitoring
```

For high ingest rates (> 500 records/s), replace the sync backends with an async connection pool (e.g. `sqlx` + `PgPool`) as a custom `AsyncAuditLedger` implementation.

### 2.4 Partitioning for long-term retention

Partition `audit_records` by `ingested_at` when the table is expected to exceed 100 M rows:

```sql
-- Convert to range-partitioned table (run once, before data accumulates)
CREATE TABLE audit_records_new (LIKE audit_records INCLUDING ALL)
    PARTITION BY RANGE (ingested_at);

CREATE TABLE audit_records_2026_q1
    PARTITION OF audit_records_new
    FOR VALUES FROM ('2026-01-01') TO ('2026-04-01');

-- Attach, swap, drop
ALTER TABLE audit_records RENAME TO audit_records_old;
ALTER TABLE audit_records_new RENAME TO audit_records;
DROP TABLE audit_records_old;
```

---

## 3 — Object Storage: Bucket Policy and Lifecycle Rules

### 3.1 AWS S3 — bucket policy (least privilege)

Create a dedicated IAM role for the ingest service with write-only access:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "IngestWriteOnly",
      "Effect": "Allow",
      "Action": ["s3:PutObject"],
      "Resource": "arn:aws:s3:::edgesentry-audit/*"
    },
    {
      "Sid": "ListBucket",
      "Effect": "Allow",
      "Action": ["s3:ListBucket"],
      "Resource": "arn:aws:s3:::edgesentry-audit"
    }
  ]
}
```

Attach a separate read-only role to compliance auditors.

### 3.2 Lifecycle rules (retention + cost management)

```json
{
  "Rules": [
    {
      "Id": "TransitionToIA",
      "Status": "Enabled",
      "Filter": { "Prefix": "" },
      "Transitions": [
        { "Days": 90,  "StorageClass": "STANDARD_IA" },
        { "Days": 365, "StorageClass": "GLACIER_IR" }
      ]
    },
    {
      "Id": "ExpireOldObjects",
      "Status": "Enabled",
      "Filter": { "Prefix": "" },
      "Expiration": { "Days": 2555 }
    }
  ]
}
```

Apply via CLI:

```bash
aws s3api put-bucket-lifecycle-configuration \
  --bucket edgesentry-audit \
  --lifecycle-configuration file://lifecycle.json
```

### 3.3 MinIO (on-premises)

```bash
# Create bucket with object locking (immutability for compliance)
mc mb --with-lock minio/edgesentry-audit

# Set lifecycle: transition to cheaper tier after 90 days
mc ilm import minio/edgesentry-audit <<EOF
{
  "Rules": [{
    "ID": "expire-3-years",
    "Status": "Enabled",
    "Expiration": { "Days": 1095 }
  }]
}
EOF

# Server-side encryption at rest
mc encrypt set sse-s3 minio/edgesentry-audit
```

---

## 4 — Process Management

### 4.1 systemd service unit (HTTP + TLS)

```ini
# /etc/systemd/system/edgesentry.service
[Unit]
Description=EdgeSentry-RS ingest server
After=network-online.target postgresql.service
Wants=network-online.target

[Service]
Type=exec
User=edgesentry
Group=edgesentry
ExecStart=/usr/local/bin/eds serve \
    --addr 0.0.0.0:8443 \
    --tls-cert /etc/edgesentry/server.crt \
    --tls-key  /etc/edgesentry/server.key \
    --allowed-sources 10.0.0.0/8 \
    --device lift-01=<PUBLIC_KEY_HEX>
Restart=on-failure
RestartSec=5
Environment=RUST_LOG=edgesentry_rs=info

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/log/edgesentry
PrivateTmp=true
CapabilityBoundingSet=

[Install]
WantedBy=multi-user.target
```

```bash
# Install and start
install -m 755 target/release/eds /usr/local/bin/eds
useradd --system --no-create-home edgesentry
mkdir -p /var/log/edgesentry && chown edgesentry:edgesentry /var/log/edgesentry

systemctl daemon-reload
systemctl enable --now edgesentry
systemctl status edgesentry
```

### 4.2 systemd service unit (MQTT)

```ini
# /etc/systemd/system/edgesentry-mqtt.service
[Unit]
Description=EdgeSentry-RS MQTT ingest subscriber
After=network-online.target mosquitto.service
Wants=network-online.target

[Service]
Type=exec
User=edgesentry
Group=edgesentry
ExecStart=/usr/local/bin/eds serve-mqtt \
    --broker 10.0.1.10 \
    --port 1883 \
    --topic edgesentry/ingest \
    --client-id eds-prod-1 \
    --device lift-01=<PUBLIC_KEY_HEX>
Restart=on-failure
RestartSec=10
Environment=RUST_LOG=edgesentry_rs=info

[Install]
WantedBy=multi-user.target
```

### 4.3 Health check

`eds serve` does not expose a `/health` endpoint itself — wire a TCP check in your load balancer or monitoring agent:

```bash
# Confirm the TLS port is accepting connections
openssl s_client -connect ingest.example.com:8443 -verify_return_error </dev/null
echo $?   # 0 = healthy
```

For Kubernetes, use a `tcpSocket` liveness probe:

```yaml
livenessProbe:
  tcpSocket:
    port: 8443
  initialDelaySeconds: 5
  periodSeconds: 15
```

---

## 5 — Horizontal Scaling

### 5.1 Architecture

```
                      ┌─────────────────┐
Edge devices  ──TLS──►│  Load balancer  │
                      │  (e.g. nginx /  │
                      │   AWS ALB)      │
                      └────────┬────────┘
                               │  Round-robin
                ┌──────────────┼──────────────┐
                ▼              ▼              ▼
         ┌────────────┐ ┌────────────┐ ┌────────────┐
         │  eds serve │ │  eds serve │ │  eds serve │
         │  node 1    │ │  node 2    │ │  node 3    │
         └──────┬─────┘ └──────┬─────┘ └──────┬─────┘
                └──────────────┼──────────────┘
                               │
                ┌──────────────┼──────────────┐
                ▼              ▼              ▼
         ┌─────────┐    ┌──────────┐   ┌─────────┐
         │Postgres │    │  S3 /    │   │ MinIO   │
         │(primary)│    │  bucket  │   │ cluster │
         └─────────┘    └──────────┘   └─────────┘
```

### 5.2 Key properties

- **`IngestState` is per-process.** Each `eds serve` node maintains its own in-memory sequence/hash-chain state. The `UNIQUE (device_id, sequence)` constraint in PostgreSQL is the cross-node replay fence — a duplicate insert raises a unique-violation error that `PostgresAuditLedger` surfaces as a store error, causing the ingest to be rejected and logged.
- **No sticky sessions required.** Sequence enforcement happens at the DB level; any node can handle any device's request.
- **S3/MinIO writes are stateless.** All nodes write to the same bucket; object keys are derived from `object_ref`, which is set by the edge device and globally unique by convention (e.g. `<device_id>/<sequence>.bin`).

### 5.3 nginx TLS termination + upstream proxy

```nginx
upstream edgesentry_nodes {
    least_conn;
    server 10.0.1.11:8080;
    server 10.0.1.12:8080;
    server 10.0.1.13:8080;
}

server {
    listen 443 ssl;
    server_name ingest.example.com;

    ssl_certificate     /etc/letsencrypt/live/ingest.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/ingest.example.com/privkey.pem;
    ssl_protocols       TLSv1.2 TLSv1.3;
    ssl_ciphers         HIGH:!aNULL:!MD5;

    location /api/v1/ingest {
        proxy_pass         http://edgesentry_nodes;
        proxy_set_header   X-Forwarded-For $remote_addr;
        proxy_read_timeout 10s;
    }
}
```

Run `eds serve` on each node **without** `--tls-cert / --tls-key` (plain HTTP on a private port) and let nginx handle TLS termination. Pass `--allowed-sources` with the nginx upstream IP range.

> **Note:** When TLS is terminated at the load balancer, `eds serve` sees the LB's IP rather than the device's IP. Set `--allowed-sources` to the LB's internal address range, and rely on the LB's own allowlist for per-device source control.

### 5.4 PostgreSQL read replica for reporting

Write path (ingest): primary only.
Read path (compliance queries, chain verification): direct to read replica.

```bash
# Read replica connection for compliance tooling
psql "postgres://audit_ro:pass@pg-replica:5432/audit?sslmode=require"
```

---

## 6 — Observability

Structured logging and tracing are handled by the `tracing` facade. See the [Operations Runbook — Observability](operations.md#observability) section for the full setup including JSON log format, structured event fields emitted by the library, Prometheus metric derivation, and OpenTelemetry span configuration.

### Quick-start: JSON logs to stdout (for Loki / CloudWatch)

```toml
# Cargo.toml of your binary wrapper
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
```

```bash
# Run eds with JSON logs
RUST_LOG=edgesentry_rs=info eds serve ... 2>&1 | \
  promtail --stdin --client.url http://loki:3100/loki/api/v1/push
```

### Key log fields to alert on

| Field | Value | Alert condition |
|-------|-------|----------------|
| `message` | `"MQTT record rejected"` / `"record rejected"` | Rejection rate > 1 % over 5 min |
| `reason` | `"invalid signature"` | Any occurrence — possible tamper attempt |
| `reason` | `"unknown device"` | Sustained — unregistered device probing |
| `message` | `"MQTT event loop error"` | Any — broker connectivity lost |

See [Operations Runbook — Alert Definitions](operations.md#alert-definitions) for Prometheus alerting rules.

---

## See Also

- [Interactive Demo](demo.md) — local Docker Compose quickstart
- [Key Management](key_management.md) — device key provisioning and rotation
- [Operations Runbook](operations.md) — observability, backup, restore, failure drills
- [CLI Reference](cli.md) — full flag reference for `eds serve`, `eds serve-mqtt`, and all subcommands
