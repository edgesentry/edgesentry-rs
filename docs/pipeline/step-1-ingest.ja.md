# Step 1 - Ingest

Capture real-world data and normalise it into a schema the rest of the pipeline understands.
Two sub-commands cover structured continuous data (sensor streams); one covers structured
document data (maritime voyage records).

## eds ingest replay

Replay entity positions from a CSV file. Used for testing, CI, and offline demos.

```
eds ingest replay --source <FILE> [--profile <DIR>] --out <FILE>
```

| Flag | Description |
|------|-------------|
| `--source` | Input CSV file |
| `--profile` | Profile directory (reserved for future use) |
| `--out` | Output EntityFrame JSONL file |

**CSV format** (header required):

```
timestamp_ms,entity_id,entity_type,x,y,vx,vy
0,FL-01,forklift,25.0,8.0,-1.0,0.0
0,W-03,pedestrian,15.0,8.0,0.0,0.0
1000,FL-01,forklift,24.0,8.0,-1.0,0.0
```

**Output schema** (`eds.entity-frame`): one record per row in the CSV.

```json
{"eds_schema":"eds.entity-frame","version":"0.1"}
{"timestamp_ms":0,"entity_id":"FL-01","entity_type":"forklift","x":25.0,"y":8.0,"vx":-1.0,"vy":0.0}
```

## eds ingest stream

Stream entity positions from a live UDP source. Used for real-time deployment.

```
eds ingest stream --source <udp://HOST:PORT> --profile <DIR> --out <FILE>
```

| Flag | Description |
|------|-------------|
| `--source` | UDP address, e.g. `udp://127.0.0.1:9000` |
| `--profile` | Profile directory |
| `--out` | Output EntityFrame JSONL file |

Reads JSON-encoded entity packets from UDP and writes them to the output JSONL file until
the process is interrupted. Designed for piping into `eds evaluate run` in a live monitoring loop.

## eds parse maritime

Parse structured maritime voyage data from CSV into `DocumentEntity` JSONL.
Used as the ingest step for the document compliance pipeline.

```
eds parse maritime --source <FILE> --out <FILE>
```

| Flag | Description |
|------|-------------|
| `--source` | Input maritime voyage CSV file |
| `--out` | Output DocumentEntity JSONL file |

**CSV format** (header required):

```
voyage_id,vessel_name,vessel_imo,flag_state,port_of_arrival,arrival_date,
cargo_description,cargo_hs_code,crew_count,gross_tonnage,
bwm_certificate_expiry,dangerous_goods,quarantine_status
```

Empty cells become `null` in the output. Boolean fields accept `true`/`false`/`1`/`0`.

**Output schema** (`eds.document-entity`):

```json
{"eds_schema":"eds.document-entity","version":"0.1"}
{"voyage_id":"V001","vessel_name":"MV Horizon","vessel_imo":"IMO9876543",
 "flag_state":"SGP","port_of_arrival":"SGSIN","arrival_date":"2026-06-15",
 "cargo_description":"General industrial machinery","cargo_hs_code":"8428",
 "crew_count":23,"gross_tonnage":45000.0,"bwm_certificate_expiry":"2027-03-01",
 "dangerous_goods":false,"quarantine_status":"CLEAR","crew_nationalities":null}
```
