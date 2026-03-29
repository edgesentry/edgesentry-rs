//! Reference mesh types and serialisation for the Three.js viewer.
//!
//! The `reference.json` file is produced by `eds inspect extract-mesh` (via the
//! IfcOpenShell Python sidecar) and consumed by the Inspect App viewer to render
//! the IFC design model as a semi-transparent wireframe alongside the scan cloud.
//!
//! # Schema
//!
//! ```json
//! {
//!   "vertices": [[x, y, z], ...],
//!   "faces":    [[i, j, k], ...]
//! }
//! ```

use std::path::Path;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Triangulated reference mesh produced by the IfcOpenShell sidecar.
///
/// Serialises to / deserialises from `reference.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceMesh {
    /// Vertex positions as `[x, y, z]` triples (metres, world coordinates).
    pub vertices: Vec<[f32; 3]>,
    /// Triangle face indices as `[i, j, k]` triples into `vertices`.
    pub faces: Vec<[u32; 3]>,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors produced while reading or writing a reference mesh.
#[derive(Debug, thiserror::Error)]
pub enum MeshError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

// ---------------------------------------------------------------------------
// I/O
// ---------------------------------------------------------------------------

/// Serialise a [`ReferenceMesh`] to `path` as JSON.
pub fn write_mesh(mesh: &ReferenceMesh, path: &Path) -> Result<(), MeshError> {
    let json = serde_json::to_string(mesh)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Deserialise a [`ReferenceMesh`] from a `reference.json` file at `path`.
pub fn load_mesh(path: &Path) -> Result<ReferenceMesh, MeshError> {
    let content = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_mesh() -> ReferenceMesh {
        ReferenceMesh {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0], [0.5, 0.5, 1.0]],
            faces: vec![[0, 1, 2], [0, 1, 3], [1, 2, 3], [0, 2, 3]],
        }
    }

    #[test]
    fn roundtrip_via_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("reference.json");
        let mesh = sample_mesh();
        write_mesh(&mesh, &path).unwrap();
        let loaded = load_mesh(&path).unwrap();
        assert_eq!(loaded.vertices.len(), 4);
        assert_eq!(loaded.faces.len(), 4);
        assert!((loaded.vertices[1][0] - 1.0).abs() < 1e-5);
        assert_eq!(loaded.faces[0], [0, 1, 2]);
    }

    #[test]
    fn json_has_expected_keys() {
        let mesh = sample_mesh();
        let v = serde_json::to_value(&mesh).unwrap();
        assert!(v["vertices"].is_array());
        assert!(v["faces"].is_array());
        assert_eq!(v["vertices"].as_array().unwrap().len(), 4);
    }
}
