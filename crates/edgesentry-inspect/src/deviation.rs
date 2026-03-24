//! Deviation engine — compute per-point nearest-neighbour distances between a
//! scan point cloud and a design reference cloud.
//!
//! Uses a k-d tree (via the `kd-tree` crate) for O(log n) nearest-neighbour
//! queries. Distances are in metres internally and reported in millimetres.

use kd_tree::KdTree;
use serde::{Deserialize, Serialize};
use trilink_core::Point3D;

/// Summary statistics from a scan-vs-reference deviation analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviationReport {
    /// Percentage of scan points whose nearest-reference distance is within
    /// `threshold_mm` (as supplied to [`compute_deviation`]).
    pub compliant_pct: f64,
    /// Maximum nearest-neighbour distance across all scan points, in mm.
    pub max_deviation_mm: f64,
    /// Mean nearest-neighbour distance across all scan points, in mm.
    pub mean_deviation_mm: f64,
    /// Total number of scan points analysed.
    pub point_count: usize,
}

/// Compute nearest-neighbour deviation between `scan` and `reference` clouds.
///
/// For each point in `scan` the closest point in `reference` is found via a
/// k-d tree, the Euclidean distance (in metres) is multiplied by 1 000 to
/// obtain millimetres, and summary statistics are accumulated.
///
/// # Panics
///
/// Panics if `reference` is empty (a k-d tree cannot be built from zero points).
pub fn compute_deviation(
    scan: &[Point3D],
    reference: &[Point3D],
    threshold_mm: f64,
) -> DeviationReport {
    assert!(!reference.is_empty(), "reference cloud must not be empty");

    if scan.is_empty() {
        return DeviationReport {
            compliant_pct: 100.0,
            max_deviation_mm: 0.0,
            mean_deviation_mm: 0.0,
            point_count: 0,
        };
    }

    // Build k-d tree from reference cloud.
    let ref_pts: Vec<[f32; 3]> = reference.iter().map(|p| [p.x, p.y, p.z]).collect();
    let tree = KdTree::build_by_ordered_float(ref_pts);

    let mut max_dist_sq: f64 = 0.0;
    let mut sum_dist_mm: f64 = 0.0;
    let mut compliant_count: usize = 0;

    for pt in scan {
        let nearest = tree.nearest(&[pt.x, pt.y, pt.z]).unwrap();
        let dist_sq = nearest.squared_distance as f64; // metres²
        let dist_mm = dist_sq.sqrt() * 1000.0;

        if dist_sq > max_dist_sq {
            max_dist_sq = dist_sq;
        }
        sum_dist_mm += dist_mm;
        if dist_mm <= threshold_mm {
            compliant_count += 1;
        }
    }

    let n = scan.len();
    DeviationReport {
        compliant_pct: compliant_count as f64 / n as f64 * 100.0,
        max_deviation_mm: max_dist_sq.sqrt() * 1000.0,
        mean_deviation_mm: sum_dist_mm / n as f64,
        point_count: n,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_grid() -> Vec<Point3D> {
        let mut pts = Vec::new();
        for i in 0..3_i32 {
            for j in 0..3_i32 {
                pts.push(Point3D { x: i as f32, y: j as f32, z: 0.0 });
            }
        }
        pts
    }

    #[test]
    fn identical_clouds_zero_deviation() {
        let cloud = make_grid();
        let report = compute_deviation(&cloud, &cloud, 1.0);
        assert_eq!(report.point_count, 9);
        assert!(report.max_deviation_mm < 1e-3);
        assert!((report.compliant_pct - 100.0).abs() < 1e-6);
    }

    #[test]
    fn single_displaced_point_detected() {
        let reference = make_grid();
        let mut scan = make_grid();
        // Displace last point by 0.02 m = 20 mm along Z
        let last = scan.last_mut().unwrap();
        last.z += 0.02;

        let report = compute_deviation(&scan, &reference, 15.0);
        assert_eq!(report.point_count, 9);
        assert!((report.max_deviation_mm - 20.0).abs() < 0.1);
        assert!(report.compliant_pct < 100.0);
    }
}
