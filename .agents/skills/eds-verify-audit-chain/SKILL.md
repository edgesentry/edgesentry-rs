---
name: eds-verify-audit-chain
description: Verify an edgesentry audit chain using the eds CLI. Use to confirm a chain is intact and a payload matches its AuditRecord.
license: Apache-2.0
compatibility: Requires eds binary (cargo build --release)
metadata:
  repo: edgesentry-rs
---

## Build

```bash
cargo build --release
```

## Generate a keypair

```bash
./target/release/eds audit keygen > /tmp/keypair.json
KEY=$(python3 -c "import json; print(json.load(open('/tmp/keypair.json'))['private_key_hex'])")
```

## Sign a payload

```bash
./target/release/eds audit sign-document \
  --payload <input.jsonl> \
  --key "$KEY" \
  --device-id <device-id> \
  --out /tmp/chain.json
```

## Verify payload integrity

```bash
./target/release/eds audit verify-document \
  --payload <input.jsonl> \
  --chain /tmp/chain.json
```

Exits 0 if payload matches the AuditRecord.

## Verify chain integrity

```bash
./target/release/eds audit verify-chain --chain /tmp/chain.json
```

Exits 0 and prints `chain OK` if no records have been tampered with.

## Interpreting failures

| Command | Non-zero exit means |
|---|---|
| `verify-document` | Payload was modified after signing |
| `verify-chain` | A record was altered or the sequence is broken |
