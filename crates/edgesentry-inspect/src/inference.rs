//! AI inference for defect detection.
//!
//! Three runtime modes are supported:
//!
//! - **mock** — hardcoded bounding boxes for the synthetic wall fixture (no deps).
//! - **onnx** — local `.onnx` model loaded and executed in-process via `tract`.
//! - **http** — POST depth-map PNG to a third-party HTTP server (e.g. YOLOv8).

use std::path::Path;

use image::codecs::png::PngEncoder;
use image::ImageEncoder;
use serde::Deserialize;
use trilink_core::{BBox2D, DepthMap};

/// Errors produced during inference.
#[derive(Debug, thiserror::Error)]
pub enum InferenceError {
    #[error("failed to encode depth map as PNG: {0}")]
    Encode(String),
    #[error("ONNX inference error: {0}")]
    Onnx(String),
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

/// Return hardcoded bounding boxes for the built-in synthetic wall fixture.
///
/// The fixture has a 20 mm bulge centred at (0, 0) in a 3 m × 2 m wall placed
/// 2 m from the camera. With the demo camera (`fx = 1280`, `fy = 1080`,
/// `cx = 960`, `cy = 540`, `1920 × 1080`) the defect region (`|x| ≤ 0.35`,
/// `|y| ≤ 0.35`) projects to approximately pixel box (734, 349) → (1186, 731).
pub fn mock_infer() -> Vec<BBox2D> {
    vec![BBox2D { u0: 734.0, v0: 349.0, u1: 1186.0, v1: 731.0 }]
}

// ---------------------------------------------------------------------------
// ONNX (in-process) inference
// ---------------------------------------------------------------------------

/// Resize a flat float32 image `(in_w × in_h)` to `(out_w × out_h)` using
/// min-pooling over each source tile.
///
/// For each output pixel the minimum value in the corresponding source region
/// is taken. This preserves defect signals in sparse depth maps: if any source
/// pixel within a tile has a small (close) depth it propagates to the output
/// rather than being lost to a nearest-neighbour sample that may miss it.
/// No-data pixels encoded as `1.0` never win the min unless the tile is empty,
/// in which case `1.0` (far background) is the correct output anyway.
fn resize_min_pool(
    src: &[f32],
    in_w: usize,
    in_h: usize,
    out_w: usize,
    out_h: usize,
) -> Vec<f32> {
    let mut dst = vec![1.0f32; out_w * out_h];
    for oy in 0..out_h {
        let y0 = (oy * in_h) / out_h;
        let y1 = (((oy + 1) * in_h) / out_h).max(y0 + 1);
        for ox in 0..out_w {
            let x0 = (ox * in_w) / out_w;
            let x1 = (((ox + 1) * in_w) / out_w).max(x0 + 1);
            let mut min_val = 1.0f32;
            for iy in y0..y1 {
                for ix in x0..x1 {
                    let v = src[iy * in_w + ix];
                    if v < min_val {
                        min_val = v;
                    }
                }
            }
            dst[oy * out_w + ox] = min_val;
        }
    }
    dst
}

/// Load a `.onnx` model from `model_path` and run it on `depth_map`.
///
/// # Encoding
///
/// The depth map is normalised to `[0.0, 1.0]` where `0.0` is the minimum
/// (closest) finite depth in the scene and `1.0` is the maximum. Pixels with
/// no ToF reading (`f32::INFINITY`) are mapped to `1.0` so they are treated as
/// distant background rather than as potential defects.
///
/// The normalised map is then downsampled to `32 × 32` by nearest-neighbour
/// and fed to the model as a `[1, 1, 32, 32]` float32 NCHW tensor.
///
/// # Model contract
///
/// | Slot | Name | Shape | Values |
/// |------|------|-------|--------|
/// | input 0 | `image` | `[1, 1, 32, 32]` float32 | depth normalised to `[0, 1]` |
/// | output 0 | `boxes` | `[1, 5]` float32 | `[u0, v0, u1, v1, confidence]` normalised to `[0, 1]` |
///
/// If `confidence < 0.05` the function returns an empty Vec (no detections).
/// Otherwise the normalised coordinates are scaled to pixel space using the
/// depth map's actual width and height.
pub fn onnx_infer(model_path: &Path, depth_map: &DepthMap) -> Result<Vec<BBox2D>, InferenceError> {
    use tract_onnx::prelude::*;

    const MODEL_W: usize = 32;
    const MODEL_H: usize = 32;
    const CONF_THRESHOLD: f32 = 0.02;

    // Step 1 — normalise depth values: finite→[0,1], INFINITY→1.0 (far background).
    let finite: Vec<f32> =
        depth_map.data.iter().copied().filter(|v| v.is_finite()).collect();
    let (d_min, d_range) = if finite.is_empty() {
        (0.0f32, 1.0f32)
    } else {
        let mn = finite.iter().copied().fold(f32::INFINITY, f32::min);
        let mx = finite.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let range = if (mx - mn).abs() < 1e-6 { 1.0 } else { mx - mn };
        (mn, range)
    };
    let normalised: Vec<f32> = depth_map
        .data
        .iter()
        .map(|&v| if v.is_finite() { (v - d_min) / d_range } else { 1.0 })
        .collect();

    // Step 2 — downsample to 32×32 using min-pooling to preserve sparse defect pixels.
    let resized = resize_min_pool(
        &normalised,
        depth_map.width as usize,
        depth_map.height as usize,
        MODEL_W,
        MODEL_H,
    );

    // Step 3 — load, optimise, and run the ONNX model.
    let model = tract_onnx::onnx()
        .model_for_path(model_path)
        .map_err(|e| InferenceError::Onnx(e.to_string()))?
        .with_input_fact(
            0,
            InferenceFact::dt_shape(
                f32::datum_type(),
                tvec![1usize, 1usize, MODEL_H, MODEL_W],
            ),
        )
        .map_err(|e| InferenceError::Onnx(e.to_string()))?
        .into_optimized()
        .map_err(|e| InferenceError::Onnx(e.to_string()))?
        .into_runnable()
        .map_err(|e| InferenceError::Onnx(e.to_string()))?;

    let input: Tensor =
        tract_ndarray::Array4::from_shape_vec((1, 1, MODEL_H, MODEL_W), resized)
            .map_err(|e| InferenceError::Onnx(e.to_string()))?
            .into();

    let result = model
        .run(tvec!(input.into()))
        .map_err(|e| InferenceError::Onnx(e.to_string()))?;

    // Step 4 — decode output [1, 5]: normalised [u0, v0, u1, v1, confidence].
    let output = result[0]
        .to_plain_array_view::<f32>()
        .map_err(|e| InferenceError::Onnx(e.to_string()))?;

    let confidence = output[[0, 4]];
    if confidence < CONF_THRESHOLD {
        return Ok(vec![]);
    }

    let w = depth_map.width as f32;
    let h = depth_map.height as f32;
    Ok(vec![BBox2D {
        u0: output[[0, 0]] * w,
        v0: output[[0, 1]] * h,
        u1: output[[0, 2]] * w,
        v1: output[[0, 3]] * h,
    }])
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
