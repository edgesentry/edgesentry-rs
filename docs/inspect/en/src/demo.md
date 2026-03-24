# Demo Pipeline

This page describes how to build a self-contained proof-of-concept demonstration using open datasets and the Inspect pipeline. It is intended for use in technical evaluations and field demos before production data is available.

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

Use [IfcOpenShell](https://ifcopenshell.org/) to sample the IFC surface geometry into a reference point cloud (the "ground truth" design):

```python
import ifcopenshell
import ifcopenshell.geom
import numpy as np

settings = ifcopenshell.geom.settings()
model = ifcopenshell.open("design.ifc")

points = []
for product in model.by_type("IfcProduct"):
    try:
        shape = ifcopenshell.geom.create_shape(settings, product)
        verts = np.array(shape.geometry.verts).reshape(-1, 3)
        points.append(verts)
    except Exception:
        pass

design_cloud = np.vstack(points)  # shape: (N, 3)
```

### Step 2 — Simulate a damaged scan

Use [Open3D](https://www.open3d.org/) to introduce controlled deformations into a copy of the design cloud, producing a simulated "as-built" scan with known defects:

```python
import open3d as o3d
import numpy as np

pcd = o3d.geometry.PointCloud()
pcd.points = o3d.utility.Vector3dVector(design_cloud)

# Deform a region: push points inward by 15 mm
points = np.asarray(pcd.points)
mask = (points[:, 0] > 1.0) & (points[:, 0] < 1.5)
points[mask, 2] -= 0.015  # 15 mm depression

pcd.points = o3d.utility.Vector3dVector(points)
o3d.io.write_point_cloud("scan.ply", pcd)
```

### Step 3 — Compute deviation (M2)

The IFC deviation engine compares the simulated scan against the design cloud and produces a JSON report with per-point deviation in millimetres:

```
edgesentry-inspect scan \
  --ifc design.ifc \
  --scan scan.ply \
  --threshold-mm 5.0 \
  --out report.json
```

This step exercises `src/ifc.rs` and `src/deviation.rs` (M2).

### Step 4 — Project 3D → 2D (trilink-core)

`trilink-core::project_to_depth_map` converts the scan point cloud into a depth map image for AI inference input:

```
# Handled internally by the pipeline — no manual step required.
# The CLI calls project_to_depth_map with the configured camera intrinsics.
```

This step exercises `trilink-core::project_to_depth_map` (foundation #31).

### Step 5 — AI defect detection

Run a detection model over the depth map. For demos, use YOLOv8 via the HTTP inference path (`inference.mode = "http"`):

```python
from ultralytics import YOLO
import requests

model = YOLO("yolov8n.pt")  # or a fine-tuned defect model
results = model("depth_map.png")
# Forward detections to the Inspect HTTP inference endpoint
```

The CLI is pre-configured to receive detections from an HTTP server (`inference.mode = "http"`), so YOLOv8 running in Python connects without any Rust changes (M4).

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
| IFC deviation engine | Rust / `src/ifc.rs`, `src/deviation.rs` | M2 |
| 3D ↔ 2D projection | Rust / trilink-core | Foundation #31–#32 |
| AI defect detection | Python / YOLOv8 (HTTP) | M4 `inference.mode = "http"` |
| Report + heatmap | Rust / `src/report.rs`, `src/heatmap.rs` | M2–M3 |
