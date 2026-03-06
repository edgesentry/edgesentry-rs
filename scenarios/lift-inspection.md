# Lift Inspection Scenario

## Goal

Demonstrate a tamper-evident inspection session for one lift (`lift-01`) using signed records and hash-chain verification.

## Inspection Steps

1. Door open/close cycle check
2. Vibration check
3. Emergency brake response check

## CLI Flow

Generate a complete signed chain:

```bash
cargo run -p audit-cli -- demo-lift-inspection \
  --device-id lift-01 \
  --out-file lift_inspection_records.json
```

Verify the generated chain:

```bash
cargo run -p audit-cli -- verify-chain --records-file lift_inspection_records.json
```

Generate and verify a single event:

```bash
cargo run -p audit-cli -- sign-record \
  --device-id lift-01 \
  --sequence 1 \
  --timestamp-ms 1700000000000 \
  --payload "scenario=lift-inspection,check=door,status=ok" \
  --object-ref "s3://bucket/lift-01/door-check-1.bin" \
  --private-key-hex 0101010101010101010101010101010101010101010101010101010101010101 \
  --out lift_single_record.json

cargo run -p audit-cli -- verify-record \
  --record-file lift_single_record.json \
  --public-key-hex 8a88e3dd7409f195fd52db2d3cba5d72ca670bf1d94121bf3748801b40f6f5c0
```
