# WASM Build — edgesentry-wasm

The `edgesentry-wasm` crate compiles the maritime document pipeline to WebAssembly for use in browser applications. It is consumed by [documaris](https://documaris.pages.dev).

## Why WASM

The full pipeline (parse → fill → check → render → seal) runs entirely client-side in the browser with no server round-trip. Crew PII (FAL Form 5) never leaves the device.

## Build

```bash
cd crates/edgesentry-wasm
wasm-pack build --target web --no-default-features
# Output written to crates/edgesentry-wasm/pkg/
# Copy pkg/ to documaris/app/src/wasm-pkg/ to update the web app
```

`--no-default-features` is required — see Feature Flags below.

## Exported API

All functions accept and return JSON strings or byte slices. No Rust types cross the WASM boundary.

| Function | Signature | Description |
|----------|-----------|-------------|
| `parse_maritime_csv` | `(csv: &str) -> String` | Parse vessel/voyage CSV → JSON `ParsedDocument` |
| `fill` | `(parsed: &str, rules: &str) -> String` | Fill fields, derive confidence scores → JSON `FilledDocument` |
| `check` | `(filled: &str, rules: &str) -> String` | Run compliance rules → JSON `ComplianceAlert[]` |
| `render_html` | `(filled: &str) -> String` | Render FAL Form 1 → HTML string |
| `build_audit_payload` | `(filled: &str, alerts: &str) -> Vec<u8>` | Build AuditRecord payload bytes |
| `seal` | `(payload: &[u8], key_hex: &str) -> String` | Sign with Ed25519 → JSON `AuditRecord` |
| `compute_hash` | `(payload: &[u8]) -> String` | BLAKE3 hash → hex string |

## Feature flags

Two upstream dependencies are gated behind optional features to keep the WASM binary free of C/ASM code that wasm-bindgen cannot compile:

| Crate | Feature | Dep gated | C/ASM source | Action |
|-------|---------|-----------|--------------|--------|
| `edgesentry-parse` | `parquet-support` | `parquet` (via `snap`) | `snap` Snappy codec | Disable — Parquet not needed in browser |
| `edgesentry-document` | `llm` | `ureq` (via `rustls` → `ring`) | `ring` crypto primitives | Disable — LLM calls not made from WASM |

Both features are on by default for native CLI/server builds. The `edgesentry-wasm` `Cargo.toml` sets `default-features = false` for both upstream crates:

```toml
edgesentry-parse    = { path = "../edgesentry-parse",    default-features = false }
edgesentry-document = { path = "../edgesentry-document", default-features = false }
```

## Tests

The crate has 21 unit tests using standard `#[test]` (not `wasm-bindgen-test`) so they run under `cargo test` without a browser:

```bash
cargo test -p edgesentry-wasm
```

## Integration with documaris

1. Build the WASM package: `wasm-pack build --target web --no-default-features`
2. Copy `pkg/` to `documaris/app/src/wasm-pkg/`
3. The documaris app imports from `../wasm-pkg/edgesentry_wasm.js` via `pipeline.ts`

The documaris CI pipeline includes a test that loads the WASM bytes from disk using `readFileSync` to avoid the browser-only `fetch` dependency during testing.
