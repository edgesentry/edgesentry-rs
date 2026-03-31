//! Sensor frame ingress — abstract frame source trait and sensor frame type.
//!
//! `FrameSource` abstracts over live hardware sensors, file replay, and test
//! mocks. It belongs here (OSS inspection pipeline) rather than in trilink-core
//! because it is pipeline-specific, not pure geometry math.
//!
//! ## Crate placement principle
//!
//! - **trilink-core** — pure geometry/math only (projections, pose buffer, math types)
//! - **edgesentry-inspect** (this crate) — OSS pipeline features, including sensor ingress
//! - **edgesentry-app** — commercial features (SQLite egress, BIM server, PDF reports)

pub mod mock;

use crate::pipeline::ScanError;
use trilink_core::Transform4x4;

/// A frame emitted by a sensor platform: raw JPEG image + pose at shutter time.
#[derive(Debug, Clone)]
pub struct SensorFrame {
    /// Microseconds since UNIX epoch when the shutter opened.
    pub capture_ts_us: u64,
    /// Platform pose at shutter time from the localisation subsystem.
    pub pose: Transform4x4,
    /// JPEG-encoded image bytes.
    pub jpeg: Vec<u8>,
    /// ToF depth at the frame centre, if available.
    pub depth_m: Option<f32>,
}

/// Trait implemented by any source of sensor frames (real hardware, file replay, or mock).
///
/// `next_frame` is `async` so implementations can await hardware SDKs or
/// network streams without blocking a tokio worker thread.
///
/// Note: `async fn` in traits is not object-safe (`dyn FrameSource` is not
/// supported without a wrapper). Use a concrete type or a newtype wrapper when
/// dynamic dispatch is needed.
///
/// The `async_fn_in_trait` lint is suppressed intentionally: auto-trait bounds
/// (Send, Sync) on the returned future cannot be expressed in this form, but all
/// current implementations are `Send` and callers drive the future on a single
/// task.
#[allow(async_fn_in_trait)]
pub trait FrameSource: Send {
    /// Returns the next available frame, yielding until one is ready.
    ///
    /// Returns `Err(ScanError::Io(UnexpectedEof))` when the source is exhausted.
    async fn next_frame(&mut self) -> Result<SensorFrame, ScanError>;
}
