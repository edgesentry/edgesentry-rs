//! Integration tests for `eds inspect generate-fixtures`.
//!
//! Verifies that the generated files are valid and that the full scan pipeline
//! succeeds when run against them — no external data or network access required.

use std::path::Path;

use edgesentry_inspect::{
    config::{CameraConfig, InferenceConfig, InferenceMode, OutputConfig, ScanConfig},
    fixtures::generate_fixtures,
    ifc::load_ifc_points,
    pipeline::run_scan,
    ply::load_ply_points,
};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// File-level checks
// ---------------------------------------------------------------------------

#[test]
fn generate_fixtures_creates_all_files() {
    let tmp = TempDir::new().unwrap();
    let summary = generate_fixtures(tmp.path()).unwrap();

    assert!(
        tmp.path().join("wall_slab.ifc").exists(),
        "wall_slab.ifc must be created"
    );
    assert!(
        tmp.path().join("wall_slab_scan.ply").exists(),
        "wall_slab_scan.ply must be created"
    );
    assert!(
        tmp.path().join("config.toml").exists(),
        "config.toml must be created"
    );

    assert_eq!(summary.point_count, 651, "31 × 21 grid = 651 points");
    assert!(
        summary.defect_point_count > 0,
        "defect region must be non-empty"
    );
    assert!(
        summary.defect_point_count < summary.point_count,
        "only a subset of points should be in the defect region"
    );
}

#[test]
fn generated_ifc_loads_correctly() {
    let tmp = TempDir::new().unwrap();
    generate_fixtures(tmp.path()).unwrap();

    let pts = load_ifc_points(&tmp.path().join("wall_slab.ifc"))
        .expect("generated IFC must be loadable");
    assert_eq!(pts.len(), 651);

    // All reference points should be at z ≈ 2.0 m
    for p in &pts {
        assert!(
            (p.z - 2.0).abs() < 1e-3,
            "reference point z = {:.4} should be 2.0", p.z
        );
    }
}

#[test]
fn generated_ply_loads_correctly() {
    let tmp = TempDir::new().unwrap();
    let summary = generate_fixtures(tmp.path()).unwrap();

    let pts = load_ply_points(&tmp.path().join("wall_slab_scan.ply"))
        .expect("generated PLY must be loadable");
    assert_eq!(pts.len(), summary.point_count);
}

#[test]
fn generated_config_toml_is_valid() {
    let tmp = TempDir::new().unwrap();
    generate_fixtures(tmp.path()).unwrap();

    let text = std::fs::read_to_string(tmp.path().join("config.toml")).unwrap();
    let cfg: toml::Value = toml::from_str(&text).expect("config.toml must be valid TOML");
    assert!(cfg.get("ifc_path").is_some(), "config must contain ifc_path");
    assert!(cfg.get("scan_path").is_some(), "config must contain scan_path");
    assert!(cfg.get("camera").is_some(), "config must contain [camera]");
    assert!(cfg.get("inference").is_some(), "config must contain [inference]");
    assert!(cfg.get("output").is_some(), "config must contain [output]");
}

// ---------------------------------------------------------------------------
// End-to-end pipeline
// ---------------------------------------------------------------------------

#[test]
fn pipeline_detects_defect_in_generated_fixtures() {
    let tmp = TempDir::new().unwrap();
    generate_fixtures(tmp.path()).unwrap();

    let out_dir = tmp.path().join("out");
    let cfg = ScanConfig {
        ifc_path: tmp.path().join("wall_slab.ifc"),
        scan_path: tmp.path().join("wall_slab_scan.ply"),
        camera: CameraConfig {
            fx: 1280.0,
            fy: 1080.0,
            cx: 960.0,
            cy: 540.0,
            width: 1920,
            height: 1080,
        },
        inference: InferenceConfig {
            model_path: None,
            mode: InferenceMode::Off,
            endpoint: None,
            fallback_depth_m: 2.0,
        },
        mesh_path: None,
        output: OutputConfig { dir: out_dir.clone(), threshold_mm: 10.0 },
    };

    let out = run_scan(&cfg).expect("pipeline must succeed on generated fixtures");

    // The 20 mm defect exceeds the 10 mm threshold → compliance < 100%
    assert!(
        out.report.compliant_pct < 100.0,
        "defect region must cause non-compliance, got {:.1}%",
        out.report.compliant_pct
    );
    // Max deviation ≥ 19 mm (20 mm displacement, nearest-neighbour may be slightly less)
    assert!(
        out.report.max_deviation_mm > 19.0,
        "max deviation should exceed 19 mm, got {:.2} mm",
        out.report.max_deviation_mm
    );
    // Non-defect points are ≤ 2 mm → mean should be well below threshold
    assert!(
        out.report.mean_deviation_mm < 10.0,
        "mean deviation should be below threshold, got {:.2} mm",
        out.report.mean_deviation_mm
    );

    // All output files must exist
    assert!(out.report_path.exists());
    assert!(out.heatmap_path.exists());
    assert!(out.points_path.exists());
}

#[test]
fn idempotent_generation() {
    // Running generate-fixtures twice in the same directory must succeed
    // (files are overwritten, not appended).
    let tmp = TempDir::new().unwrap();
    generate_fixtures(tmp.path()).unwrap();
    let summary2 = generate_fixtures(tmp.path()).unwrap();
    assert_eq!(summary2.point_count, 651);
}

fn _assert_path_exists(p: &Path) {
    assert!(p.exists(), "{} must exist", p.display());
}
