---
name: eds-deploy
description: Deploy edgesentry-rs to a new server — TLS, PostgreSQL, S3/MinIO, systemd. Use when setting up a new deployment environment.
license: Apache-2.0
compatibility: Requires Ubuntu/Debian, PostgreSQL 15+, access to S3/MinIO, Let's Encrypt or existing TLS cert
metadata:
  repo: edgesentry-rs
---

## 1. TLS certificate

```bash
# Let's Encrypt (internet-facing)
certbot certonly --standalone -d <domain>

# Self-signed (air-gapped)
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes
```

## 2. PostgreSQL

```bash
createdb edgesentry
createuser edgesentry --pwprompt
psql edgesentry < crates/edgesentry-audit/migrations/001_init.sql
```

## 3. S3/MinIO bucket

Enable Object Lock (Compliance mode) for tamper-proof retention:

```bash
aws s3api create-bucket --bucket edgesentry-audit
aws s3api put-object-lock-configuration \
  --bucket edgesentry-audit \
  --object-lock-configuration '{"ObjectLockEnabled":"Enabled"}'
```

## 4. systemd service

```bash
cp deploy/edgesentry-ingest.service /etc/systemd/system/
systemctl daemon-reload
systemctl enable edgesentry-ingest
systemctl start edgesentry-ingest
```

## 5. Verify

```bash
systemctl status edgesentry-ingest
curl -sf https://<host>/health | jq .
```

Full reference (PostgreSQL tuning, Kubernetes probes, horizontal scaling): [references/deployment.md](references/deployment.md)
