# edgesentry-wasm

WebAssembly bindings for the EdgeSentry maritime document pipeline.

Exposes `edgesentry-parse`, `edgesentry-document`, and `edgesentry-audit` to browser applications via [wasm-bindgen](https://rustwasm.github.io/wasm-bindgen/).

Used by [documaris](https://documaris.pages.dev) — the browser-based FAL Form 1 generator.

## Build

```bash
wasm-pack build --target web --no-default-features
# Output: pkg/  (copy to documaris/app/src/wasm-pkg/)
```

`--no-default-features` disables the `parquet-support` and `llm` features in the upstream crates — both pull in C/ASM code that wasm-bindgen cannot compile.

## Exported functions

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `parse_maritime_csv` | CSV string | JSON `ParsedDocument` | Parse vessel/voyage CSV |
| `fill` | JSON `ParsedDocument` + rules JSON | JSON `FilledDocument` | Fill fields, derive confidence scores |
| `check` | JSON `FilledDocument` + rules JSON | JSON `ComplianceAlert[]` | Run compliance rules |
| `render_html` | JSON `FilledDocument` | HTML string | Render FAL Form 1 |
| `build_audit_payload` | JSON `FilledDocument` + alerts JSON | bytes | Build AuditRecord payload |
| `seal` | payload bytes + private key hex | JSON `AuditRecord` | Sign with Ed25519 |
| `compute_hash` | payload bytes | hex string | BLAKE3 hash |

All functions take/return JSON strings or byte slices — no Rust types cross the WASM boundary.

## Feature flags

This crate depends on `edgesentry-parse` and `edgesentry-document` with `default-features = false`:

- `parquet-support` (edgesentry-parse) — disabled: `snap` C bindings
- `llm` (edgesentry-document) — disabled: `ring` C/ASM code via `ureq` → `rustls`
