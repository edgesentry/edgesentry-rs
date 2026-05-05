# eds — Unified CLI

Composes all pipeline stages as subcommands. Same binary at edge and cloud — context is determined by which subcommands are invoked and which profile files are present.

## Inter-stage JSONL schemas

| Schema | Producer | Consumer |
|---|---|---|
| `eds.entity-frame` | `eds ingest replay` | `eds compute run` |
| `eds.measurement-frame` | `eds compute run` | `eds evaluate run` |
| `eds.risk-event` | `eds evaluate run` | `eds assess`, `eds explain`, `eds audit` |
| `eds.document-entity` | `eds parse maritime` | `eds document fill` |
