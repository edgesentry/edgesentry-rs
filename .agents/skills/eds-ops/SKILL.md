---
name: eds-ops
description: Operate edgesentry-rs in production — check health, query metrics, run backups, restore from backup. Use when monitoring or maintaining a deployed instance.
license: Apache-2.0
compatibility: Requires access to deployed instance, Prometheus, PostgreSQL, S3/MinIO
metadata:
  repo: edgesentry-rs
---

## Health check

```bash
curl -sf https://<host>/health | jq .
```

Expected: `{"status":"ok","chain_head":<n>}`

## Key metrics to watch

| Metric | Alert threshold |
|---|---|
| `edgesentry_ingest_errors_total` | > 0 sustained for 5 min |
| `edgesentry_chain_lag_seconds` | > 30 |
| `edgesentry_verify_failures_total` | Any non-zero value |

Full metric reference: [references/operations.md](references/operations.md)

## Backup

```bash
# PostgreSQL
pg_basebackup -h <host> -U edgesentry -D /backup/pg -Ft -z -P

# S3/MinIO — verify Object Lock is enabled before treating as tamper-proof
aws s3 ls s3://edgesentry-audit --recursive | wc -l
```

## Restore

```bash
# Stop service
systemctl stop edgesentry-ingest

# Restore PostgreSQL
pg_restore -h <host> -U edgesentry -d edgesentry /backup/pg/base.tar.gz

# Restart and verify chain
systemctl start edgesentry-ingest
eds audit verify-chain --chain <latest-export>.json
```

See [references/operations.md](references/operations.md) for RTO/RPO targets, alert routing, and OpenTelemetry configuration.
