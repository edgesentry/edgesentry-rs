# Inspect — Roadmap

## Release tracks

| Track | Scope | Audience |
|---|---|---|
| **OSS** | trilink-core (3D/2D projection, deviation engine), edgesentry-audit, edgesentry-inspect (CLI) | Developers, researchers |
| **Commercial — App** | Inspect App (Tauri/GUI), 3D heatmap, field photos, BIM integration UI | Site supervisors, inspectors |
| **Commercial — Reporting** | CONQUAS / MLIT-compliant automated report generation | Regulators, audit bodies |
| **Commercial — Partnership** | Advanced inference and sensor integration plugins for partner platforms | Partner companies |

Milestones marked **[OSS]** ship as open-source. Milestones marked **[Commercial]** are closed-source products built on top of the OSS layer.

## Ecosystem Strategy

Following the DuckDB model — keep algorithms, tools, and specifications as open as possible so that adoption spreads through the ecosystem rather than through lock-in.

**Why maximise the open core**

Publishing the deviation engine, projection algorithms, and CLI in full allows researchers, field engineers, regulators, and partner companies to verify, integrate, and extend independently. Transparency in the algorithms is itself the source of trust — it establishes Inspect as public infrastructure for construction inspection that no single vendor controls.

**Commercial layer sustains the open core**

Instant paperwork, regulation-compliant reports, and partner sensor integration are areas where a CLI alone falls short of what field operators need. The Inspect App, compliance report engine, and partner plugins cover these gaps and provide the revenue foundation that keeps the OSS development going. The commercial tier augments the OSS layer — it does not replace it.

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

## M4.5 — Inspect App (Visualisation Prototype) \[Commercial\] *(parallel to M5/M6)*

**Goal:** Interactive 3D heatmap viewer for field demos; runs alongside the CLI pipeline with no dependency on M5 or M6.

**Architecture (Python × JS hybrid / Tauri shell):**

| Layer | Technology | Role |
|---|---|---|
| Frontend | JavaScript / Three.js + Potree Core | Renders millions of points, Vertex Color heatmap, BIM integration UI |
| Backend (Sidecar) | Python / IfcOpenShell | Parses IFC geometry and per-element `GlobalId` attributes → JSON |
| Core Engine | Rust / trilink-core | Deviation calculation, coordinate transforms, audit hash signing |

**Deliverables:**

- Tauri desktop shell (Windows/macOS/Linux) — bundles the Python environment as a `sidecar`, distributed as a single executable
- Potree Core: efficient in-browser rendering of millions of point cloud points
- Python sidecar: IfcOpenShell extracts per-element `GlobalId` and attribute metadata from IFC → JSON
- Metadata overlay: clicking an element looks up attributes by `GlobalId` and displays them in a Tooltip/sidebar
- Vertex Color heatmap: deviation values mapped to RGB and applied directly to the Three.js mesh (real-time, not a PNG)
- Field photo viewer: site photographs displayed alongside the 3D view
- Consumes the JSON report produced by M4 — no changes to the Rust codebase required

> **Why now?** CLI output is not intuitive for site supervisors and inspectors. A visual demo accelerates proof-of-concept approval. This milestone runs in parallel and does not block M5 or M6.
>
> **Demo value:** Design data (BIM attributes) and as-built measurements (deviation) unified in 3D using only an OSS stack (Tauri + Python + Three.js) — no dependency on proprietary tools.

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

## M7 — Compliance Report Generation \[Commercial\]

**Goal:** Automatically generate PDF reports compliant with CONQUAS (Singapore) and MLIT (Japan) inspection standards from M4 deviation data.

**Background:** Implementing Singapore regulation (CLS / CONQUAS) compliance first means the OSS core is independently validated against globally recognised standards — accelerating international ecosystem adoption and providing third-party evidence of quality when entering the Japanese market.

**Deliverables:**

- CONQUAS-compliant report template — BCA (Building and Construction Authority) submission format
- MLIT-compliant report template — Japan construction quality management standard
- Report engine generating PDF from `report.json` and heatmap PNG
- Tamper-evident output with electronic signature via edgesentry-audit integration

**Prerequisites:** M4 (report JSON), M4.5 (Inspect App)

---

## M8 — Partner Sensor Integration Plugins \[Commercial\]

**Goal:** Integrate partner sensor and inference platforms directly with Inspect to enable advanced defect detection and specialised sensor data ingestion.

**Deliverables:**

- `plugins/<partner>/` — integration interface for partner AI inference engines (high-precision defect detection)
- Direct point cloud ingestion from partner sensor platforms
- Plugin API: extends the M6 `InferenceBackend` trait (symmetric with the built-in model)
- Plugin SDK documentation for partner onboarding

**Prerequisites:** M6 (`InferenceBackend` trait)

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
                   ├── M4.5 (Inspect App — Python×JS)     [Commercial, parallel]
                   │    └── M7 (compliance report gen.)   [Commercial]
                   ├── M5 (cloud sync)                    [OSS]
                   ├── M6 (built-in inference model)      [OSS]
                   │    └── M8 (partner sensor plugins)   [Commercial]
                   └── Demo Pipeline (open datasets + CLI)
```
