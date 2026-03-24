# Inspect — Roadmap

## Release tracks

| Track | Scope | Audience |
|---|---|---|
| **OSS** | trilink-core, edgesentry-audit, edgesentry-inspect (CLI) | Developers, researchers |
| **Commercial** | Inspect App (GUI), immutable audit connector | Site supervisors, regulators, audit bodies |

Milestones marked **[OSS]** ship as open-source. Milestones marked **[Commercial]** are closed-source products built on top of the OSS layer.

---

## Foundation (trilink-core repo)

The following are prerequisites for all Inspect milestones.
They are tracked and implemented in the [`trilink-core`](https://github.com/edgesentry/trilink-core) repository.

| Issue | Deliverable | Status |
|---|---|---|
| [#30](https://github.com/edgesentry/trilink-core/issues/30) | `PointCloud`, `DepthMap`, `HeightMap` types | Done |
| [#31](https://github.com/edgesentry/trilink-core/issues/31) | `project_to_depth_map` (3D → depth map) | Done |
| [#32](https://github.com/edgesentry/trilink-core/issues/32) | `project_to_height_map` (3D → height map) | Todo |
| [#33](https://github.com/edgesentry/trilink-core/issues/33) | `docs/math.md` forward projection sections | Todo |
| [#34](https://github.com/edgesentry/trilink-core/issues/34) | Project → unproject round-trip tests | Todo |

Do not start M2 until #30, #31, #32, and #34 are merged.

---

## M2 — IFC Loader and Deviation Engine \[OSS\]

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

## M4.5 — Visualisation Prototype \[Commercial\] *(parallel to M5/M6)*

**Goal:** Interactive 3D heatmap viewer for field demos; runs alongside the CLI pipeline with no dependency on M5 or M6.

**Deliverables:**

- Tauri desktop shell (Windows/macOS/Linux) wrapping a Three.js WebGL renderer
- Loads the JSON report and PNG heatmap produced by M4; renders coloured deviation point cloud
- No Rust code changes required — consumes the existing M4 output files

> **Why now?** CLI output is not intuitive for site supervisors and inspectors. A visual demo accelerates proof-of-concept approval. This milestone runs in parallel and does not block M5 or M6.

---

## M5 — Cloud Sync \[OSS\]

**Goal:** Upload the deviation report and heatmap to an S3-compatible store; emit structural-change flags.

**Deliverables:**

- `src/sync.rs` — S3-compatible upload (standard PUT); structural-change flag → SQS or MQTT when anomaly exceeds 2× threshold
- Integration test: mock S3 + mock SQS → assert report uploaded, flag published for above-threshold anomaly, no flag for below-threshold

> **Commercial extension:** Object Lock (WORM) enforcement and API-based official certificate issuance are out of scope for the OSS release and are delivered as part of the commercial immutable audit connector.

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

## Audit layer — ISO 19650 \[planned\]

The current audit layer records a hash chain over inspection events. A planned extension will reframe each record as an **information container** in the sense of ISO 19650, adding structured BIM status transitions (WIP → Shared → Published) and a conformant payload schema. This enables interoperability with third-party BIM tools and positions the audit chain as a de-facto standard for construction inspection traceability.

This extension is tracked separately from the Inspect milestones above.

---

## Dependency graph

```
trilink-core #30, #31, #32, #34  (foundation — must be done first)
    └── M2 (IFC loader + deviation engine)          [OSS]
         └── M3 (heatmap rendering)                 [OSS]
              └── M4 (field PC pipeline CLI)         [OSS]
                   ├── M4.5 (visualisation)          [Commercial, parallel]
                   ├── M5 (cloud sync)               [OSS]
                   ├── M6 (built-in inference model) [OSS]
                   └── Demo Pipeline (open datasets + CLI)
```
