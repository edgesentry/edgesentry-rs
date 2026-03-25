//! AI inference client for defect detection.
//!
//! Encodes a [`trilink_core::DepthMap`] as a grayscale PNG and POSTs it to an
//! HTTP inference server (e.g. YOLOv8). The server responds with a JSON array
//! of bounding boxes.

use image::codecs::png::PngEncoder;
use image::ImageEncoder;
use serde::Deserialize;
use trilink_core::{BBox2D, DepthMap};

/// Errors produced during inference.
#[derive(Debug, thiserror::Error)]
pub enum InferenceError {
    #[error("failed to encode depth map as PNG: {0}")]
    Encode(String),
    #[error("HTTP request failed: {0}")]
    Http(String),
    #[error("failed to parse inference response: {0}")]
    Parse(#[from] serde_json::Error),
}

/// JSON shape returned by the inference server.
#[derive(Debug, Deserialize)]
struct DetectionJson {
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
}

/// Encode a [`DepthMap`] as an 8-bit grayscale PNG.
///
/// Finite depth values are normalised to `[0, 255]`. Pixels with no depth
/// (`f32::INFINITY`) map to black (0).
pub fn depth_map_to_png(dm: &DepthMap) -> Result<Vec<u8>, InferenceError> {
    let finite: Vec<f32> = dm.data.iter().copied().filter(|v| v.is_finite()).collect();

    let (d_min, d_range) = if finite.is_empty() {
        (0.0f32, 1.0f32)
    } else {
        let mn = finite.iter().copied().fold(f32::INFINITY, f32::min);
        let mx = finite.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let range = if (mx - mn).abs() < 1e-6 { 1.0 } else { mx - mn };
        (mn, range)
    };

    let pixels: Vec<u8> = dm
        .data
        .iter()
        .map(|&v| {
            if v.is_infinite() {
                0u8
            } else {
                ((v - d_min) / d_range * 255.0).clamp(0.0, 255.0) as u8
            }
        })
        .collect();

    let mut buf = Vec::new();
    PngEncoder::new(&mut buf)
        .write_image(
            &pixels,
            dm.width,
            dm.height,
            image::ExtendedColorType::L8,
        )
        .map_err(|e| InferenceError::Encode(e.to_string()))?;

    Ok(buf)
}

/// POST a depth-map PNG to an HTTP inference server and return bounding boxes.
///
/// The server must accept `Content-Type: image/png` and respond with a JSON
/// array of objects with `u0`, `v0`, `u1`, `v1` fields (pixel coordinates).
pub fn http_infer(endpoint: &str, png_bytes: &[u8]) -> Result<Vec<BBox2D>, InferenceError> {
    let response = ureq::post(endpoint)
        .set("Content-Type", "image/png")
        .send_bytes(png_bytes)
        .map_err(|e| InferenceError::Http(e.to_string()))?;

    let body = response
        .into_string()
        .map_err(|e| InferenceError::Http(e.to_string()))?;

    let detections: Vec<DetectionJson> = serde_json::from_str(&body)?;

    Ok(detections
        .into_iter()
        .map(|d| BBox2D { u0: d.u0, v0: d.v0, u1: d.u1, v1: d.v1 })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_depth_map(w: u32, h: u32, depth: f32) -> DepthMap {
        DepthMap { width: w, height: h, data: vec![depth; (w * h) as usize] }
    }

    #[test]
    fn depth_map_to_png_produces_valid_png() {
        let dm = flat_depth_map(4, 4, 2.0);
        let png = depth_map_to_png(&dm).unwrap();
        // PNG magic bytes: 0x89 0x50 0x4E 0x47
        assert_eq!(&png[..4], &[0x89, 0x50, 0x4e, 0x47]);
    }

    #[test]
    fn depth_map_to_png_all_infinity_produces_black() {
        let dm = DepthMap { width: 2, height: 2, data: vec![f32::INFINITY; 4] };
        let png = depth_map_to_png(&dm).unwrap();
        assert!(!png.is_empty(), "PNG should still be produced for all-infinity map");
    }

    #[test]
    fn depth_map_to_png_size_matches() {
        let dm = flat_depth_map(8, 6, 1.5);
        let png = depth_map_to_png(&dm).unwrap();
        // Decode and check dimensions
        let img = image::load_from_memory(&png).unwrap();
        assert_eq!(img.width(), 8);
        assert_eq!(img.height(), 6);
    }
}
