# CLI Reference

## Device Provisioning

Generate a fresh Ed25519 keypair for a new device:

```bash
cargo run -p edgesentry-rs -- keygen
```

Save directly to a file:

```bash
cargo run -p edgesentry-rs -- keygen --out device-lift-01.key.json
```

Derive the public key from an existing private key:

```bash
cargo run -p edgesentry-rs -- inspect-key \
  --private-key-hex 0101010101010101010101010101010101010101010101010101010101010101
```

See [Key Management](key_management.md) for the full provisioning and rotation workflow.

---

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
  --public-key-hex 8a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c
```

Verify a whole chain from a JSON array file:

```bash
cargo run -p edgesentry-rs -- verify-chain --records-file records.json
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
  --public-key-hex 8a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c
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
  --public-key-hex 8a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c
```

Expected output:

```text
INVALID
```

---

## Server Commands

### `eds serve` — HTTP ingest server

Requires the `transport-http` Cargo feature.

| Flag | Default | Description |
|------|---------|-------------|
| `--addr` | `0.0.0.0:8080` | Socket address to bind |
| `--allowed-sources` | `127.0.0.1` | Comma-separated CIDRs / IPs allowed to connect |
| `--device ID=PUBKEY_HEX` | _(none)_ | Register a device; repeat for multiple devices |

```bash
eds serve \
  --addr 0.0.0.0:8080 \
  --allowed-sources 10.0.0.0/8 \
  --device lift-01=<PUBLIC_KEY_HEX>
```

Plain HTTP on port 8080. Use behind a TLS-terminating reverse proxy, or use `eds serve-tls` for built-in TLS.

---

### `eds serve-tls` — HTTPS ingest server (TLS 1.2/1.3)

Requires the `transport-tls` Cargo feature.

| Flag | Default | Description |
|------|---------|-------------|
| `--addr` | `0.0.0.0:8443` | Socket address to bind |
| `--allowed-sources` | `127.0.0.1` | Comma-separated CIDRs / IPs allowed to connect |
| `--device ID=PUBKEY_HEX` | _(none)_ | Register a device; repeat for multiple devices |
| `--tls-cert` | _(required)_ | Path to PEM certificate chain (leaf first) |
| `--tls-key` | _(required)_ | Path to PEM private key (PKCS #8 or PKCS #1 RSA) |

```bash
eds serve-tls \
  --addr 0.0.0.0:8443 \
  --allowed-sources 10.0.0.0/8 \
  --device lift-01=<PUBLIC_KEY_HEX> \
  --tls-cert /etc/edgesentry/server.crt \
  --tls-key  /etc/edgesentry/server.key
```

Uses rustls TLS 1.2/1.3. Network policy (IP allowlist) is enforced at TCP accept time, before the TLS handshake.

---

### `eds serve-mqtt` — MQTT ingest subscriber

Requires the `transport-mqtt` Cargo feature. Optionally add `transport-mqtt-tls` for MQTTS.

| Flag | Default | Description |
|------|---------|-------------|
| `--broker` | `localhost` | MQTT broker host |
| `--port` | `1883` | MQTT broker port (use `8883` for MQTTS) |
| `--topic` | `edgesentry/ingest` | Topic to subscribe for ingest records |
| `--client-id` | `eds-server` | MQTT client identifier |
| `--device ID=PUBKEY_HEX` | _(none)_ | Register a device; repeat for multiple devices |
| `--tls-ca-cert` | _(none)_ | Path to PEM CA cert for MQTTS broker verification (`transport-mqtt-tls` only) |

```bash
# Plain MQTT (port 1883)
eds serve-mqtt \
  --broker broker.example.com \
  --port 1883 \
  --topic edgesentry/ingest \
  --device lift-01=<PUBLIC_KEY_HEX>

# MQTTS (port 8883, requires transport-mqtt-tls feature)
eds serve-mqtt \
  --broker broker.example.com \
  --port 8883 \
  --tls-ca-cert /etc/edgesentry/ca.crt \
  --device lift-01=<PUBLIC_KEY_HEX>
```

Responses are published on `<topic>/response` as JSON with `status: "accepted"` or `status: "rejected"`.

---

## Ingestion Demo (PostgreSQL + MinIO)

Requires the `s3` and `postgres` Cargo features and a running PostgreSQL + MinIO instance (use `docker compose -f docker-compose.local.yml up -d`).

### 1) Generate a chain with payloads file

```bash
cargo run -p edgesentry-rs --features s3,postgres -- demo-lift-inspection \
  --device-id lift-01 \
  --out-file lift_inspection_records.json \
  --payloads-file lift_inspection_payloads.json
```

### 2) Ingest records through IngestService

```bash
cargo run -p edgesentry-rs --features s3,postgres -- demo-ingest \
  --records-file lift_inspection_records.json \
  --payloads-file lift_inspection_payloads.json \
  --device-id lift-01 \
  --pg-url postgresql://trace:trace@localhost:5433/trace_audit \
  --minio-endpoint http://localhost:9000 \
  --minio-bucket bucket \
  --minio-access-key minioadmin \
  --minio-secret-key minioadmin \
  --reset
```

`--reset` truncates `audit_records` and `operation_logs` before ingesting.  Omit it to append to an existing run.

Pass `--tampered-records-file <path>` to also demonstrate rejection of a tampered chain through the same `IngestService`.

See [Interactive Demo](demo.md) for the full guided walkthrough with PostgreSQL and MinIO.
