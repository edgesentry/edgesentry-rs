//! Integration tests for M3 — heatmap rendering.
//!
//! Camera setup used throughout:
//!   identity pose (camera frame == world frame)
//!   fx = fy = 10, cx = cy = 50
//!   100 × 100 image
//!   threshold_mm = 10
//!
//! Three test points (all at z = 2, so Zc = 2):
//!   A: world (0, 0, 2)  → u = 10*(0/2)+50 = 50, v = 50   deviation =  5 mm → green
//!   B: world (2, 0, 2)  → u = 10*(2/2)+50 = 60, v = 50   deviation = 15 mm → yellow
//!   C: world (4, 0, 2)  → u = 10*(4/2)+50 = 70, v = 50   deviation = 50 mm → red

use edgesentry_inspect::heatmap::{render_heatmap, write_heatmap_png};
use image::Rgb;
use trilink_core::{CameraIntrinsics, Point3D, Transform4x4};

fn camera() -> (Transform4x4, CameraIntrinsics) {
    let pose = Transform4x4::identity();
    let k = CameraIntrinsics { fx: 10.0, fy: 10.0, cx: 50.0, cy: 50.0 };
    (pose, k)
}

fn test_points() -> (Vec<Point3D>, Vec<f64>) {
    let pts = vec![
        Point3D { x: 0.0, y: 0.0, z: 2.0 }, // → pixel (50, 50)
        Point3D { x: 2.0, y: 0.0, z: 2.0 }, // → pixel (60, 50)
        Point3D { x: 4.0, y: 0.0, z: 2.0 }, // → pixel (70, 50)
    ];
    let devs = vec![5.0, 15.0, 50.0];
    (pts, devs)
}

#[test]
fn green_pixel_at_expected_position() {
    let (pose, k) = camera();
    let (pts, devs) = test_points();
    let img = render_heatmap(&pts, &devs, &pose, &k, 100, 100, 10.0);
    let pixel = img.get_pixel(50, 50);
    assert_eq!(*pixel, Rgb([0, 200, 0]), "deviation 5 mm ≤ threshold 10 mm should be green");
}

#[test]
fn yellow_pixel_at_expected_position() {
    let (pose, k) = camera();
    let (pts, devs) = test_points();
    let img = render_heatmap(&pts, &devs, &pose, &k, 100, 100, 10.0);
    let pixel = img.get_pixel(60, 50);
    assert_eq!(*pixel, Rgb([255, 200, 0]), "deviation 15 mm (≤ 2×10) should be yellow");
}

#[test]
fn red_pixel_at_expected_position() {
    let (pose, k) = camera();
    let (pts, devs) = test_points();
    let img = render_heatmap(&pts, &devs, &pose, &k, 100, 100, 10.0);
    let Rgb([r, g, b]) = *img.get_pixel(70, 50);
    assert_eq!(r, 220, "red channel should be 220 for deviation 50 mm >> 4×threshold");
    assert_eq!(g, 0, "green channel should be 0 for full red");
    assert_eq!(b, 0);
}

#[test]
fn unprojected_pixels_are_black() {
    let (pose, k) = camera();
    let (pts, devs) = test_points();
    let img = render_heatmap(&pts, &devs, &pose, &k, 100, 100, 10.0);
    // Pixel (0, 0) has no point projected onto it.
    assert_eq!(*img.get_pixel(0, 0), Rgb([0, 0, 0]));
}

#[test]
fn z_buffer_nearest_point_wins() {
    let (pose, k) = camera();
    // Two points projecting to the same pixel (50, 50):
    // closer point (z=1) has deviation 50 mm → should be red
    // farther point (z=5) has deviation 0 mm → should be green but z-buffered out
    let pts = vec![
        Point3D { x: 0.0, y: 0.0, z: 5.0 }, // farther, green
        Point3D { x: 0.0, y: 0.0, z: 1.0 }, // nearer, red
    ];
    let devs = vec![0.0, 50.0];
    let img = render_heatmap(&pts, &devs, &pose, &k, 100, 100, 10.0);
    let Rgb([r, _, _]) = *img.get_pixel(50, 50);
    assert_eq!(r, 220, "nearer point (z=1, deviation=50 mm) should win z-buffer");
}

#[test]
fn png_roundtrip() {
    let (pose, k) = camera();
    let (pts, devs) = test_points();
    let img = render_heatmap(&pts, &devs, &pose, &k, 100, 100, 10.0);

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("heatmap.png");
    write_heatmap_png(&img, &path).expect("write should succeed");

    let loaded = image::open(&path).unwrap().to_rgb8();
    assert_eq!(loaded.get_pixel(50, 50), img.get_pixel(50, 50));
    assert_eq!(loaded.get_pixel(60, 50), img.get_pixel(60, 50));
    assert_eq!(loaded.get_pixel(70, 50), img.get_pixel(70, 50));
}
