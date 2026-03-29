# Demo Pipeline

Two paths are available depending on whether you want a fully offline demo or a real IFC model with 3D mesh overlay.

---

## Path 1 — Fully offline (no downloads, no Python)

The fastest way to see the complete pipeline end-to-end. Everything runs locally with zero external dependencies.

```bash
# 1. Generate synthetic wall fixture (651-point 3 m × 2 m wall + 20 mm centre defect)
eds inspect generate-fixtures --dir ./demo

# 2. Run the scan pipeline
cd demo
eds inspect scan --config config.toml
```

Expected output:
```
compliant_pct    : 92.5%
max_deviation_mm : 20.000 mm
mean_deviation_mm: 2.680 mm
```

Output files written to `./demo/output/`:

| File | Contents |
|------|----------|
| `report.json` | Deviation statistics |
| `heatmap.png` | 2D colour map — green (compliant) → red (defect) |
| `points.json` | Per-point 3D positions + deviations for the viewer |

Open `./demo/output/` in the Inspect App viewer to see the coloured point cloud.

---

## Path 2 — Real IFC with 3D mesh overlay

Uses a real buildingSMART sample IFC and renders the IFC reference geometry as a blue wireframe in the viewer alongside the scan cloud.

### Prerequisites

- `uv` — `brew install uv` (manages Python and `ifcopenshell` automatically)

### Step 1 — Download sample IFC

```bash
eds inspect download-samples --dir ./ifc-samples
```

Downloads `Building-Architecture.ifc` (~220 KB, IFC 4 PCERT sample) from buildingSMART. Skipped if already present.

### Step 2 — Extract IFC mesh

```bash
eds inspect extract-mesh \
    --ifc ./ifc-samples/Building-Architecture.ifc \
    --out ./ifc-samples/reference.json
```

On first run, `uv` downloads Python and installs `ifcopenshell` automatically (cached at `~/.cache/uv/`). Subsequent calls are instant.

Output: `reference.json` — vertices and triangle faces in world coordinates.

### Step 3 — Generate a demo scan

```bash
eds inspect generate-fixtures --dir ./demo
```

This provides a PLY scan and a pre-configured `config.toml`. For a real scan, replace `wall_slab_scan.ply` with your own PLY file.

### Step 4 — Add `mesh_path` to config

```bash
echo 'mesh_path = "../ifc-samples/reference.json"' >> ./demo/config.toml
```

### Step 5 — Run the pipeline

```bash
cd demo
eds inspect scan --config config.toml
```

`reference.json` is copied to `./demo/output/reference.json` alongside `points.json`.

### Step 6 — View in the Inspect App

Open `./demo/output/` in the Inspect App viewer. The IFC reference mesh renders as a semi-transparent blue wireframe over the coloured scan point cloud. Use the **Reference mesh** toggle in the sidebar to show or hide it.

---

## Tech stack summary

| Component | Implementation | Command |
|-----------|---------------|---------|
| Synthetic fixture | Rust (built-in) | `eds inspect generate-fixtures` |
| IFC sample download | Rust + ureq | `eds inspect download-samples` |
| IFC mesh extraction | Python / IfcOpenShell (via `uv run`) | `eds inspect extract-mesh` |
| Deviation engine | Rust / `deviation.rs` | `eds inspect scan` |
| 3D ↔ 2D projection | Rust / trilink-core | automatic in `scan` |
| AI defect detection | External HTTP server | `inference.mode = "http"` |
| Heatmap + report | Rust / `heatmap.rs`, `report.rs` | automatic in `scan` |
| 3D viewer | Three.js (Inspect App) | open output folder in app |
