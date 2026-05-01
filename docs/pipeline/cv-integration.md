# CV Integration: System Boundary and Constraints

edgesentry-rs begins at **entity positions**. It does not process video frames.

This document defines where edgesentry-rs fits in the full production pipeline,
what the ingest layer assumes about its inputs, and what environmental constraints
propagate from the CV layer upstream.

---

## Pipeline position

```
Camera frames  (outside edgesentry-rs)
      │
      ▼
CV model — object detection + tracking  (outside edgesentry-rs)
      │
      │  EntityFrame: { id, class, x_m, y_m, vx_ms, vy_ms, timestamp_ms }
      │  optional:    { confidence: f32 }
      ▼
edgesentry-ingest  ◄── edgesentry-rs boundary starts here
      │
edgesentry-compute → edgesentry-evaluate → edgesentry-assess
      │
edgesentry-explain → edgesentry-audit → R2 Object Lock
```

The ingest adapter is responsible for:

1. Receiving entity positions (from UDP socket, file, AIS stream, or CV adapter)
2. Converting site-specific pixel or geographic coordinates to metres
3. Normalising entity class names to `EntityClass` variants
4. Emitting `EntityFrame` JSONL

Everything downstream of ingest operates in metric coordinates and is site-agnostic.

---

## What edgesentry-rs assumes about entity positions

| Field | Assumption |
|---|---|
| `x`, `y` | Real-world metres, origin at a site-defined reference point |
| `vx`, `vy` | Metres per second, derived from frame-to-frame delta or CV tracker output |
| `class` | One of: `Forklift`, `Person`, `Vessel`, `ReachStacker`, `AisGap`, … |
| `timestamp_ms` | Unix milliseconds, monotonically increasing per entity |
| Accuracy | Position error < 0.5 m at operational range (required for TTC to be meaningful) |

If the CV layer produces noisy positions (jitter, teleportation between frames),
the physics calculations will produce incorrect TTC and braking-distance values.
edgesentry-rs has no way to detect or correct for this upstream noise.

---

## Site adaptation: the profile system

Different sites have different camera configurations.
The `profile/` directory is the per-site adaptation layer.

```toml
# profiles/my-site/params.toml
[reference_point]
lat_deg = 1.2640      # for AIS / GPS inputs
lon_deg = 103.8200

[camera.zone_b]
px_per_metre = 42.3
origin_px = [960, 540]

[ais_gap]
threshold_s = 480
```

Physics rules reference metres, not pixels.
Calibration (pixel → metre) is the responsibility of the ingest adapter,
using the params.toml values.

Zone polygons in rules.json are in metres relative to the site reference point:

```json
{
  "rule_id": "RESTRICTED_ZONE",
  "condition": "zone_member",
  "zone": [[-300, 200], [300, 200], [300, 700], [-300, 700]]
}
```

---

## Environmental constraints and propagation

edgesentry-rs is deterministic given correct inputs.
Incorrect inputs from the CV layer propagate as follows:

### 1. Entity not detected (low light, occlusion, fog)

The entity simply does not appear in the EntityFrame.
No `RiskEvent` is generated.
The audit chain records silence — indistinguishable from "nothing happened."

**Mitigation:** emit a `CoverageGap` event from the CV adapter when
detection confidence falls below threshold. This gets sealed into the chain
alongside risk events, making degraded-monitoring periods auditable.

### 2. Low-confidence detection (shadow, glare)

The CV tracker may output a position with low confidence.
edgesentry-rs accepts it as-is — it has no confidence field in the current `Entity` struct.

**Planned:** add optional `confidence: Option<f32>` to `Entity`.
The evaluate layer can then suppress events below a threshold,
and the audit record can include the confidence score.

### 3. Coordinate noise / jitter

Noisy positions cause jittery velocity vectors.
This can generate spurious TTC alerts or miss real ones.

**Mitigation:** apply a Kalman filter or exponential moving average
in the ingest adapter before emitting EntityFrames.
edgesentry-rs does not filter — filtering is an ingest-layer concern.

### 4. Timestamp skew (thermal throttling, frame drop)

If the edge device drops frames under thermal load, timestamps become irregular.
TTC calculations remain correct per-frame but may miss the brief moment
when two entities were actually closest.

**Mitigation:** use hardware timestamping (camera PPS or NTP-sync) rather
than software timestamps from the processing thread.

---

## What the audit chain guarantees (and does not)

### Guarantees

- Every `RiskEvent` that was generated has not been modified, deleted,
  or reordered since sealing (BLAKE3 hash linkage + Ed25519 signature).
- The chain can be verified by any third party without operator involvement.

### Does not guarantee

- That every dangerous event was *detected* by the CV layer.
- That the camera was pointing at the right area.
- That the CV model was operating within its accuracy envelope.

The audit chain is a tamper-proof record of what edgesentry-rs was told.
It is not a proof of complete situational awareness.

---

## Phase 1 scope (POC)

Phase 1 uses CSV fixture replay (`eds ingest replay`).
The fixture represents pre-computed entity positions — equivalent to
what a CV adapter would produce after calibration.

No live camera integration exists in Phase 1.
The demo validates the physics engine, rule evaluation, and audit chain.

CV integration is a Phase 2 deliverable.
