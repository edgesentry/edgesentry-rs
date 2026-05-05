# edgesentry-wasm

WebAssembly bindings for the document pipeline (`edgesentry-parse` → `edgesentry-document` → `edgesentry-audit`).

Build: `wasm-pack build --target web --no-default-features` — both `parquet-support` and `llm` must be disabled (C/ASM deps incompatible with wasm-bindgen).

Consumer: [documaris](https://documaris.pages.dev)
