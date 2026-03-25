//! Heatmap rendering — project a scan point cloud with per-point deviation
//! values to a 2D PNG image using the same pinhole camera model as
//! [`trilink_core::project_to_depth_map`].
//!
//! # Colour scale
//!
//! | Deviation               | Colour  |
//! |-------------------------|---------|
//! | ≤ threshold             | green   |
//! | ≤ 2 × threshold         | yellow  |
//! | ≤ 4 × threshold         | orange → red ramp |
//! | > 4 × threshold         | red     |

use std::path::Path;

use image::{ImageBuffer, Rgb};
use thiserror::Error;
use trilink_core::{CameraIntrinsics, Point3D, Transform4x4};

// ---------------------------------------------------------------------------
// Colour mapping
// ---------------------------------------------------------------------------

/// Map a deviation value to an RGB colour using the standard three-zone scale.
///
/// - **green**  `[0, 200, 0]`   — deviation ≤ `threshold_mm`
/// - **yellow** `[255, 200, 0]` — deviation ≤ 2 × `threshold_mm`
/// - **red**    `[220, 0, 0]`   — linearly interpolated from yellow at 2 ×
///   threshold to full red at 4 × threshold, clamped above that
pub fn deviation_to_rgb(deviation_mm: f64, threshold_mm: f64) -> Rgb<u8> {
    if deviation_mm <= threshold_mm {
        Rgb([0, 200, 0])
    } else if deviation_mm <= 2.0 * threshold_mm {
        Rgb([255, 200, 0])
    } else {
        // Linear ramp from yellow at 2× to red at 4×, then clamped.
        let t = ((deviation_mm - 2.0 * threshold_mm) / (2.0 * threshold_mm)).clamp(0.0, 1.0);
        let g = (200.0 * (1.0 - t)) as u8;
        Rgb([220, g, 0])
    }
}

// ---------------------------------------------------------------------------
// Heatmap rendering
// ---------------------------------------------------------------------------

/// Render a per-point deviation heatmap as an [`ImageBuffer`].
///
/// Projects each point in `scan` to a 2D pixel using the same pinhole camera
/// model as [`trilink_core::project_to_depth_map`]:
///
/// 1. `P_camera = pose⁻¹ · P_world`
/// 2. Behind-camera points (`Zc ≤ 0`) are skipped.
/// 3. `u = fx·(Xc/Zc) + cx`, `v = fy·(Yc/Zc) + cy`
/// 4. Out-of-bounds pixels are skipped.
/// 5. Z-buffer: when multiple points project to the same pixel the nearest
///    one (smallest `Zc`) determines the colour.
///
/// Pixels with no projected point are black `[0, 0, 0]`.
///
/// # Panics
///
/// Panics if `scan.len() != deviations_mm.len()`.
pub fn render_heatmap(
    scan: &[Point3D],
    deviations_mm: &[f64],
    pose: &Transform4x4,
    k: &CameraIntrinsics,
    width: u32,
    height: u32,
    threshold_mm: f64,
) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    assert_eq!(
        scan.len(),
        deviations_mm.len(),
        "scan and deviations_mm must have equal length"
    );

    let n = (width * height) as usize;
    let mut z_buf: Vec<f32> = vec![f32::INFINITY; n];
    let mut dev_buf: Vec<f64> = vec![0.0; n];

    let world_to_cam = pose.mat.inverse();

    for (pt, &dev_mm) in scan.iter().zip(deviations_mm.iter()) {
        let pc = world_to_cam.transform_point3(glam::vec3(pt.x, pt.y, pt.z));

        if pc.z <= 0.0 {
            continue;
        }

        let zc = pc.z as f64;
        let u = k.fx * (pc.x as f64 / zc) + k.cx;
        let v = k.fy * (pc.y as f64 / zc) + k.cy;

        if u < 0.0 || v < 0.0 || u >= width as f64 || v >= height as f64 {
            continue;
        }

        let idx = v as u32 * width + u as u32;
        let idx = idx as usize;

        if pc.z < z_buf[idx] {
            z_buf[idx] = pc.z;
            dev_buf[idx] = dev_mm;
        }
    }

    let mut img = ImageBuffer::from_pixel(width, height, Rgb([0u8, 0, 0]));
    for (idx, &z) in z_buf.iter().enumerate() {
        if z.is_finite() {
            let x = (idx as u32) % width;
            let y = (idx as u32) / width;
            img.put_pixel(x, y, deviation_to_rgb(dev_buf[idx], threshold_mm));
        }
    }

    img
}

// ---------------------------------------------------------------------------
// PNG I/O
// ---------------------------------------------------------------------------

/// Error type for heatmap PNG write failures.
#[derive(Debug, Error)]
pub enum HeatmapError {
    #[error("failed to write heatmap PNG: {0}")]
    Io(String),
}

/// Write a rendered heatmap to a PNG file at `path`.
pub fn write_heatmap_png(
    img: &ImageBuffer<Rgb<u8>, Vec<u8>>,
    path: &Path,
) -> Result<(), HeatmapError> {
    img.save(path).map_err(|e| HeatmapError::Io(e.to_string()))
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn green_below_threshold() {
        assert_eq!(deviation_to_rgb(0.0, 10.0), Rgb([0, 200, 0]));
        assert_eq!(deviation_to_rgb(10.0, 10.0), Rgb([0, 200, 0]));
    }

    #[test]
    fn yellow_between_one_and_two_times_threshold() {
        assert_eq!(deviation_to_rgb(10.001, 10.0), Rgb([255, 200, 0]));
        assert_eq!(deviation_to_rgb(20.0, 10.0), Rgb([255, 200, 0]));
    }

    #[test]
    fn red_above_four_times_threshold() {
        let Rgb([r, g, b]) = deviation_to_rgb(100.0, 10.0);
        assert_eq!(r, 220);
        assert_eq!(g, 0);
        assert_eq!(b, 0);
    }

    #[test]
    fn behind_camera_point_not_rendered() {
        use trilink_core::Transform4x4;
        let pts = vec![Point3D { x: 0.0, y: 0.0, z: -1.0 }];
        let devs = vec![99.0f64];
        let k = CameraIntrinsics { fx: 10.0, fy: 10.0, cx: 50.0, cy: 50.0 };
        let img = render_heatmap(&pts, &devs, &Transform4x4::identity(), &k, 100, 100, 10.0);
        // All pixels should be black (no point was projected).
        assert!(img.pixels().all(|p| *p == Rgb([0, 0, 0])));
    }
}
