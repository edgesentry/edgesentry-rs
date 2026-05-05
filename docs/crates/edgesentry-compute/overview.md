# edgesentry-compute

Physics and geometry computations over entity frames.

## Input → Output
`EntityFrame` JSONL → `MeasurementFrame` JSONL (`eds.measurement-frame`)

## Computed per frame
- Pairwise Euclidean distances (2D fallback when `position_z` absent)
- Relative velocity vectors
- Time-to-collision (TTC) estimates
- Zone membership booleans
- Entity confidence scores (`compute_entity_confidence`)
- `CalibrationStatus` (`Valid` / `Degraded` / `Uncalibrated`)

## Confidence formula
`clamp((base + stddev_adj) × freshness × calib_mult, 0.0, 1.0)`
Base by source: AIS=1.00, LiDAR=0.95, UWB=0.90, PointSensor=0.80, CV=score.
