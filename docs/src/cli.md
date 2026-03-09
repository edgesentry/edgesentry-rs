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
  --public-key-hex 8a88e3dd7409f195fd52db2d3cba5d72ca670bf1d94121bf3748801b40f6f5c0
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
