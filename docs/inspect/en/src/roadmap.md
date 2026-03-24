# EdgeSentry-Inspect — Roadmap

## Foundation (trilink-core repo)

The following are prerequisites for all EdgeSentry-Inspect milestones.
They are tracked and implemented in the [`trilink-core`](https://github.com/edgesentry/trilink-core) repository.

| Issue | Deliverable | Status |
|---|---|---|
| [#30](https://github.com/edgesentry/trilink-core/issues/30) | `PointCloud`, `DepthMap`, `HeightMap` types | Todo |
| [#31](https://github.com/edgesentry/trilink-core/issues/31) | `project_to_depth_map` (3D → depth map) | Todo |
| [#32](https://github.com/edgesentry/trilink-core/issues/32) | `project_to_height_map` (3D → height map) | Todo |
| [#33](https://github.com/edgesentry/trilink-core/issues/33) | `docs/math.md` forward projection sections | Todo |
| [#34](https://github.com/edgesentry/trilink-core/issues/34) | Project → unproject round-trip tests | Todo |

Do not start M2 until #30, #31, #32, and #34 are merged.

---

## M2 — IFC Loader and Deviation Engine

**Goal:** Given a scanned point cloud and an IFC design file, compute a per-point deviation in millimetres.

**Deliverables:**

- `Cargo.toml` — workspace root; member: `crates/edgesentry-inspect`
- `src/ifc.rs` — load IFC geometry as `Vec<Point3D>` (design reference cloud)
- `src/deviation.rs` — k-d tree nearest-neighbour deviation; configurable threshold
- `src/report.rs` — JSON report serialisation (schema in [architecture.md](architecture.md))
- Integration test: load sample IFC fixture → compute deviation against known scan cloud → assert `compliant_pct`, `max_deviation_mm`, `mean_deviation_mm`

---

## M3 — Heatmap Rendering

**Goal:** Produce a PNG heatmap that maps per-point deviation to colour, positioned in 2D using the depth map projection.

**Deliverables:**

- `src/heatmap.rs` — deviation → RGB colour (green ≤ threshold, yellow 2×, red 4×+) → PNG via `image` crate
- Reuses `trilink-core::project_to_depth_map` to position each coloured point in 2D
- Integration test: known deviation values → verify expected pixel colours at expected positions in output PNG

---

## M4 — Field PC Pipeline (CLI)

**Goal:** End-to-end pipeline on the field PC from point cloud to deviation report, runnable as a single CLI command.

**Deliverables:**

- `src/main.rs` — CLI: `edgesentry-inspect scan --config config.toml`
- Wires: point cloud ingress (`trilink-core::FrameSource`) → `project_to_depth_map` → AI inference client → `unproject` → deviation → heatmap → report
- Config: IFC file path, `inference.mode` (`builtin` | `http`), inference endpoint URL (if `http`), deviation threshold, output directory
- End-to-end test with `MockSource` + mock inference server: report produced, all fields correct, heatmap PNG written

---

## M5 — Cloud Sync

**Goal:** Upload the deviation report and heatmap to the immutable audit store; emit structural-change flags.

**Deliverables:**

- `src/sync.rs` — S3-compatible upload (Object Lock WORM); structural-change flag → SQS or MQTT when anomaly exceeds 2× threshold
- Integration test: mock S3 + mock SQS → assert report uploaded, flag published for above-threshold anomaly, no flag for below-threshold

---

## M6 — Built-in Inference Model

**Goal:** Ship a lightweight ONNX defect-detection model with EdgeSentry-Inspect so that `inference.mode = "builtin"` works out of the box without an external server.

**Deliverables:**

- `src/inference/mod.rs` — `InferenceBackend` trait; dispatches to built-in or HTTP based on `inference.mode`
- `src/inference/builtin.rs` — ONNX Runtime runner (`ort` crate); loads bundled model weights
- `src/inference/http.rs` — HTTP client extracted from M4 into the same module for parity
- `models/detect.onnx` — initial model covering `surface_void`, `misalignment`, `rebar_exposure`
- Integration test: run built-in model on a sample depth map → assert detections are non-empty and class labels are valid

---

## Dependency graph

```
trilink-core #30, #31, #32, #34  (foundation — must be done first)
    └── M2 (IFC loader + deviation engine)
         └── M3 (heatmap rendering)
              └── M4 (field PC pipeline CLI)
                   ├── M5 (cloud sync)
                   └── M6 (built-in inference model)
```
