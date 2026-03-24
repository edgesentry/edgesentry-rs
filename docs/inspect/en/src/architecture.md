# EdgeSentry-Inspect — Architecture

## Edge-cloud split

```
┌──────────────────────────────────────────────────────────┐
│  FIELD PC (Edge)                                         │
│                                                          │
│  3D sensor (LiDAR / ToF)                                 │
│      │  point cloud (PointCloud)                         │
│      ▼                                                   │
│  trilink-core::project_to_depth_map                      │
│  trilink-core::project_to_height_map                     │
│      │  DepthMap  HeightMap                              │
│      ▼                                                   │
│  AI inference (built-in model or HTTP endpoint)         │
│      │  Vec<Detection>  (BBox2D + class + confidence)    │
│      ▼                                                   │
│  trilink-core::unproject                                 │
│      │  world-space Point3D per detection                │
│      ▼                                                   │
│  edgesentry-inspect::ifc      — IFC geometry             │
│  edgesentry-inspect::deviation — deviation (mm)          │
│  edgesentry-inspect::heatmap   — heatmap PNG             │
│  edgesentry-inspect::report    — JSON report             │
│      │                                                   │
│      ├── displayed on tablet / AR headset immediately    │
└──────┬───────────────────────────────────────────────────┘
       │  report JSON + heatmap PNG  (not raw point cloud)
       ▼
┌──────────────────────────────────────────────────────────┐
│  CLOUD (Audit Store / Digital Twin)                      │
│                                                          │
│  edgesentry-inspect::sync                                │
│      │  S3-compatible upload (Object Lock WORM)          │
│      │  structural-change flag → message queue           │
│      ▼                                                   │
│  Audit report store   — immutable evidence               │
│  Digital twin update  — as-built IFC delta               │
│  Central dashboard    — fleet-wide deviation trends      │
└──────────────────────────────────────────────────────────┘
```

### What runs on the field PC

| Step | Why edge |
|---|---|
| 3D → 2D projection | Point clouds are gigabytes; projecting locally avoids upload before verdict |
| AI inference | Sub-second latency; local GPU; works offline |
| 2D → 3D unprojection | Needed for on-site AR feedback |
| IFC load + deviation computation | Inspector must see deviation before leaving the site |
| Heatmap + report generation | Report is the upload artefact; must be ready on site |

### What goes to the cloud

| Data | Why cloud |
|---|---|
| Deviation report (JSON) | Immutable audit evidence; regulatory archive |
| Heatmap (PNG) | Human-readable evidence attached to the report |
| Structural-change flag | Real-time alert to central monitoring (UC-2) |
| As-built IFC delta | Persistent update to the digital twin asset model |

---

## Component design

### edgesentry-inspect::ifc

- Input: IFC file path (`.ifc`)
- Output: `Vec<Point3D>` — design reference point cloud sampled from wall/slab/column geometry
- Implementation: `ifcopenshell` via Python FFI (`pyo3`) or a native Rust IFC reader
- The reference cloud is loaded once per inspection session and cached in memory

### edgesentry-inspect::deviation

- Input: scan `Vec<Point3D>` (from `trilink-core::unproject`) + design `Vec<Point3D>` (from `ifc`)
- Output: per-scan-point deviation `f32` in metres
- Algorithm: k-d tree nearest-neighbour search (`kiddo` crate); O(n log m) per scan
- Threshold: configurable (default 10 mm for construction, 5 mm for maritime hull)

### edgesentry-inspect::heatmap

- Input: scan points + per-point deviation values
- Output: PNG image — deviation mapped to colour (green ≤ threshold, yellow 2×, red 4×+)
- Reuses `trilink-core::project_to_depth_map` to position coloured points in 2D

### edgesentry-inspect::report

JSON schema:

```json
{
  "capture_ts_us": 1711234567000000,
  "ifc_ref": "building-A-floor-3-v12.ifc",
  "scan_point_count": 142850,
  "compliant_pct": 94.2,
  "max_deviation_mm": 23.1,
  "mean_deviation_mm": 3.8,
  "anomalies": [
    {
      "world_pos": { "x": 12.3, "y": 4.1, "z": 2.05 },
      "deviation_mm": 23.1,
      "ai_class": "rebar_missing",
      "ai_confidence": 0.91
    }
  ]
}
```

### edgesentry-inspect::sync

- Uploads report JSON and heatmap PNG to an S3-compatible audit store (Object Lock WORM)
- Emits a structural-change flag to a message queue (SQS or MQTT) when any anomaly exceeds 2× the configured threshold
- Reuses the S3-compatible interface pattern from `edgesentry-rs`

---

## AI inference modes

EdgeSentry-Inspect supports two inference backends, selected by `inference.mode` in `config.toml`. Both produce the same `Vec<Detection>` output consumed by the rest of the pipeline.

### Built-in model (`inference.mode = "builtin"`)

A lightweight defect-detection model bundled with EdgeSentry-Inspect. Runs in-process via ONNX Runtime — no external server or network access required.

- Input: `DepthMap` + `HeightMap` images produced by `trilink-core`
- Output: `Vec<Detection>` — bounding boxes with class labels and confidence scores
- Initial class coverage: `surface_void`, `misalignment`, `rebar_exposure`
- Hardware: runs on a standard field PC CPU; no dedicated GPU required for basic use

Use `builtin` for getting started quickly, offline-only deployments, or when no vendor model is available.

### External HTTP endpoint (`inference.mode = "http"`)

The inference client POSTs the depth map and height map to `inference.base_url` and receives a detection list. The endpoint can be:

- A vendor's model server running locally on the field PC or robot (same host, no internet needed)
- A specialised cloud inference API (Scenario 1 / connected deployments only)

This mode is the integration point for vendor collaboration. Vendors implement the server side with their own model; EdgeSentry-Inspect calls it with a fixed schema. The operator sets `inference.base_url` in config — no code change required.

**Interface contract:**

```
POST /detect
Content-Type: multipart/form-data
  depth_map: <PNG bytes>
  height_map: <PNG bytes>

200 OK
[{"x":120,"y":45,"w":30,"h":20,"class":"surface_void","confidence":0.87}, ...]
```

| Mode | When to use |
|---|---|
| `builtin` | No vendor model; offline-only; getting started |
| `http` — local vendor server | Partner model on the same device; no internet needed |
| `http` — cloud API | Scenario 1 (connected); vendor hosts the model remotely |

---

## Optional: cryptographically verifiable audit records

If the inspection context requires **mathematically verifiable, tamper-evident audit records** — for example, regulatory submissions where a third party must independently verify that a report was not altered after the fact — the deviation report can be signed and hash-chained using [`edgesentry-rs`](https://github.com/edgesentry/edgesentry-rs).

`edgesentry-rs` provides:

| Capability | How it applies to EdgeSentry-Inspect |
|---|---|
| Ed25519 payload signing | The field PC signs each deviation report with a device key stored in a hardware secure element — proof that the report came from a specific sensor device |
| BLAKE3 hash chaining | Each report carries `prev_record_hash`, forming a chain — a missing or reordered report is immediately detectable |
| Sequence monotonicity | Report sequence numbers are strictly increasing — replay and deletion are cryptographically detectable |
| `IngestService::ingest()` | Cloud-side gate re-verifies signature and hash chain on upload — rejects tampered or out-of-sequence reports |

This layer is **opt-in**. For standard construction inspections, the S3 Object Lock WORM store (`edgesentry-inspect::sync`) is sufficient. For high-assurance contexts (maritime hull certification, legally binding structural sign-off), wrapping the report in an `edgesentry-rs` `AuditRecord` before upload provides a cryptographic audit trail that can be verified independently of EdgeSentry-Inspect infrastructure.

---

## Accuracy factors

Target accuracy is 10 mm for construction (UC-1) and 5 mm for maritime (UC-2). The following table shows the main factors that determine measurement accuracy in the field and how each is mitigated.

| Factor | Impact | Mitigation |
|---|---|---|
| 3D sensor accuracy | Primary driver | Use a sensor rated for the target accuracy at the required range |
| SLAM pose accuracy | Propagates into deviation computation | Loop closure at regular intervals; fiducial markers in featureless spaces |
| IFC alignment error | Shifts the entire deviation map | Use ≥ 3 known control points for IFC-to-SLAM registration; verify residuals < 2 mm. For consistent results regardless of operator, place fiducial markers (ArUco / AprilTag) at IFC-known coordinates before the inspection — the SLAM system detects them automatically and removes manual judgement from the registration step. |
| Projection round-trip error | Verified < 1 mm by `trilink-core` round-trip test (#34) | Arithmetic error is not a significant contributor |
| k-d tree resolution | Nearest-neighbour search accuracy | Design cloud sampled at ≤ 2 mm pitch (finer than the detection threshold) |

---

## Technology summary

| Component | Language | Key dependencies |
|---|---|---|
| `edgesentry-inspect` (deviation engine) | Rust | `trilink-core`, `kiddo` (k-d tree), `image` (PNG), `pyo3` (IFC via Python) |
| `edgesentry-inspect` (CLI) | Rust | `clap`, `tokio`, `reqwest` (inference client), `serde_json` |
| `edgesentry-inspect` (cloud sync) | Rust | S3-compatible HTTP client (reuse `edgesentry-rs` interface) |
| IFC geometry | Python (via `pyo3`) | `ifcopenshell` |
| AI inference — built-in | Rust + ONNX Runtime | Bundled lightweight defect detection model (`ort` crate) |
| AI inference — external | HTTP (`reqwest`) | Vendor endpoint: POST image → `Vec<BBox2D>`; local or cloud |
| Cloud audit store | AWS | S3 + Object Lock (WORM), SQS |

---

## Open datasets for PoC

| Domain | Dataset | Purpose |
|---|---|---|
| Construction | BIMNet (public IFC models) | Reference design geometry for scan-vs-BIM |
| Construction | ETH3D / S3DIS point clouds | Sample scan clouds for deviation testing |
| Maritime | MBES survey data | Hull scan point clouds |
| General | NYU Depth V2 | Depth map validation for projection correctness |
