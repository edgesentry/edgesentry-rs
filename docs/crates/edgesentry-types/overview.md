# edgesentry-types

Shared types across all edgesentry crates. No I/O.

## Key types

| Type | Purpose |
|---|---|
| `Entity` | A tracked object: position, velocity, class, sensor, confidence |
| `EntityFrame` | Snapshot of all entities at one timestamp |
| `EntityClass` | `Vessel`, `AisGap`, `Forklift`, `Worker`, … |
| `SensorReading` | `SourceType` + optional confidence: `Ais`, `LiDAR`, `UWB`, `Simulation`, … |
| `EvidenceQuality` | `CERTIFIED` / `DEGRADED` / `NOT_APPLICABLE` — derived from confidence |
| `Vec2` / `Vec3` | Geometry primitives (`f32`, consistent with trilink-core) |
