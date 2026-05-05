# CV Adapter Specification

edgesentry-rs starts at entity positions. The component that converts camera frames
into entity positions is called a **CV adapter**. This document defines the contract
a CV adapter must satisfy, describes the in-house OSS adapter (specula) that is
ready for PoC use, and explains how to plug in a specialist CV solution.

---

## Design principle: clean adapter boundary

edgesentry-rs is agnostic about where entity positions come from.
The CV layer connects at a single interface: `eds.entity-frame` JSONL.

This means:
- The in-house OSS adapter (specula) and a specialist vendor adapter are interchangeable
- Swapping CV solutions requires no changes to the physics engine or audit chain
- Engineering effort stays focused on what is differentiated: physics evaluation,
  regulatory mapping, and tamper-proof evidence infrastructure

```
Camera frames
  │
  ▼
CV adapter  ←── specula (OSS, ready now)  or  specialist vendor adapter
  │
  │  eds.entity-frame JSONL
  ▼
edgesentry-ingest  ◄── edgesentry-rs boundary
  │
edgesentry-evaluate → edgesentry-audit → R2
```

---

## Output contract: EntityFrame JSONL

Any CV adapter must produce `eds.entity-frame` JSONL, one record per timestamp:

```json
{"eds_schema": "eds.entity-frame", "version": "0.2.0"}
{
  "timestamp_ms": 6000,
  "entities": [
    {
      "id": "FL-01",
      "class": "Forklift",
      "x": 6.0,
      "y": 0.0,
      "vx": 3.0,
      "vy": 0.0,
      "confidence": 0.91
    },
    {
      "id": "W-03",
      "class": "Person",
      "x": 12.0,
      "y": 0.0,
      "vx": 0.0,
      "vy": 0.0,
      "confidence": 0.87
    }
  ]
}
```

**Requirements:**

| Field | Requirement |
|---|---|
| `id` | Stable across frames for the same physical entity (tracker output) |
| `class` | One of: `Forklift`, `Person`, `Vessel`, `ReachStacker` |
| `x`, `y` | Real-world metres from site reference point (not pixels) |
| `vx`, `vy` | Metres per second (frame-delta or tracker-provided) |
| `confidence` | 0.0–1.0, optional but strongly recommended |
| `timestamp_ms` | Unix milliseconds, monotonically increasing |
| Position accuracy | < 0.5 m at operational range for TTC to be meaningful |

---

## In-house OSS adapter: specula

**Repository:** `edgesentry/specula`
**Status:** ready for PoC use

specula is an in-house OSS CV adapter built on proven open-source components.
It runs the full pipeline from a live camera to `eds.entity-frame` JSONL output,
enabling real on-site PoC work without dependency on any third-party vendor.

A specialist CV solution can replace specula at the adapter boundary
with no changes to edgesentry-rs. specula is retained as the reference implementation
and a working fallback.

### Stack

| Component | Choice | Reason |
|---|---|---|
| Object detection | YOLO v11 (Ultralytics) | Apache 2.0, strong terminal/warehouse pretrained weights |
| Multi-object tracking | ByteTrack (via supervision) | Stable ID maintenance across occlusion |
| Coordinate transform | OpenCV homography | Per-camera calibration from 4+ ground-truth points |
| Output | UDP → `edgesentry-ingest` or JSONL file | Matches edgesentry-rs ingest interface |
| Language | Python 3.11+ | Fastest iteration; not deployed to production Rust stack |

### Adapter structure

```
specula/
  adapters/
    mock_replay/   # CSV fixture → EntityFrame UDP (demo / CI)
    yolo_v8/       # live camera or recorded video → EntityFrame
  calibration/
    homography.py  # pixel-to-metre transform
    site_config.toml
  specula/
    entity_stream.py   # EntityFrame JSONL / UDP writer
    gap_detector.py    # emits EntityGap when entity disappears
  README.md
```

### Limitations (to be disclosed at any PoC)

- Detection accuracy is untested against industrial certification standards
- Calibration is manual (4-point homography); errors propagate to TTC calculations
- No multi-camera fusion; each camera is an independent adapter instance
- Low-light and high-glare scenarios require IR camera or separate lighting setup
- Not suitable as evidence of system reliability — only as a functional demonstration

### Gap between specula and production

| Requirement | specula | Production (vendor) |
|---|---|---|
| Detection accuracy | ~85–90 % (YOLO pretrained) | Vendor-certified |
| Multi-camera fusion | Manual, per-camera | Vendor-provided |
| Confidence calibration | Raw softmax (not calibrated) | Platt scaling or equivalent |
| Edge device deployment | Python, GPU recommended | Vendor SDK, may run on CPU |
| Support / liability | None | Vendor SLA |

---

## Integration test

The `mock_replay` adapter replays any edgesentry-rs CSV fixture as a UDP EntityFrame
stream. This allows the full pipeline (specula → edgesentry → seal → R2) to be
validated end-to-end without a live camera.

```bash
# start mock replay
python specula/adapters/mock_replay/replay.py \
  --fixture ../../clarus/fixtures/forklift_approach.csv \
  --port 9000 --fps 2

# edgesentry receives on UDP
eds ingest stream --source udp://localhost:9000 --profile profiles/demo --out /tmp/frames.jsonl
```
