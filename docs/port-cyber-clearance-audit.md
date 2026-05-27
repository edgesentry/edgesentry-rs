# Port Cyber Clearance — audit seal and third-party verify

Cap Vista PoC (W4): seal indago clearance evaluation manifests with `edgesentry-rs` `AuditRecord` chains.

## Prerequisites

- `eds` binary: `cargo build --release -p eds`
- indago evaluation output: `*_evaluation_manifest.json` from `port_clearance_eval.py evaluate --write`

## 1. Generate a keypair (once per demo environment)

```bash
eds audit keygen > /tmp/clearance-keypair.json
KEY=$(python3 -c "import json; print(json.load(open('/tmp/clearance-keypair.json'))['private_key_hex'])")
```

## 2. Sign the evaluation manifest

```bash
eds audit sign-clearance \
  --manifest data/processed/maritime_cyber/vessel-hold_port-call-demo-sgsin_evaluation_manifest.json \
  --key "$KEY" \
  --device-id port-clearance-poc \
  --out /tmp/clearance-chain.json
```

## 3. Third-party verify (no EdgeSentry UI)

### Chain integrity

```bash
eds audit verify-chain --records-file /tmp/clearance-chain.json
# CHAIN_VALID
```

### Manifest matches sealed payload

```bash
eds audit verify-clearance \
  --manifest data/processed/maritime_cyber/vessel-hold_port-call-demo-sgsin_evaluation_manifest.json \
  --chain /tmp/clearance-chain.json
# VERIFIED + CHAIN_VALID
```

If the manifest or chain was tampered with after signing, `verify-clearance` exits non-zero.

## End-to-end with indago (demo script)

```bash
# indago repo
uv run python pipelines/port_clearance_eval.py evaluate vessel-hold --write

# edgesentry-rs repo
cargo build --release -p eds
eds audit sign-clearance --manifest .../vessel-hold_port-call-demo-sgsin_evaluation_manifest.json ...
eds audit verify-chain --records-file /tmp/clearance-chain.json
eds audit verify-clearance --manifest ... --chain /tmp/clearance-chain.json
```

## Honesty (portal)

PoC uses **public CVE** + **synthetic SBOM/asset_map** fixtures. The audit chain proves **what was evaluated and when** — not MPA official berth approval.
