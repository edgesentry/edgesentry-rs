# edgesentry-inspect

Edge-first 3D scan vs. reference deviation detection.

## Input → Output
Point cloud (LiDAR/ToF) → deviation report (heatmap + world-coordinate anomaly list)

## Pipeline
```
point cloud → trilink-core::project → depth map / height map
           → AI inference (ONNX or HTTP endpoint) → bounding boxes
           → trilink-core::unproject → 3D world coords
           → compare vs. reference geometry → deviation report
           → edgesentry-audit → AuditRecord
```

## Key dependency: trilink-core
`PointCloud`, `DepthMap`, `HeightMap`, `project_to_depth_map`, `project_to_height_map`,
`unproject`, `PoseBuffer` — implemented in `edgesentry/trilink-core`.
Do not reimplement here.

## Roadmap
See [docs/roadmap/inspect.md](../../roadmap/inspect.md).
