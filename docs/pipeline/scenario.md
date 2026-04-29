# Scenario - Synthetic Data Generation

Generate synthetic entity CSV fixtures and stream them over UDP to test the pipeline without
a physical sensor.

## eds scenario generate

Generate a CSV file containing synthetic entity positions across N frames.

```
eds scenario generate --out <FILE>
                      [--entities N] [--frames N] [--seed N]
                      [--scenario-type entity]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--entities` | 2 | Number of entities to simulate |
| `--frames` | 10 | Number of time frames |
| `--seed` | 0 | Integer seed for reproducible output (LCG RNG -- no external dependency) |
| `--scenario-type` | `entity` | Scenario type (only `entity` is currently supported) |
| `--out` | | Output CSV file path |

**Output format** -- same header as `eds ingest replay` input:

```
timestamp_ms,entity_id,entity_type,x,y,vx,vy
0,E-0,Forklift,12.3,7.6,-0.8,0.2
0,E-1,Person,4.1,9.3,0.0,0.0
100,E-0,Forklift,12.2,7.6,-0.8,0.2
...
```

Entity types alternate: even-indexed entities are `Forklift`, odd-indexed are `Person`.
Each entity has a random starting position within `[0, 20]` metres and a fixed velocity.
Frame interval is 100 ms (10 fps by default).

**Bundled fixture**: `crates/edgesentry-scenario/fixtures/simple_crossing.csv` -- two Forklift
entities approaching each other head-on across 10 frames.

## eds scenario simulate

Read a scenario CSV and stream entity frames over UDP to a running `eds ingest stream` process.

```
eds scenario simulate --source <FILE> --target <udp://HOST:PORT>
                      [--fps N]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--source` | | Input CSV file (produced by `eds scenario generate` or hand-written) |
| `--target` | | UDP target address, e.g. `udp://127.0.0.1:9000` |
| `--fps` | 10 | Frames per second -- controls sleep between frame sends |

Each frame is sent as a single UDP datagram containing a JSON object:

```json
{"entities": [
  {"id": "E-0", "class": "Forklift", "x": 12.3, "y": 7.6, "vx": -0.8, "vy": 0.2, "timestamp_ms": 0},
  {"id": "E-1", "class": "Person",   "x": 4.1,  "y": 9.3, "vx": 0.0,  "vy": 0.0, "timestamp_ms": 0}
]}
```

This matches the `UnityPacket` format expected by `eds ingest stream`.

## End-to-end example

```bash
# Terminal 1 -- start ingest stream listener
eds ingest stream \
  --source udp://127.0.0.1:9000 \
  --profile crates/edgesentry-profile/fixtures/demo \
  --out /tmp/live.jsonl

# Terminal 2 -- generate and stream a scenario
eds scenario generate --frames 20 --entities 3 --out /tmp/scenario.csv
eds scenario simulate --source /tmp/scenario.csv --target udp://127.0.0.1:9000 --fps 10

# Evaluate the captured frames
eds evaluate run \
  --input /tmp/live.jsonl \
  --profile crates/edgesentry-profile/fixtures/demo \
  --out /tmp/events.jsonl
```
