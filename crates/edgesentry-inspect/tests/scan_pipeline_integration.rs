//! End-to-end integration tests for the M4 scan pipeline.
//!
//! Tests run against the bundled `sample.ifc` fixture and a programmatically
//! generated PLY scan, so no external data or network access is required.

use std::path::Path;

use edgesentry_inspect::{
    config::{InferenceConfig, InferenceMode, OutputConfig, ScanConfig, CameraConfig},
    pipeline::run_scan,
    ply::write_ply_points,
};
use tempfile::TempDir;
use trilink_core::Point3D;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn sample_ifc() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/sample.ifc")
}

fn make_scan_with_defect() -> Vec<Point3D> {
    // Clone the reference set (7 points identical to sample.ifc) then displace
    // the last one by 15 mm along Z — the same simulation as the demo example.
    let reference = edgesentry_inspect::ifc::load_ifc_points(&sample_ifc())
        .expect("sample.ifc must be loadable");
    let mut scan = reference;
    if let Some(last) = scan.last_mut() {
        last.z += 0.015;
    }
    scan
}

fn default_camera() -> CameraConfig {
    // Small image: keeps test fast, still exercises the full projection path.
    CameraConfig { fx: 100.0, fy: 100.0, cx: 100.0, cy: 100.0, width: 200, height: 200 }
}

// ---------------------------------------------------------------------------
// Deviation-only pipeline (mode = "off")
// ---------------------------------------------------------------------------

#[test]
fn scan_off_mode_produces_report_and_heatmap() {
    let tmp = TempDir::new().unwrap();

    // Write a PLY scan
    let scan_pts = make_scan_with_defect();
    let ply_path = tmp.path().join("scan.ply");
    write_ply_points(&ply_path, &scan_pts).unwrap();

    let cfg = ScanConfig {
        ifc: None, ifc_path: sample_ifc(),
        scan_path: ply_path,
        camera: default_camera(),
        inference: InferenceConfig {
            model_path: None,
            mode: InferenceMode::Off,
            endpoint: None,
            fallback_depth_m: 2.0,
        },
        mesh_path: None,
        output: OutputConfig { dir: tmp.path().join("out"), threshold_mm: 10.0 },
    };

    let out = run_scan(&cfg).expect("pipeline must succeed");

    // Report fields
    assert_eq!(out.report.point_count, 7);
    assert!(out.report.max_deviation_mm > 14.0, "displaced point should exceed 14 mm");
    assert!(out.report.compliant_pct < 100.0, "at least one non-compliant point");
    assert!(out.report.compliant_pct > 0.0, "at least some compliant points");
    assert_eq!(out.detection_count, 0, "no AI detections in off mode");

    // Output files exist
    assert!(out.report_path.exists(), "report.json must be written");
    assert!(out.heatmap_path.exists(), "heatmap.png must be written");
    assert!(out.points_path.exists(), "points.json must be written");

    // report.json is valid JSON with expected fields
    let json = std::fs::read_to_string(&out.report_path).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(v["point_count"].as_u64().unwrap() == 7);
    assert!((v["threshold_mm"].as_f64().unwrap() - 10.0).abs() < 1e-9);

    // heatmap.png is a valid PNG (correct magic bytes)
    let png = std::fs::read(&out.heatmap_path).unwrap();
    assert_eq!(&png[..4], &[0x89, 0x50, 0x4e, 0x47], "heatmap must be a valid PNG");
}

#[test]
fn scan_zero_deviation_fully_compliant() {
    let tmp = TempDir::new().unwrap();

    // Scan identical to reference → all points compliant
    let reference = edgesentry_inspect::ifc::load_ifc_points(&sample_ifc()).unwrap();
    let ply_path = tmp.path().join("scan.ply");
    write_ply_points(&ply_path, &reference).unwrap();

    let cfg = ScanConfig {
        ifc: None, ifc_path: sample_ifc(),
        scan_path: ply_path,
        camera: default_camera(),
        inference: InferenceConfig {
            model_path: None,
            mode: InferenceMode::Off,
            endpoint: None,
            fallback_depth_m: 2.0,
        },
        mesh_path: None,
        output: OutputConfig { dir: tmp.path().join("out"), threshold_mm: 10.0 },
    };

    let out = run_scan(&cfg).unwrap();
    assert!((out.report.compliant_pct - 100.0).abs() < 1e-6);
    assert!(out.report.max_deviation_mm < 1e-3);
}

// ---------------------------------------------------------------------------
// Mock inference mode (built-in)
// ---------------------------------------------------------------------------

#[test]
fn scan_mock_mode_returns_one_detection_without_server() {
    let tmp = TempDir::new().unwrap();
    let scan_pts = make_scan_with_defect();
    let ply_path = tmp.path().join("scan.ply");
    write_ply_points(&ply_path, &scan_pts).unwrap();

    let cfg = ScanConfig {
        ifc: None, ifc_path: sample_ifc(),
        scan_path: ply_path,
        camera: default_camera(),
        inference: InferenceConfig {
            model_path: None,
            mode: InferenceMode::Mock,
            endpoint: None,
            fallback_depth_m: 2.0,
        },
        mesh_path: None,
        output: OutputConfig { dir: tmp.path().join("out"), threshold_mm: 10.0 },
    };

    let out = run_scan(&cfg).unwrap();

    assert_eq!(out.detection_count, 1, "mock mode must return exactly one detection");
    assert_eq!(out.world_detections.len(), 1);
    let wp = &out.world_detections[0];
    assert!(wp.x.is_finite() && wp.y.is_finite() && wp.z.is_finite());
    assert!(out.report_path.exists());
    assert!(out.heatmap_path.exists());
    assert!(out.points_path.exists());
}

// ---------------------------------------------------------------------------
// ONNX inference mode (local model file)
// ---------------------------------------------------------------------------

#[test]
fn scan_onnx_mode_detects_fixture_defect() {
    let tmp = TempDir::new().unwrap();

    // Use the full synthetic fixture (651 points, 1920×1080 camera) so the
    // depth-map encoding matches what the prototype model was designed for.
    let fixture_dir = tempfile::TempDir::new().unwrap();
    edgesentry_inspect::fixtures::generate_fixtures(fixture_dir.path()).unwrap();

    let cfg = edgesentry_inspect::config::ScanConfig {
        ifc: None, ifc_path:  fixture_dir.path().join("wall_slab.ifc"),
        scan_path: fixture_dir.path().join("wall_slab_scan.ply"),
        camera: edgesentry_inspect::config::CameraConfig {
            fx: 1280.0, fy: 1080.0,
            cx: 960.0,  cy: 540.0,
            width: 1920, height: 1080,
        },
        inference: edgesentry_inspect::config::InferenceConfig {
            mode: edgesentry_inspect::config::InferenceMode::Onnx,
            model_path: Some(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures/prototype_detector.onnx"),
            ),
            endpoint: None,
            fallback_depth_m: 2.0,
        },
        mesh_path: None,
        output: edgesentry_inspect::config::OutputConfig {
            dir: tmp.path().join("out"),
            threshold_mm: 10.0,
        },
    };

    let out = run_scan(&cfg).expect("ONNX pipeline must succeed");

    assert_eq!(out.detection_count, 1, "prototype model must return exactly one detection");
    let wp = &out.world_detections[0];
    assert!(wp.x.is_finite() && wp.y.is_finite() && wp.z.is_finite());
    // Defect centre is at (0, 0, 1.98) — check we are within 0.5 m (coarse, prototype accuracy)
    assert!(wp.x.abs() < 0.5, "detection x should be near 0, got {}", wp.x);
    assert!(wp.y.abs() < 0.5, "detection y should be near 0, got {}", wp.y);
    assert!((wp.z - 1.98).abs() < 0.5, "detection z should be near 1.98, got {}", wp.z);
    assert!(out.report_path.exists());
    assert!(out.heatmap_path.exists());
    assert!(out.points_path.exists());
}

// ---------------------------------------------------------------------------
// HTTP inference mode (mock server)
// ---------------------------------------------------------------------------

#[test]
fn scan_http_mode_returns_detections() {
    let tmp = TempDir::new().unwrap();
    let scan_pts = make_scan_with_defect();
    let ply_path = tmp.path().join("scan.ply");
    write_ply_points(&ply_path, &scan_pts).unwrap();

    // Mock inference server: always returns one bounding box
    let mut server = mockito::Server::new();
    let _mock = server
        .mock("POST", "/detect")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"u0":50.0,"v0":50.0,"u1":80.0,"v1":80.0}]"#)
        .create();

    let endpoint = format!("{}/detect", server.url());

    let cfg = ScanConfig {
        ifc: None, ifc_path: sample_ifc(),
        scan_path: ply_path,
        camera: default_camera(),
        inference: InferenceConfig {
            model_path: None,
            mode: InferenceMode::Http,
            endpoint: Some(endpoint),
            fallback_depth_m: 2.0,
        },
        mesh_path: None,
        output: OutputConfig { dir: tmp.path().join("out"), threshold_mm: 10.0 },
    };

    let out = run_scan(&cfg).unwrap();

    assert_eq!(out.detection_count, 1, "mock server returns one detection");
    assert_eq!(out.world_detections.len(), 1);
    // The detection must have a finite world position
    let wp = &out.world_detections[0];
    assert!(wp.x.is_finite() && wp.y.is_finite() && wp.z.is_finite());

    // Deviation report is still produced
    assert_eq!(out.report.point_count, 7);
    assert!(out.report_path.exists());
    assert!(out.heatmap_path.exists());
    assert!(out.points_path.exists());
}
