//! End-to-end scan pipeline.
//!
//! [`run_scan`] wires together all M2–M4 components:
//! IFC loader → deviation engine → depth-map projection → AI inference →
//! unproject detections → heatmap → JSON report.

use std::path::PathBuf;

use trilink_core::{
    BBox2D, CameraIntrinsics, Point3D, PointCloud, Transform4x4,
    bridge::{project_to_depth_map, unproject},
};

use crate::{
    config::{InferenceMode, ScanConfig},
    deviation::{compute_deviation, per_point_deviations_mm, DeviationReport},
    heatmap::{render_heatmap, write_heatmap_png},
    ifc::load_ifc_points,
    inference::{depth_map_to_png, http_infer, mock_infer, InferenceError},
    ply::{load_ply_points, PlyError},
    points::{write_points, PointsError, PointsJson},
    report::{write_report, ReportError},
};

/// Outputs produced by a successful scan run.
#[derive(Debug)]
pub struct ScanOutput {
    /// Deviation summary statistics.
    pub report: DeviationReport,
    /// Absolute path to the written `report.json`.
    pub report_path: PathBuf,
    /// Absolute path to the written `heatmap.png`.
    pub heatmap_path: PathBuf,
    /// Absolute path to the written `points.json`.
    pub points_path: PathBuf,
    /// Absolute path to the copied `reference.json`, if `mesh_path` was set in config.
    pub reference_mesh_path: Option<PathBuf>,
    /// Number of AI detections back-projected to 3D (0 when `mode = "off"`).
    pub detection_count: usize,
    /// 3D world positions for each detection (parallel to `detection_count`).
    pub world_detections: Vec<Point3D>,
}

/// Errors that can occur during a scan run.
#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("IFC load failed: {0}")]
    Ifc(String),
    #[error("PLY load failed: {0}")]
    Ply(#[from] PlyError),
    #[error("inference error: {0}")]
    Inference(#[from] InferenceError),
    #[error("config error: {0}")]
    Config(String),
    #[error("report write failed: {0}")]
    Report(#[from] ReportError),
    #[error("heatmap write failed: {0}")]
    Heatmap(String),
    #[error("points write failed: {0}")]
    Points(#[from] PointsError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Run the full M4 scan pipeline from a [`ScanConfig`].
///
/// # Pipeline steps
///
/// 1. Load IFC design file → reference point cloud
/// 2. Load PLY scan file → as-built point cloud
/// 3. Build camera model from config
/// 4. Project scan to depth map (pinhole)
/// 5. Optionally POST depth map to HTTP inference server → bounding boxes
/// 6. Unproject each detection bbox to 3D world coordinates
/// 7. Compute per-point deviation (scan vs. IFC reference)
/// 8. Render deviation heatmap PNG
/// 9. Write `report.json`, `heatmap.png`, `points.json` (and optionally `reference.json`)
pub fn run_scan(config: &ScanConfig) -> Result<ScanOutput, ScanError> {
    // Step 1 — IFC reference cloud
    let reference =
        load_ifc_points(&config.ifc_path).map_err(|e| ScanError::Ifc(e.to_string()))?;

    // Step 2 — Scan point cloud
    let scan = load_ply_points(&config.scan_path)?;

    // Step 3 — Camera model (identity pose: sensor frame == world frame for this run)
    let k = CameraIntrinsics {
        fx: config.camera.fx,
        fy: config.camera.fy,
        cx: config.camera.cx,
        cy: config.camera.cy,
    };
    let pose = Transform4x4::identity();

    // Step 4 — Depth map
    let cloud = PointCloud { capture_ts_us: 0, points: scan.clone(), intensities: None };
    let depth_map = project_to_depth_map(&cloud, &pose, &k, config.camera.width, config.camera.height);

    // Step 5 — AI inference (optional)
    let detections: Vec<BBox2D> = match &config.inference.mode {
        InferenceMode::Off => vec![],
        InferenceMode::Mock => mock_infer(),
        InferenceMode::Http => {
            let endpoint = config.inference.endpoint.as_deref().ok_or_else(|| {
                ScanError::Config(
                    "inference.endpoint is required when inference.mode = \"http\"".into(),
                )
            })?;
            let png = depth_map_to_png(&depth_map)?;
            http_infer(endpoint, &png)?
        }
    };

    // Step 6 — Unproject detections to 3D world positions
    let world_detections: Vec<Point3D> = detections
        .iter()
        .map(|bbox| unproject(bbox, None, config.inference.fallback_depth_m, &k, &pose))
        .collect();

    // Step 7 — Deviation (per-point + summary)
    let deviations_mm = per_point_deviations_mm(&scan, &reference);
    let report = compute_deviation(&scan, &reference, config.output.threshold_mm);

    // Step 8 — Heatmap
    let img = render_heatmap(
        &scan,
        &deviations_mm,
        &pose,
        &k,
        config.camera.width,
        config.camera.height,
        config.output.threshold_mm,
    );

    // Step 9 — Write outputs
    std::fs::create_dir_all(&config.output.dir)?;
    let report_path = config.output.dir.join("report.json");
    let heatmap_path = config.output.dir.join("heatmap.png");
    let points_path = config.output.dir.join("points.json");

    write_report(&report, &report_path)?;
    write_heatmap_png(&img, &heatmap_path)
        .map_err(|e| ScanError::Heatmap(e.to_string()))?;
    let points_json = PointsJson::new(&scan, &deviations_mm, &world_detections);
    write_points(&points_json, &points_path)?;

    // Optional: copy reference mesh to output directory so the viewer finds it
    let reference_mesh_path = if let Some(src) = &config.mesh_path {
        let dest = config.output.dir.join("reference.json");
        std::fs::copy(src, &dest)?;
        Some(dest)
    } else {
        None
    };

    Ok(ScanOutput {
        report,
        report_path,
        heatmap_path,
        points_path,
        reference_mesh_path,
        detection_count: detections.len(),
        world_detections,
    })
}
