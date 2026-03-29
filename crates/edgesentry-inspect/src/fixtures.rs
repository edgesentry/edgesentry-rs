//! Offline fixture generator for `eds inspect generate-fixtures`.
//!
//! Produces three files in a target directory:
//!
//! - `wall_slab.ifc`      — 3 m × 2 m flat wall as `IFCCARTESIANPOINT` entries
//! - `wall_slab_scan.ply` — same wall with a 20 mm outward bulge in the centre
//! - `config.toml`        — pre-configured for `eds inspect scan`
//!
//! All generation runs offline with no external dependencies.

use std::fmt::Write as FmtWrite;
use std::path::Path;

use trilink_core::Point3D;

use crate::ply::write_ply_points;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Errors produced while generating fixture files.
#[derive(Debug, thiserror::Error)]
pub enum FixturesError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("PLY write error: {0}")]
    Ply(#[from] crate::ply::PlyError),
}

/// Generate demo data files into `dir` (created if absent).
///
/// | File | Contents |
/// |------|----------|
/// | `wall_slab.ifc` | 651 `IFCCARTESIANPOINT` entries — flat 3 m × 2 m wall |
/// | `wall_slab_scan.ply` | Same grid; centre 7 × 7 patch displaced 20 mm toward camera |
/// | `config.toml` | Ready-to-use config for `eds inspect scan` |
pub fn generate_fixtures(dir: &Path) -> Result<FixtureSummary, FixturesError> {
    std::fs::create_dir_all(dir)?;

    let (reference, scan, defect_count) = build_wall_points();

    write_ifc(dir, &reference)?;
    write_ply_points(&dir.join("wall_slab_scan.ply"), &scan)?;
    write_config(dir)?;

    Ok(FixtureSummary {
        point_count: reference.len(),
        defect_point_count: defect_count,
    })
}

/// Counts returned by [`generate_fixtures`] for display and testing.
pub struct FixtureSummary {
    /// Total number of reference points (= scan points).
    pub point_count: usize,
    /// Number of scan points in the defect region (displaced 20 mm).
    pub defect_point_count: usize,
}

// ---------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------

/// Coordinate system:
/// - Camera at origin pointing in +Z
/// - Wall at Z = 2.0 m, X ∈ [−1.5, +1.5], Y ∈ [−1.0, +1.0]
/// - Grid step: 0.1 m → 31 × 21 = 651 points
///
/// Defect: points where |X| ≤ 0.35 and |Y| ≤ 0.35 are displaced by −20 mm
/// (moved 20 mm closer to the camera, i.e. Z decreases).
fn build_wall_points() -> (Vec<Point3D>, Vec<Point3D>, usize) {
    const DEPTH: f32 = 2.0;
    const DEFECT_Z: f32 = 0.020; // 20 mm displacement toward camera

    let x_steps = 31usize; // −1.5 … +1.5 inclusive
    let y_steps = 21usize; // −1.0 … +1.0 inclusive

    let mut reference = Vec::with_capacity(x_steps * y_steps);
    let mut scan = Vec::with_capacity(x_steps * y_steps);
    let mut defect_count = 0usize;

    for row in 0..y_steps {
        let y = -1.0_f32 + row as f32 * 0.1;
        for col in 0..x_steps {
            let x = -1.5_f32 + col as f32 * 0.1;
            reference.push(Point3D { x, y, z: DEPTH });

            let in_defect = x.abs() <= 0.35 && y.abs() <= 0.35;
            if in_defect {
                defect_count += 1;
                scan.push(Point3D { x, y, z: DEPTH - DEFECT_Z });
            } else {
                let noise = sinusoidal_noise(row, col);
                scan.push(Point3D { x, y, z: DEPTH + noise });
            }
        }
    }

    (reference, scan, defect_count)
}

/// Deterministic sub-millimetre noise using a sinusoidal pattern.
///
/// Returns a value in [−0.002, +0.002] m (±2 mm) — well within the 10 mm threshold.
fn sinusoidal_noise(row: usize, col: usize) -> f32 {
    let phase = (row * 7 + col * 13) as f32 * 0.3;
    phase.sin() * 0.002
}

// ---------------------------------------------------------------------------
// IFC writer
// ---------------------------------------------------------------------------

fn write_ifc(dir: &Path, points: &[Point3D]) -> Result<(), FixturesError> {
    let mut content = String::new();
    writeln!(content, "ISO-10303-21;").unwrap();
    writeln!(content, "HEADER;").unwrap();
    writeln!(
        content,
        "FILE_DESCRIPTION(('EdgeSentry Inspect fixture — 3m x 2m synthetic wall'),'2;1');"
    )
    .unwrap();
    writeln!(
        content,
        "FILE_NAME('wall_slab.ifc','2024-01-01T00:00:00',(''),(''),'','','');"
    )
    .unwrap();
    writeln!(content, "FILE_SCHEMA(('IFC4'));").unwrap();
    writeln!(content, "ENDSEC;").unwrap();
    writeln!(content, "DATA;").unwrap();
    writeln!(
        content,
        "#1= IFCPROJECT('WALLSLAB01',$,'WallSlabFixture',$,$,$,$,$,$);"
    )
    .unwrap();
    for (i, p) in points.iter().enumerate() {
        writeln!(
            content,
            "#{}= IFCCARTESIANPOINT(({:.4},{:.4},{:.4}));",
            i + 2,
            p.x,
            p.y,
            p.z
        )
        .unwrap();
    }
    writeln!(content, "ENDSEC;").unwrap();
    writeln!(content, "END-ISO-10303-21;").unwrap();

    std::fs::write(dir.join("wall_slab.ifc"), content)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Config writer
// ---------------------------------------------------------------------------

fn write_config(dir: &Path) -> Result<(), FixturesError> {
    // Camera intrinsics are chosen so the wall exactly fills the 1920×1080 frame
    // at a depth of 2 m:
    //   horizontal: tan(half-fov) = 1.5/2.0 → fx = 960 / 0.75 = 1280
    //   vertical:   tan(half-fov) = 1.0/2.0 → fy = 540 / 0.50 = 1080
    let content = "\
# edgesentry-inspect demo — generated by `eds inspect generate-fixtures`
#
# Run the scan pipeline with:
#   eds inspect scan --config config.toml

ifc_path  = \"wall_slab.ifc\"
scan_path = \"wall_slab_scan.ply\"

[camera]
fx     = 1280.0   # wall fills image width at depth 2 m
fy     = 1080.0   # wall fills image height at depth 2 m
cx     = 960.0
cy     = 540.0
width  = 1920
height = 1080

[inference]
mode = \"off\"   # no AI server required for the demo

[output]
dir          = \"./output\"
threshold_mm = 10.0
";

    std::fs::write(dir.join("config.toml"), content)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wall_has_correct_point_count() {
        let (reference, scan, defect_count) = build_wall_points();
        assert_eq!(reference.len(), 651, "31 × 21 grid = 651 points");
        assert_eq!(scan.len(), 651);
        assert!(defect_count > 0, "defect region must contain at least one point");
        assert!(defect_count < 651, "not every point should be a defect");
    }

    #[test]
    fn defect_points_displaced_20mm() {
        let (reference, scan, _) = build_wall_points();
        for (r, s) in reference.iter().zip(scan.iter()) {
            if r.x.abs() <= 0.35 && r.y.abs() <= 0.35 {
                let dz = (r.z - s.z).abs();
                assert!(
                    (dz - 0.020).abs() < 1e-4,
                    "defect displacement should be 20 mm, got {dz:.4} m"
                );
            }
        }
    }

    #[test]
    fn non_defect_points_within_threshold() {
        let (reference, scan, _) = build_wall_points();
        for (r, s) in reference.iter().zip(scan.iter()) {
            if !(r.x.abs() <= 0.35 && r.y.abs() <= 0.35) {
                let dev = ((r.x - s.x).powi(2) + (r.y - s.y).powi(2) + (r.z - s.z).powi(2))
                    .sqrt()
                    * 1000.0; // mm
                assert!(dev < 10.0, "non-defect deviation {dev:.2} mm exceeds threshold");
            }
        }
    }

    #[test]
    fn generate_fixtures_writes_three_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        let summary = generate_fixtures(tmp.path()).unwrap();
        assert!(tmp.path().join("wall_slab.ifc").exists());
        assert!(tmp.path().join("wall_slab_scan.ply").exists());
        assert!(tmp.path().join("config.toml").exists());
        assert_eq!(summary.point_count, 651);
    }

    #[test]
    fn generated_ifc_is_parseable() {
        let tmp = tempfile::TempDir::new().unwrap();
        generate_fixtures(tmp.path()).unwrap();
        let pts = crate::ifc::load_ifc_points(&tmp.path().join("wall_slab.ifc")).unwrap();
        assert_eq!(pts.len(), 651);
    }

    #[test]
    fn generated_ply_is_parseable() {
        let tmp = tempfile::TempDir::new().unwrap();
        generate_fixtures(tmp.path()).unwrap();
        let pts = crate::ply::load_ply_points(&tmp.path().join("wall_slab_scan.ply")).unwrap();
        assert_eq!(pts.len(), 651);
    }
}
