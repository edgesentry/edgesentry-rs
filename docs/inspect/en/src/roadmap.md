# Inspect — Roadmap

## Release tracks

| Track | Scope | Audience |
|---|---|---|
| **OSS** (this repo) | trilink-core (3D/2D projection, deviation engine), edgesentry-audit, edgesentry-inspect (CLI) | Developers, researchers |
| **Commercial** ([edgesentry-app](https://github.com/edgesentry/edgesentry-app)) | Inspect App (Tauri/GUI), compliance reports, partner sensor plugins | Site supervisors, inspectors, regulators |

All milestones in this document ship as open-source. Commercial milestones are tracked in [edgesentry-app](https://github.com/edgesentry/edgesentry-app).

## Ecosystem Strategy

Following the DuckDB model — keep algorithms, tools, and specifications as open as possible so that adoption spreads through the ecosystem rather than through lock-in.

**Why maximise the open core**

Publishing the deviation engine, projection algorithms, and CLI in full allows researchers, field engineers, regulators, and partner companies to verify, integrate, and extend independently. Transparency in the algorithms is itself the source of trust — it establishes Inspect as public infrastructure for construction inspection that no single vendor controls.

**Co-creating standards with regulators**

Regulators — BCA, CSA, MLIT — are partners in building construction quality standards, not gatekeepers to route around. Implementing CLS / JC-STAR / CONQUAS compliance up front is a commitment to taking those standards seriously, and an invitation for independent third-party validation of the OSS core's quality. That trust relationship accelerates international ecosystem adoption.

---

## Foundation (trilink-core repo)

The following are prerequisites for all Inspect milestones.
They are tracked and implemented in the [`trilink-core`](https://github.com/edgesentry/trilink-core) repository.

| Issue | Deliverable | Status |
|---|---|---|
| [#30](https://github.com/edgesentry/trilink-core/issues/30) | `PointCloud`, `DepthMap`, `HeightMap` types | Done |
| [#31](https://github.com/edgesentry/trilink-core/issues/31) | `project_to_depth_map` (3D → depth map) | Done |
| [#32](https://github.com/edgesentry/trilink-core/issues/32) | `project_to_height_map` (3D → height map) | Done |
| [#33](https://github.com/edgesentry/trilink-core/issues/33) | `docs/math.md` forward projection sections | Done |
| [#34](https://github.com/edgesentry/trilink-core/issues/34) | Project → unproject round-trip tests | Done |
| [#39](https://github.com/edgesentry/trilink-core/issues/39) | `HeightMap` dimension naming (`cols/rows` → `width/height`) | Done |
| [#40](https://github.com/edgesentry/trilink-core/issues/40) | Coordinate precision decision (`Point3D` stays `f32`) | Done |
| [#38](https://github.com/edgesentry/trilink-core/issues/38) | Adopt glam for `Transform4x4` / `Point3D` (SIMD, inversion) | Done |

All foundation items are merged. M2 is also complete. M3 is unblocked.

---

## M2 — IFC Loader and Deviation Engine \[OSS\] ✅ Implemented

**Goal:** Given a scanned point cloud and an IFC design file, compute a per-point deviation in millimetres.

**Deliverables:**

- `Cargo.toml` — workspace root; member: `crates/edgesentry-inspect`
- `src/ifc.rs` — load IFC geometry as `Vec<Point3D>` (design reference cloud)
- `src/deviation.rs` — k-d tree nearest-neighbour deviation; configurable threshold
- `src/report.rs` — JSON report serialisation (schema in [architecture.md](architecture.md))
- Integration test: load sample IFC fixture → compute deviation against known scan cloud → assert `compliant_pct`, `max_deviation_mm`, `mean_deviation_mm`

---

## M3 — Heatmap Rendering \[OSS\]

**Goal:** Produce a PNG heatmap that maps per-point deviation to colour, positioned in 2D using the depth map projection.

**Deliverables:**

- `src/heatmap.rs` — deviation → RGB colour (green ≤ threshold, yellow 2×, red 4×+) → PNG via `image` crate
- Reuses `trilink-core::project_to_depth_map` to position each coloured point in 2D
- Integration test: known deviation values → verify expected pixel colours at expected positions in output PNG

---

## M4 — Field PC Pipeline (CLI) \[OSS\]

**Goal:** End-to-end pipeline on the field PC from point cloud to deviation report, runnable as a single CLI command.

**Deliverables:**

- `src/main.rs` — CLI: `edgesentry-inspect scan --config config.toml`
- Wires: point cloud ingress (`trilink-core::FrameSource`) → `project_to_depth_map` → AI inference client → `unproject` → deviation → heatmap → report
- Config: IFC file path, `inference.mode` (`builtin` | `http`), inference endpoint URL (if `http`), deviation threshold, output directory
- End-to-end test with `MockSource` + mock inference server: report produced, all fields correct, heatmap PNG written

---

## M5 — Cloud Sync \[OSS\]

**Goal:** Upload the deviation report and heatmap to an S3-compatible store; emit structural-change flags.

**Deliverables:**

- `src/sync.rs` — S3-compatible upload (standard PUT); structural-change flag → SQS or MQTT when anomaly exceeds 2× threshold
- Integration test: mock S3 + mock SQS → assert report uploaded, flag published for above-threshold anomaly, no flag for below-threshold

---

## M6 — Built-in Inference Model \[OSS\]

**Goal:** Ship a lightweight ONNX defect-detection model with Inspect so that `inference.mode = "builtin"` works out of the box without an external server.

**Deliverables:**

- `src/inference/mod.rs` — `InferenceBackend` trait; dispatches to built-in or HTTP based on `inference.mode`
- `src/inference/builtin.rs` — ONNX Runtime runner (`ort` crate); loads bundled model weights
- `src/inference/http.rs` — HTTP client extracted from M4 into the same module for parity
- `models/detect.onnx` — initial model covering `surface_void`, `misalignment`, `rebar_exposure`
- Integration test: run built-in model on a sample depth map → assert detections are non-empty and class labels are valid

---

## Known Limitations

The following constraints are inherent to the current design. They are documented in full in [`trilink-core/docs/limitations.md`](https://github.com/edgesentry/trilink-core/blob/main/docs/limitations.md).

| # | Limitation | Affected milestone | Workaround |
|---|---|---|---|
| L1 | **Single-viewpoint occlusion** — Z-buffer projection discards surfaces not visible from the capture pose | M3, M4 | Fuse multiple poses before projection; monitor NaN fraction in depth map |
| L2 | **Height map is protrusion-only** — maximum-Z aggregation misses depressions (spalling, section loss) | M3 | Use deviation engine (M2) for depression detection; height map is supplementary |
| L3 | **Curved-surface back-projection bias** — unproject assumes a flat plane; ~11.7% relative error on cylinders/arches vs ~2.5% on flat surfaces | M4, M6 | Flag detections on high-curvature regions; apply expanded tolerances |
| L4 | **f32 precision outside local frame** — coordinates must be in a local tangent-plane frame; UTM/WGS-84 input silently degrades to ~12 mm steps | Foundation | Subtract site origin before constructing `Point3D`; see `trilink-core/docs/math.md` |
| L5 | **Depth-only inference** — built-in ONNX model uses depth map only; no RGB channel; ~76% F1 vs ~87% achievable with RGB-D fusion | M6 | Planned RGB-D extension to `InferenceBackend`; `FusionPacket.image_jpeg` already available |
| L6 | **Fallback depth degrades localisation** — `fallback_depth_m = 2.0 m` when no sensor reading; position error ∝ `|true_depth − 2.0|` | M4 | Always co-register a range sensor; treat fallback detections as positional annotations only |
| L7 | **Pose buffer dead zone** — inference results arriving >200 ms after capture, or after >33 s buffer window, are silently dropped | Foundation | Monitor `world_pos = None` rate; log warn on tolerance vs. buffer-exhausted failures |
| L8 | **Not yet near-visual-inspection equivalent** — no documented MLIT/CONQUAS equivalence test; no IFC 4.3 metadata write-back in OSS layer yet | — | Addressed by the commercial compliance layer in [edgesentry-app](https://github.com/edgesentry/edgesentry-app) |

### RGB-D Fusion (M6 enhancement)

The built-in inference model (M6) will be extended to accept an optional RGB channel alongside the depth map, forming an RGB-D input tensor. The `FusionPacket` already carries `image_jpeg`; the main change is in the inference module and model retraining.

Published benchmarks on concrete infrastructure damage detection show the impact:

| Input | F1 |
|---|---|
| 2D RGB only | 67.6% |
| 3D depth only | 76.0% |
| RGB-D fused | **86.7%** |

This item is tracked as part of M6. It does not change the `InferenceBackend` trait signature — the RGB tensor is passed as an optional additional channel.

---

## Demo Pipeline

**Goal:** Run a fully self-contained end-to-end demonstration of the Inspect CLI using open datasets — no production hardware or data required.

**Prerequisites:** M2, M3, M4 (CLI must be built and on PATH).

**Steps:**

1. Download a public IFC file (buildingSMART BIMNet gallery) and an indoor LiDAR scan (S3DIS dataset).
2. Use IfcOpenShell to sample the IFC surface into a reference point cloud.
3. Use Open3D to introduce a controlled 15 mm deformation, producing a simulated scan with a known defect.
4. Run `edgesentry-inspect scan --config config.toml` — the CLI loads the IFC, computes deviation, projects to a depth map, calls the HTTP inference server, back-projects detections, renders a heatmap, and writes the JSON report.
5. Inspect `report.json` (`compliant_pct`, `max_deviation_mm`, `mean_deviation_mm`) and the PNG heatmap to verify the simulated defect is detected and quantified.

See [Demo Pipeline](demo.md) for the full walkthrough.

---

## Audit layer — ISO 19650

The ISO 19650 information container schema (BIM status transitions, conformant payload, third-party BIM tool interoperability) is implemented in the edgesentry-rs crate, not here.

See **[edgesentry-audit roadmap — Milestone 2.7](../../audit/en/src/roadmap.md)** for the implementation plan.

---

## Dependency graph

```
trilink-core #30, #31, #32, #33, #34  (foundation — complete)
    └── M2 (IFC loader + deviation engine)               [OSS]
         └── M3 (heatmap rendering)                      [OSS]
              └── M4 (field PC pipeline CLI)              [OSS]
                   ├── M5 (cloud sync)                    [OSS]
                   ├── M6 (built-in inference model)      [OSS]
                   └── Demo Pipeline (open datasets + CLI)

Commercial milestones (M4.5, M7, M8) → edgesentry-app
Phase 2 (2D/MPA/JTC, 1D/NEA/PUB) → edgesentry-app Phase 2 roadmap
```

The Phase 2 expansion (YOLO11/SAM 2 for 2D maritime/industrial, PatchTST/iTransformer for 1D time-series) is planned in the [edgesentry-app inspect roadmap](https://github.com/edgesentry/edgesentry-app/blob/main/docs/inspect/roadmap.md). Development priority is 3D demo first, then 2D, then 1D.
