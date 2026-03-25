# CLI Reference

`eds inspect` runs the M4 field PC pipeline: IFC reference + PLY scan → deviation → optional AI inference → heatmap + report.

---

## Installation

### For end users — pre-built binary

Download the latest release from the [GitHub Releases page](https://github.com/edgesentry/edgesentry-rs/releases).

| Platform | File |
|----------|------|
| Linux (x86-64) | `eds-{version}-x86_64-unknown-linux-gnu.tar.gz` |
| macOS (Apple Silicon) | `eds-{version}-aarch64-apple-darwin.tar.gz` |
| Windows (x86-64) | `eds-{version}-x86_64-pc-windows-msvc.zip` |

Extract and place the `eds` binary on your `PATH`:

```bash
# Linux / macOS
tar -xzf eds-{version}-{target}.tar.gz
sudo mv eds /usr/local/bin/
eds --help
```

```powershell
# Windows (PowerShell)
Expand-Archive eds-{version}-x86_64-pc-windows-msvc.zip
# Move eds.exe to a directory in your PATH
eds --help
```

### For developers — install from source

Requires [Rust](https://rustup.rs) (stable toolchain).

```bash
cargo install --git https://github.com/edgesentry/edgesentry-rs --bin eds
```

---

---

## `eds inspect scan`

Run a full scan pipeline from a TOML config file:

```bash
eds inspect scan --config config.toml
```

| Flag | Description |
|------|-------------|
| `-c`, `--config` | Path to the TOML configuration file (required) |

### Config file format

```toml
ifc_path  = "path/to/design.ifc"
scan_path = "path/to/scan.ply"

[camera]
fx = 525.0
fy = 525.0
cx = 319.5
cy = 239.5
width  = 640
height = 480

[inference]
mode = "off"          # "off" or "http"
# endpoint = "http://localhost:8000/infer"   # required when mode = "http"

[output]
dir = "out"
```

See [`config.example.toml`](../../../../crates/edgesentry-inspect/config.example.toml) for an annotated example.

---

## Output files

| File | Description |
|------|-------------|
| `out/report.json` | `compliant_pct`, `max_deviation_mm`, `mean_deviation_mm`, optional `detections` |
| `out/heatmap.png` | Per-point deviation heatmap (blue = compliant, red = exceeds threshold) |

---

## Inference modes

**`mode = "off"`** — deviation and heatmap only; no AI call.

**`mode = "http"`** — depth map is POSTed as a PNG to `endpoint`; the server must return a JSON array of bounding boxes:

```json
[{"x": 10, "y": 20, "w": 50, "h": 60}, ...]
```

Detected regions are back-projected to world coordinates via `trilink-core::unproject` and included in `report.json`.

---

## Building with optional features

The `eds inspect scan` command has no extra feature flags. Transport features (`transport-http`, `transport-tls`, etc.) apply only to `eds audit serve*` commands.

```bash
# default build — inspect scan works out of the box
cargo build -p eds

# with audit HTTP transport as well
cargo build -p eds --features transport-http
```
