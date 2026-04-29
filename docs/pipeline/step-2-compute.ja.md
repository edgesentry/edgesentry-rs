# Step 2 - Compute

Apply physics and geometry operations to raw entity measurements.

```
eds compute run --input <FILE> --out <FILE>
```

| Flag | Description |
|------|-------------|
| `--input` | Input EntityFrame JSONL file (from `eds ingest replay` or `eds ingest stream`) |
| `--out` | Output Measurement JSONL file |

## What it computes

For every pair of entities in every frame:

| Function | Output | Formula |
|---|---|---|
| `euclidean_distance` | metres | `sqrt((x2-x1)^2 + (y2-y1)^2)` |
| `relative_velocity` | m/s | component of velocity along the line between entities |
| `time_to_collision` | seconds | `distance / closing_speed` (only when entities are approaching) |
| `braking_distance` | metres | entity-class-specific lookup table |
| `zone_membership` | bool | point-in-polygon test using the profile zone definition |

TTC is only computed when `closing_speed > 0` (entities approaching). A positive TTC means
a collision would occur at current trajectories; a negative or infinite TTC means they are
separating.

## Note on pipeline usage

`eds evaluate run` reads the original `EntityFrame` JSONL and applies physics internally.
`eds compute run` is provided for inspection and debugging -- it lets you examine the raw
measurements before rule evaluation. In production pipelines both commands may be run, or
only `eds evaluate run` if intermediate measurements are not needed.
