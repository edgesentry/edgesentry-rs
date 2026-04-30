# CV Adapter Specification

edgesentry-rs starts at entity positions. The component that converts camera frames
into entity positions is called a **CV adapter**. This document defines the contract
a CV adapter must satisfy, and describes the interim OSS-based adapter (specula)
maintained as a contingency for on-site PoC work.

---

## Preferred path: MENOU

The CV adapter is not a core competency of edgesentry.
Engineering effort is concentrated on the physics engine, rule evaluation,
audit chain, and evidence infrastructure — not object detection.

When a PoC site is confirmed, cooperation will be requested from
**MENOU** ([menou.co.jp](https://menou.co.jp/home_en)), a specialist in
industrial AI visual inspection with a proven track record in manufacturing environments.

A vendor integration means:
- Detection accuracy is the vendor's certified responsibility, not ours
- Industrial safety standards are addressed by the vendor
- We focus on what is differentiated: physics evaluation, regulatory mapping, audit infrastructure

The vendor adapter only needs to implement the output contract below.
The edgesentry-rs physics engine and audit chain are vendor-agnostic.

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

## Interim solution: specula

**Repository:** `edgesentry/specula`
**Status:** fallback — used only if MENOU cooperation is not yet secured when an on-site PoC is required

specula is a minimal in-house OSS-based CV adapter. It is not a production system and not a focus area.
It exists to unblock on-site PoC work when vendor cooperation has not yet been arranged.
Engineering effort is directed at the physics engine and audit chain, not at specula.

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
