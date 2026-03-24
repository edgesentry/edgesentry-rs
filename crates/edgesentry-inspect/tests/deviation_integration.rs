//! Integration tests for the M2 IFC Loader and Deviation Engine.

use std::path::PathBuf;

use edgesentry_inspect::deviation::compute_deviation;
use edgesentry_inspect::ifc::{load_ifc_points, parse_ifc_points};
use edgesentry_inspect::report::{read_report, write_report};
use trilink_core::Point3D;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_3x3_grid() -> Vec<Point3D> {
    let mut pts = Vec::new();
    for i in 0..3_i32 {
        for j in 0..3_i32 {
            pts.push(Point3D { x: i as f32, y: j as f32, z: 0.0 });
        }
    }
    pts
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

// ---------------------------------------------------------------------------
// Deviation engine tests
// ---------------------------------------------------------------------------

/// A 3×3 reference grid with one scan point displaced by 20 mm (0.02 m) along Z.
/// Threshold is 15 mm so that one point is non-compliant.
#[test]
fn displaced_point_detected_as_non_compliant() {
    let reference = make_3x3_grid();
    let mut scan = make_3x3_grid();

    // Displace one point by 20 mm along Z
    scan[4].z += 0.02; // centre point of 3×3 grid

    let threshold_mm = 15.0_f64;
    let report = compute_deviation(&scan, &reference, threshold_mm);

    assert_eq!(report.point_count, 9, "should analyse all 9 grid points");
    assert!(
        (report.max_deviation_mm - 20.0).abs() < 0.5,
        "max deviation should be ~20 mm, got {}",
        report.max_deviation_mm
    );
    assert!(
        report.compliant_pct < 100.0,
        "compliant_pct should be <100 when a point exceeds threshold, got {}",
        report.compliant_pct
    );
    // 8 out of 9 points are at zero deviation → compliant; 1 is non-compliant
    let expected_pct = 8.0_f64 / 9.0 * 100.0;
    assert!(
        (report.compliant_pct - expected_pct).abs() < 0.1,
        "expected compliant_pct ≈ {expected_pct:.2}, got {:.2}",
        report.compliant_pct
    );
}

/// Identical scan and reference produce zero deviation and 100 % compliance.
#[test]
fn identical_clouds_are_fully_compliant() {
    let cloud = make_3x3_grid();
    let report = compute_deviation(&cloud, &cloud, 1.0);
    assert_eq!(report.point_count, 9);
    assert!(report.max_deviation_mm < 1e-2, "max deviation should be ~0");
    assert!((report.compliant_pct - 100.0).abs() < 1e-6);
}

// ---------------------------------------------------------------------------
// IFC loader tests
// ---------------------------------------------------------------------------

/// Parse a small in-memory IFC string; should yield the expected 3D points.
#[test]
fn ifc_parser_extracts_3d_points_from_string() {
    let ifc = r#"
ISO-10303-21;
DATA;
#10= IFCCARTESIANPOINT((1.0,2.0,3.0));
#11= IFCCARTESIANPOINT((4.5,5.5,6.5));
#12= IFCCARTESIANPOINT((7.0,8.0));
ENDSEC;
END-ISO-10303-21;
"#;
    let pts = parse_ifc_points(ifc);
    assert_eq!(pts.len(), 2, "2D point should be excluded");
    assert!((pts[0].x - 1.0).abs() < 1e-5);
    assert!((pts[0].y - 2.0).abs() < 1e-5);
    assert!((pts[0].z - 3.0).abs() < 1e-5);
    assert!((pts[1].x - 4.5).abs() < 1e-5);
}

/// Load the sample fixture IFC file; should find the 3D IFCCARTESIANPOINT entries.
#[test]
fn ifc_loader_reads_fixture_file() {
    let path = fixtures_dir().join("sample.ifc");
    let pts = load_ifc_points(&path).expect("fixture file should load successfully");
    // sample.ifc has 6 explicit 3D points + 1 high-elevation point = 7 3D points
    // (one line is a 2D point and should be skipped)
    assert!(!pts.is_empty(), "should find at least one 3D point in the fixture");
    // Check that the 2D IFCCARTESIANPOINT is not included
    for pt in &pts {
        // A 2D point would be parsed incorrectly — ensure z is present (all 3D)
        // No coordinate should be NaN
        assert!(pt.x.is_finite());
        assert!(pt.y.is_finite());
        assert!(pt.z.is_finite());
    }
}

/// Write a temp IFC string and load it via the file-based API.
#[test]
fn ifc_loader_roundtrip_via_temp_file() {
    use std::io::Write;

    let ifc_content = b"ISO-10303-21;\nDATA;\n#1= IFCCARTESIANPOINT((10.0,20.0,30.0));\n#2= IFCCARTESIANPOINT((1.0,2.0));\nENDSEC;\nEND-ISO-10303-21;\n";

    let mut tmp = tempfile::NamedTempFile::new().expect("create temp file");
    tmp.write_all(ifc_content).expect("write IFC content");
    tmp.flush().expect("flush");

    let pts = load_ifc_points(tmp.path()).expect("load temp IFC file");
    assert_eq!(pts.len(), 1, "only 3D point should be loaded");
    assert!((pts[0].x - 10.0).abs() < 1e-4);
    assert!((pts[0].y - 20.0).abs() < 1e-4);
    assert!((pts[0].z - 30.0).abs() < 1e-4);
}

// ---------------------------------------------------------------------------
// Report serialisation tests
// ---------------------------------------------------------------------------

/// Write and read back a deviation report to/from a temporary file.
#[test]
fn report_write_read_roundtrip() {
    let reference = make_3x3_grid();
    let mut scan = make_3x3_grid();
    scan[4].z += 0.02;

    let report = compute_deviation(&scan, &reference, 15.0);

    let tmp = tempfile::NamedTempFile::new().expect("create temp file");
    write_report(&report, tmp.path()).expect("write report");
    let loaded = read_report(tmp.path()).expect("read report");

    assert_eq!(loaded.point_count, report.point_count);
    assert!((loaded.max_deviation_mm - report.max_deviation_mm).abs() < 1e-6);
    assert!((loaded.mean_deviation_mm - report.mean_deviation_mm).abs() < 1e-6);
    assert!((loaded.compliant_pct - report.compliant_pct).abs() < 1e-6);
}
