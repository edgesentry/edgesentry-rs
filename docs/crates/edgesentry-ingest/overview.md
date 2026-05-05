# edgesentry-ingest

Produces `eds.entity-frame` JSONL from structured input sources.

Infers `SensorReading` from `EntityClass` (e.g. Vessel/AisGap → AIS) when the source does not provide it explicitly.
