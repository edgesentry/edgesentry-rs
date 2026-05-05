# eds — Unified CLI

Composes all pipeline stages as subcommands. Same binary at edge and cloud — context determined by which subcommands and profiles are present.

## Subcommand groups

| Group | Role |
|---|---|
| `eds ingest replay` | CSV / AIS fixture → `EntityFrame` JSONL |
| `eds compute run` | `EntityFrame` → `MeasurementFrame` JSONL |
| `eds evaluate run` | `MeasurementFrame` + profile → `RiskEvent` JSONL |
| `eds assess` | `RiskEvent` stream → trend / correlation output |
| `eds explain` | `RiskEvent` + KB → plain-language explanation |
| `eds report` | pipeline output → Markdown report |
| `eds audit keygen / sign-document / verify-document / verify-chain` | audit chain operations |
| `eds parse maritime` | maritime CSV → `DocumentEntity` JSONL |
| `eds document fill / check / gen` | document pipeline steps |

## Inter-stage JSONL schemas

| Schema | Producer | Consumer |
|---|---|---|
| `eds.entity-frame` | `eds ingest replay` | `eds compute run` |
| `eds.measurement-frame` | `eds compute run` | `eds evaluate run` |
| `eds.risk-event` | `eds evaluate run` | `eds assess`, `eds explain`, `eds audit` |
| `eds.document-entity` | `eds parse maritime` | `eds document fill` |
