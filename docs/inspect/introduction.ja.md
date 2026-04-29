# EdgeSentry-Inspect

Real-time digital twin audit platform for infrastructure inspection.

- **Repository:** [github.com/edgesentry/edgesentry-rs](https://github.com/edgesentry/edgesentry-rs)
- **Documentation:** [edgesentry.github.io/edgesentry-rs/inspect/introduction/](https://edgesentry.github.io/edgesentry-rs/inspect/introduction/)

## What it does

EdgeSentry-Inspect detects construction and structural deviations at the field edge by fusing 3D point clouds with BIM design data — no cloud round-trip required during inspection.

```
3D sensor (LiDAR/ToF)
    │  point cloud
    ▼
trilink-core::project          ← 3D → 2D depth map / height map
    │  depth map (image)
    ▼
Vision AI inference            ← anomaly detection on local GPU
    │  bounding boxes + class
    ▼
trilink-core::unproject        ← 2D detections → 3D world coords
    │  world-space anomaly points
    ▼
Scan-vs-BIM engine             ← compare against IFC design geometry
    │  deviation heatmap + report
    ▼
Field display (tablet / AR)    ← inspector sees deviation on site
    │
    ▼  (upload report only — not the raw point cloud)
Cloud audit store              ← immutable evidence + digital twin update
```

## Why edge-first

The field PC handles everything from scan to deviation report. Only the report (JSON + PNG heatmap) is uploaded. This makes a 30-minute on-site inspection feasible even without a reliable cloud connection.

## Built on

- [`trilink-core`](https://github.com/edgesentry/trilink-core) — point cloud projection and spatial fusion (Rust)
- [`edgesentry-rs`](https://github.com/edgesentry/edgesentry-rs) — cryptographically verifiable audit records (optional, for high-assurance contexts)

## License

MIT OR Apache-2.0
