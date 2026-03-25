# Demo Pipeline

## Try it now — no CLI required

M2 (IFC loader + deviation engine) and M3 (heatmap rendering) are already implemented. You can run a self-contained end-to-end demo against the bundled sample IFC fixture with a single command — no external data, no external tools needed:

```sh
cargo run -p edgesentry-inspect --example ifc_deviation_heatmap
```

**What it does:**

1. Loads `crates/edgesentry-inspect/tests/fixtures/sample.ifc` (7 reference points)
2. Simulates a scan by displacing one point by 15 mm along Z
3. Computes deviation — prints `compliant_pct`, `max_deviation_mm`, `mean_deviation_mm`
4. Projects points to 2D and renders a colour heatmap PNG
5. Writes the PNG to `/tmp/edgesentry_heatmap.png` — open it in any image viewer

**Source:** [`crates/edgesentry-inspect/examples/ifc_deviation_heatmap.rs`](../../../../crates/edgesentry-inspect/examples/ifc_deviation_heatmap.rs)

> The full pipeline below (Steps 1–6) requires the M4 CLI (`edgesentry-inspect scan`) which is not yet implemented. The example above is the recommended way to test M2 and M3 locally today.

---

## Open datasets

| Asset | Source | Notes |
|---|---|---|
| IFC design model | [buildingSMART BIMNet gallery](https://awards.buildingsmart.org/gallery/) | Publicly shared IFC files from BIM award entries |
| 3D point cloud | [S3DIS (Stanford Large-Area Indoor Spaces)](https://www.open3d.org/docs/latest/python_api/open3d.ml.tf.datasets.S3DIS.html) | Indoor LiDAR scans of real buildings; well-suited for structural inspection scenarios |

> Verify any IFC download URL before use. The buildingSMART gallery is the authoritative source; third-party mirrors may serve modified files.

---

## Pipeline steps

### Step 1 — Generate design point cloud from IFC

Use [IfcOpenShell](https://ifcopenshell.org/) to sample the IFC surface geometry into a reference point cloud (the "ground truth" design). Each `IfcProduct` element is triangulated and its vertices collected into a flat `(N, 3)` array representing the design surface.

### Step 2 — Simulate a damaged scan

Use [Open3D](https://www.open3d.org/) to introduce controlled deformations into a copy of the design cloud, producing a simulated "as-built" scan with known defects. A representative demo deforms a localised region by 15 mm to simulate a surface depression, then saves the result as a PLY file.

### Step 3 — Compute deviation (M2)

Run the `edgesentry-inspect scan` CLI command, pointing it at the IFC design file and the simulated scan PLY. The CLI calls `src/ifc.rs` to load the design reference cloud, then `src/deviation.rs` to compute per-point nearest-neighbour deviation and emit a JSON report containing `compliant_pct`, `max_deviation_mm`, and `mean_deviation_mm`.

This step exercises `src/ifc.rs` and `src/deviation.rs` (M2).

### Step 4 — Project 3D → 2D (trilink-core)

`trilink-core::project_to_depth_map` converts the scan point cloud into a depth map image for AI inference input. This is handled automatically by the CLI using the camera intrinsics in `config.toml` — no manual step is required.

This step exercises `trilink-core::project_to_depth_map` (foundation #31).

### Step 5 — AI defect detection

A detection model runs over the depth map via the HTTP inference path (`inference.mode = "http"`). For demos, YOLOv8 can be used as the external inference server. The CLI sends the depth map image to the configured HTTP endpoint and receives bounding-box detections in return (M4).

### Step 6 — Back-project 2D → 3D

Detected 2D bounding boxes are back-projected to world coordinates using `trilink-core::unproject`, then overlaid on the 3D model and included in the deviation report (M4).

---

## Deviation engine in the demo

The deviation engine (M2) is the quantitative centrepiece of the demo. It answers the question *"by how many millimetres does the as-built structure deviate from the IFC design?"* — not just *"is there an anomaly?"*. Make sure Step 3 is demonstrated explicitly, as it differentiates this pipeline from a generic defect detector.

---

## Tech stack summary

| Component | Language / Library | Roadmap milestone |
|---|---|---|
| IFC surface sampling | Python / IfcOpenShell | Demo setup (pre-M2) |
| Damage simulation | Python / Open3D | Demo setup only |
| IFC deviation engine | Rust CLI / `src/ifc.rs`, `src/deviation.rs` | M2 |
| 3D ↔ 2D projection | Rust / trilink-core | Foundation #31–#32 |
| AI defect detection | External HTTP server (e.g. YOLOv8) | M4 `inference.mode = "http"` |
| Report + heatmap | Rust CLI / `src/report.rs`, `src/heatmap.rs` | M2–M3 |
