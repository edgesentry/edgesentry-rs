# edgesentry-document

Document compliance pipeline — AI field filling, rule checking, HTML rendering.

## Input → Output
`DocumentEntity` JSONL → filled form JSONL → compliance alerts → HTML document

## Steps (via `eds document`)
1. `fill` — AI-assisted field completion; marks low-confidence fields as `review_required`
2. `check` — runs compliance rules (e.g. BWM_D2_EXPIRED) against filled fields
3. `gen` — renders filled form to HTML via template

## Feature flags
- `llm` (default on) — enables `ureq` HTTP client for LLM calls.
  Disable for WASM: `--no-default-features`.
