# edgesentry-parse

Maritime structured data parsing.

## Input → Output
CSV or Parquet (vessel, voyage, cargo fields) → `DocumentEntity` JSONL

## Feature flags
- `parquet-support` (default on) — enables Parquet; pulls `snap` (C bindings).
  Disable for WASM: `--no-default-features`.
