# CLI Reference

`eds inspect` runs the field PC pipeline: IFC reference + PLY scan → deviation → optional AI inference → heatmap + report + optional 3D mesh overlay.

---

## Installation

### For end users — Homebrew (macOS / Linux)

```bash
brew install edgesentry/tap/eds
```

`uv` is installed automatically as a Homebrew dependency — no separate Python install required.

### For end users — pre-built binary

Download the latest release from the [GitHub Releases page](https://github.com/edgesentry/edgesentry-rs/releases).

| Platform | File |
|----------|------|
| Linux (x86-64) | `eds-{version}-x86_64-unknown-linux-gnu.tar.gz` |
| macOS (Apple Silicon) | `eds-{version}-aarch64-apple-darwin.tar.gz` |
| Windows (x86-64) | `eds-{version}-x86_64-pc-windows-msvc.zip` |

```bash
# Linux / macOS
tar -xzf eds-{version}-{target}.tar.gz
sudo mv eds /usr/local/bin/
eds --help
```

### For developers — install from source

Requires [Rust](https://rustup.rs) (stable toolchain).

```bash
cargo install --git https://github.com/edgesentry/edgesentry-rs --locked --bin eds
```

---

## `eds inspect generate-fixtures`

Generate offline demo data — no external dependencies required:

```bash
eds inspect generate-fixtures --dir ./demo-data
```

| Flag | Description |
|------|-------------|
| `-d`, `--dir` | Output directory (created if absent, default: `demo-data`) |

Creates three files in `<dir>`:

| File | Contents |
|------|----------|
| `wall_slab.ifc` | 651 `IFCCARTESIANPOINT` entries — flat 3 m × 2 m wall |
| `wall_slab_scan.ply` | Same grid with a 20 mm outward bulge in the centre (49 non-compliant points) |
| `config.toml` | Pre-configured for `eds inspect scan` |

Then run the full pipeline:

```bash
cd demo-data && eds inspect scan --config config.toml
```

---

## `eds inspect download-samples`

Download a buildingSMART sample IFC file for offline use:

```bash
eds inspect download-samples --dir ./ifc-samples
```

| Flag | Description |
|------|-------------|
| `-d`, `--dir` | Output directory (created if absent, default: `ifc-samples`) |

Downloads `Building-Architecture.ifc` (~220 KB, IFC 4, PCERT sample scene) from the buildingSMART Sample-Test-Files repository. Files already present are skipped.

---

## `eds inspect extract-mesh`

Extract triangulated mesh geometry from an IFC file:

```bash
eds inspect extract-mesh \
    --ifc ./ifc-samples/Building-Architecture.ifc \
    --out ./ifc-samples/reference.json
```

| Flag | Description |
|------|-------------|
| `--ifc` | Input IFC file |
| `--out` | Output `reference.json` path |

**Prerequisite:** `uv` on PATH (`brew install uv`). No Python install required — `uv` manages Python and `ifcopenshell` automatically on first run (cached for subsequent calls).

The IfcOpenShell extraction script is embedded inside the `eds` binary. On first call, `uv` downloads Python and installs `ifcopenshell` into a local cache (`~/.cache/uv/`). Subsequent calls are instant.

### Output format (`reference.json`)

```json
{
  "vertices": [[x, y, z], ...],
  "faces":    [[i, j, k], ...]
}
```

Coordinates are in metres (world coordinate system). Pass the output path as `mesh_path` in `config.toml` to include it in scan output for the viewer.

---

## `eds inspect scan`

Run a full scan pipeline from a TOML config file:

```bash
eds inspect scan --config config.toml
```

| Flag | Description |
|------|-------------|
| `-c`, `--config` | Path to the TOML configuration file (default: `config.toml`) |

### Config file format

```toml
ifc_path  = "path/to/design.ifc"
scan_path = "path/to/scan.ply"

# Optional: include a pre-extracted mesh so the viewer renders the IFC
# reference as a blue wireframe alongside the scan cloud.
# mesh_path = "path/to/reference.json"

[camera]
fx = 525.0
fy = 525.0
cx = 319.5
cy = 239.5
width  = 640
height = 480

[inference]
mode = "off"          # "off", "mock", or "http"
# endpoint = "http://localhost:8000/infer"   # required when mode = "http"

[output]
dir          = "out"
threshold_mm = 10.0
```

See [`config.example.toml`](../../../../crates/edgesentry-inspect/config.example.toml) for a fully annotated example.

---

## Output files

| File | Description |
|------|-------------|
| `out/report.json` | `compliant_pct`, `max_deviation_mm`, `mean_deviation_mm`, optional `detections` |
| `out/heatmap.png` | Per-point deviation heatmap (green = compliant, red = exceeds threshold) |
| `out/points.json` | Per-point 3D positions and deviation values for the viewer |
| `out/reference.json` | Copied from `mesh_path` when set — IFC mesh for the viewer wireframe |

---

## Inference modes

**`mode = "off"`** — deviation and heatmap only; no AI call.

**`mode = "mock"`** — built-in hardcoded detections for the synthetic wall fixture. No external server required. Use this to demonstrate the full AI pipeline (depth map → orange spheres in viewer) without a production model.

**`mode = "http"`** — depth map is POSTed as a PNG to `endpoint` (third-party model, e.g. YOLOv8); the server must return a JSON array of bounding boxes:

```json
[{"u0": 10, "v0": 20, "u1": 60, "v1": 80}, ...]
```

Detected regions are back-projected to world coordinates via `trilink-core::unproject` and included in `report.json`.

---

## Building with optional features

The `eds inspect` commands have no extra feature flags. Transport features (`transport-http`, `transport-tls`, etc.) apply only to `eds audit serve*` commands.

```bash
# default build — all inspect commands work out of the box
cargo build -p eds

# with audit HTTP transport as well
cargo build -p eds --features transport-http
```
