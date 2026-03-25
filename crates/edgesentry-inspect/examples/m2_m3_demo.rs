//! Manual end-to-end demo for M2 (IFC loader + deviation engine) and
//! M3 (heatmap rendering).
//!
//! Run with:
//!   cargo run -p edgesentry-inspect --example m2_m3_demo
//!
//! Output:
//!   - deviation report printed to stdout
//!   - heatmap PNG written to /tmp/edgesentry_heatmap.png

use std::path::Path;

use edgesentry_inspect::deviation::compute_deviation;
use edgesentry_inspect::heatmap::{render_heatmap, write_heatmap_png};
use edgesentry_inspect::ifc::load_ifc_points;
use trilink_core::{CameraIntrinsics, Point3D, Transform4x4};

fn main() {
    // -----------------------------------------------------------------------
    // M2 — Load IFC reference cloud
    // -----------------------------------------------------------------------
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/sample.ifc");

    let reference = load_ifc_points(&fixture).expect("failed to load sample.ifc");
    println!("Loaded {} reference points from IFC", reference.len());
    for p in &reference {
        println!("  reference point: ({:.2}, {:.2}, {:.2})", p.x, p.y, p.z);
    }

    // -----------------------------------------------------------------------
    // M2 — Simulate a scan cloud with a known 15 mm displacement on one point
    // -----------------------------------------------------------------------
    let mut scan = reference.clone();
    if let Some(last) = scan.last_mut() {
        last.z += 0.015; // +15 mm along Z
        println!("\nSimulated scan: displaced last point by 15 mm along Z");
    }

    // -----------------------------------------------------------------------
    // M2 — Compute deviation
    // -----------------------------------------------------------------------
    let threshold_mm = 10.0;
    let report = compute_deviation(&scan, &reference, threshold_mm);

    println!("\n=== Deviation Report ===");
    println!("  point_count      : {}", report.point_count);
    println!("  compliant_pct    : {:.1}%", report.compliant_pct);
    println!("  max_deviation_mm : {:.3} mm", report.max_deviation_mm);
    println!("  mean_deviation_mm: {:.3} mm", report.mean_deviation_mm);

    // -----------------------------------------------------------------------
    // M3 — Build per-point deviation array and render heatmap
    //
    // For this demo we recompute per-point distances using the same k-d tree
    // logic, then project with a simple top-down orthographic approximation
    // (identity pose, small focal length so all points land in the image).
    // -----------------------------------------------------------------------
    let deviations_mm: Vec<f64> = scan
        .iter()
        .zip(reference.iter())
        .map(|(s, r)| {
            let dx = (s.x - r.x) as f64;
            let dy = (s.y - r.y) as f64;
            let dz = (s.z - r.z) as f64;
            (dx * dx + dy * dy + dz * dz).sqrt() * 1000.0
        })
        .collect();

    // Camera: identity pose, focal length 100px, principal point at centre of
    // 200×200 image. Points are at z~0 so we shift them forward to z=5 for
    // projection (add a constant Z offset in camera space via a translated pose).
    let shifted_scan: Vec<Point3D> = scan
        .iter()
        .map(|p| Point3D { x: p.x, y: p.y, z: p.z + 5.0 })
        .collect();

    let pose = Transform4x4::identity();
    let k = CameraIntrinsics { fx: 100.0, fy: 100.0, cx: 100.0, cy: 100.0 };

    let img = render_heatmap(&shifted_scan, &deviations_mm, &pose, &k, 200, 200, threshold_mm);

    // Count coloured pixels as a sanity check.
    let coloured: usize = img
        .pixels()
        .filter(|p| **p != image::Rgb([0u8, 0, 0]))
        .count();
    println!("\n=== Heatmap ===");
    println!("  image size     : {}×{}", img.width(), img.height());
    println!("  coloured pixels: {}", coloured);

    let out = Path::new("/tmp/edgesentry_heatmap.png");
    write_heatmap_png(&img, out).expect("failed to write heatmap PNG");
    println!("  written to     : {}", out.display());
}
