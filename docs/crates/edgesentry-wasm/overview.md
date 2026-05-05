# edgesentry-wasm

WebAssembly bindings exposing the document pipeline to JavaScript/TypeScript.

## Build
```bash
cd crates/edgesentry-wasm
wasm-pack build --target web --no-default-features
```
`--no-default-features` disables `parquet-support` and `llm` — both pull C/ASM deps
incompatible with wasm-bindgen.

## Consumer
[documaris](https://documaris.pages.dev) — browser-based FAL Form 1 generator.
