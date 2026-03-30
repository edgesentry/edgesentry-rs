//! Configuration for `edgesentry-inspect scan`.
//!
//! Loaded from a TOML file passed via `--config`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Top-level scan configuration.
#[derive(Debug, Deserialize, Serialize)]
pub struct ScanConfig {
    /// Path to the IFC design file (reference model).
    pub ifc_path: PathBuf,
    /// Path to the scan point cloud (PLY, ASCII format).
    pub scan_path: PathBuf,
    /// Optional path to a pre-extracted `reference.json` mesh file.
    ///
    /// When set, `run_scan` copies it to the output directory so the
    /// Inspect App viewer can render the IFC reference as a wireframe.
    /// Produce this file with `eds inspect extract-mesh`.
    #[serde(default)]
    pub mesh_path: Option<PathBuf>,
    /// Pinhole camera calibration used for depth-map projection and heatmap rendering.
    pub camera: CameraConfig,
    /// AI inference settings.
    pub inference: InferenceConfig,
    /// Output directory and deviation threshold.
    pub output: OutputConfig,
}

/// Pinhole camera calibration parameters.
#[derive(Debug, Deserialize, Serialize)]
pub struct CameraConfig {
    pub fx: f64,
    pub fy: f64,
    pub cx: f64,
    pub cy: f64,
    pub width: u32,
    pub height: u32,
}

/// Inference mode and endpoint configuration.
#[derive(Debug, Deserialize, Serialize)]
pub struct InferenceConfig {
    /// `"off"` — skip; `"mock"` — built-in demo; `"onnx"` — local model file; `"http"` — third-party server.
    pub mode: InferenceMode,
    /// Required when `mode = "onnx"`. Path to a `.onnx` model file.
    #[serde(default)]
    pub model_path: Option<PathBuf>,
    /// Required when `mode = "http"`. URL of the inference service.
    #[serde(default)]
    pub endpoint: Option<String>,
    /// Fallback depth (metres) used when a detection pixel has no ToF reading.
    #[serde(default = "default_fallback_depth")]
    pub fallback_depth_m: f32,
}

fn default_fallback_depth() -> f32 {
    2.0
}

/// Whether to run AI defect detection.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InferenceMode {
    /// Skip AI inference — deviation report and heatmap are still produced.
    Off,
    /// Return hardcoded bounding boxes for the built-in synthetic fixture.
    ///
    /// No external server required. Use this mode to demonstrate the full
    /// AI detection pipeline (depth map → detections → orange spheres in viewer)
    /// without a production model.
    Mock,
    /// Load a local `.onnx` model file and run inference in-process via `tract`.
    ///
    /// No external server or network access required. Suitable for edge / field-PC
    /// deployment. Set `model_path` to the `.onnx` file.
    Onnx,
    /// POST the depth-map PNG to a third-party HTTP inference server (e.g. YOLOv8).
    Http,
}

/// Output paths and quality threshold.
#[derive(Debug, Deserialize, Serialize)]
pub struct OutputConfig {
    /// Directory where `report.json` and `heatmap.png` are written.
    pub dir: PathBuf,
    /// Scan points within this distance of the reference model are considered compliant.
    #[serde(default = "default_threshold")]
    pub threshold_mm: f64,
}

fn default_threshold() -> f64 {
    10.0
}

/// Parse a [`ScanConfig`] from a TOML file.
pub fn load_config(path: &std::path::Path) -> Result<ScanConfig, ConfigError> {
    let text = std::fs::read_to_string(path)?;
    let cfg = toml::from_str(&text)?;
    Ok(cfg)
}

/// Errors produced while loading the config.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
}
