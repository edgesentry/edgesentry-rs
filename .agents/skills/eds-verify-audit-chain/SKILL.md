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
  --chain /tmp/chain.json \
  --public-key-hex "$PUB"
```

Exits 0 and prints `VERIFIED` only after the trusted public key verifies the matching record.

## Verify chain integrity

```bash
PUB=$(python3 -c "import json; print(json.load(open('/tmp/keypair.json'))['public_key_hex'])")
./target/release/eds audit verify-chain --records-file /tmp/chain.json --public-key-hex "$PUB"
```

Exits 0 and prints `CHAIN_VALID` when hash links, sequence, and every signature verify.

## Interpreting failures

| Command | Non-zero exit means |
|---|---|
| `verify-document` | Payload was modified after signing |
| `verify-chain` | A record was altered or the sequence is broken |
