//! Per-point output for the M4.5 Inspect App viewer.
//!
//! [`PointsJson`] is written to `points.json` alongside `report.json` and
//! `heatmap.png` by [`crate::pipeline::run_scan`]. The M4.5 Tauri viewer
//! reads this file to render the coloured deviation point cloud in Three.js.

use std::path::Path;

use serde::{Deserialize, Serialize};
use trilink_core::Point3D;

/// One scan point with its deviation value, as serialised to `points.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanPoint {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    /// Nearest-neighbour distance to the design reference cloud, in millimetres.
    pub deviation_mm: f64,
}

/// One AI-detected defect location, as serialised to `points.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionPoint {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Per-point output written to `points.json` by the M4 scan pipeline.
///
/// Consumed by the M4.5 Inspect App viewer for 3D rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointsJson {
    /// Per-point 3D scan positions with deviation values.
    pub scan_points: Vec<ScanPoint>,
    /// 3D world positions of AI-detected defects (empty when inference is off).
    pub detections: Vec<DetectionPoint>,
}

impl PointsJson {
    /// Build a [`PointsJson`] from parallel scan data.
    ///
    /// `scan` and `deviations_mm` must be the same length.
    pub fn new(scan: &[Point3D], deviations_mm: &[f64], world_detections: &[Point3D]) -> Self {
        assert_eq!(scan.len(), deviations_mm.len());
        Self {
            scan_points: scan
                .iter()
                .zip(deviations_mm)
                .map(|(p, &d)| ScanPoint { x: p.x, y: p.y, z: p.z, deviation_mm: d })
                .collect(),
            detections: world_detections
                .iter()
                .map(|p| DetectionPoint { x: p.x, y: p.y, z: p.z })
                .collect(),
        }
    }
}

/// Errors that can occur while writing the points file.
#[derive(Debug, thiserror::Error)]
pub enum PointsError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON serialisation error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Serialise a [`PointsJson`] to `path`.
pub fn write_points(points: &PointsJson, path: &Path) -> Result<(), PointsError> {
    let json = serde_json::to_string(points)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Deserialise a [`PointsJson`] from `path`.
pub fn read_points(path: &Path) -> Result<PointsJson, PointsError> {
    let content = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use trilink_core::Point3D;

    fn make_points() -> PointsJson {
        PointsJson::new(
            &[
                Point3D { x: 1.0, y: 2.0, z: 3.0 },
                Point3D { x: 4.0, y: 5.0, z: 6.0 },
            ],
            &[3.5, 15.2],
            &[Point3D { x: 2.0, y: 3.0, z: 4.0 }],
        )
    }

    #[test]
    fn new_builds_parallel_arrays() {
        let p = make_points();
        assert_eq!(p.scan_points.len(), 2);
        assert_eq!(p.detections.len(), 1);
        assert!((p.scan_points[0].deviation_mm - 3.5).abs() < 1e-9);
        assert!((p.scan_points[1].deviation_mm - 15.2).abs() < 1e-9);
        assert!((p.scan_points[0].x - 1.0).abs() < 1e-6);
    }

    #[test]
    fn roundtrip_via_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("points.json");
        let original = make_points();
        write_points(&original, &path).unwrap();
        let loaded = read_points(&path).unwrap();
        assert_eq!(loaded.scan_points.len(), 2);
        assert_eq!(loaded.detections.len(), 1);
        assert!((loaded.scan_points[1].deviation_mm - 15.2).abs() < 1e-9);
    }

    #[test]
    fn json_schema_has_expected_keys() {
        let p = make_points();
        let json = serde_json::to_value(&p).unwrap();
        assert!(json["scan_points"].is_array());
        assert!(json["detections"].is_array());
        assert!(json["scan_points"][0]["deviation_mm"].is_number());
        assert!(json["scan_points"][0]["x"].is_number());
    }
}
