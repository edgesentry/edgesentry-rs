# edgesentry-document

`eds.document-entity` JSONL → filled form JSONL → compliance alerts JSONL → HTML

Three `eds document` steps: `fill` (AI field completion), `check` (compliance rules), `gen` (HTML render).

`llm` feature (default on) pulls C/ASM bindings. Disable for WASM: `--no-default-features`.
