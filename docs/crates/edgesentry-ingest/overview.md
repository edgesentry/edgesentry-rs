# edgesentry-ingest

Structured data ingestion — produces `EntityFrame` JSONL.

## Adapters

| Adapter | Input | Notes |
|---|---|---|
| `csv_replay` | CSV fixture file | Infers `SensorReading` from `EntityClass` (Vessel/AisGap → AIS) |
| AIS stream | NMEA 0183 UDP/WebSocket | Live vessel positions |
| PLY / IFC | Point cloud files | For edgesentry-inspect pipeline |

## Output schema
`eds.entity-frame` — one record per timestamp, array of `Entity`.
