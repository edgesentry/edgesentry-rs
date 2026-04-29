// Image processing utilities. Heavy backends are behind feature flags.
// Enable `onnx` for ONNX Runtime inference, `opencv` for OpenCV image processing.
// Both are off by default to keep the default build dependency-free.

#[cfg(feature = "onnx")]
pub mod onnx {
    // ONNX Runtime integration — not yet implemented.
    // Add ort or onnxruntime crate here when enabling this feature.
}

#[cfg(feature = "opencv")]
pub mod cv {
    // OpenCV integration — not yet implemented.
}

/// Returns the set of enabled features for diagnostic purposes.
pub fn enabled_features() -> Vec<&'static str> {
    vec![
        #[cfg(feature = "onnx")]
        "onnx",
        #[cfg(feature = "opencv")]
        "opencv",
    ]
}
