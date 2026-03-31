//! Edge-first deviation detection for construction and maritime inspection.
//!
//! # Quick start
//!
//! ```no_run
//! use edgesentry_inspect::{ScanConfig, run_scan};
//!
//! let config: ScanConfig = toml::from_str(include_str!("../config.example.toml")).unwrap();
//! let result = run_scan(&config).unwrap();
//! println!("compliant: {:.1}%", result.report.compliant_pct);
//! ```

pub mod config;
pub mod deviation;
pub mod fixtures;
pub mod heatmap;
pub mod ifc;
pub mod inference;
pub mod ingress;
pub mod mesh;
pub mod pipeline;
pub mod ply;
pub mod points;
pub mod report;

// ---------------------------------------------------------------------------
// Top-level re-exports — stable public API surface
// ---------------------------------------------------------------------------

pub use config::{CameraConfig, InferenceConfig, InferenceMode, OutputConfig, ScanConfig};
pub use deviation::DeviationReport;
pub use ingress::{FrameSource, SensorFrame};
pub use pipeline::{run_scan, ScanError, ScanOutput};
