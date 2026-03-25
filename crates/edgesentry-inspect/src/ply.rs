//! Minimal ASCII PLY reader.
//!
//! Reads `x`, `y`, `z` float properties from the `vertex` element of an ASCII
//! PLY file. Binary PLY and additional properties are ignored.

use std::io::{BufRead, BufReader};
use std::path::Path;

use trilink_core::Point3D;

/// Errors produced while loading a PLY file.
#[derive(Debug, thiserror::Error)]
pub enum PlyError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid PLY header: {0}")]
    Header(String),
    #[error("PLY parse error at vertex {index}: {msg}")]
    Parse { index: usize, msg: String },
}

/// Load `x y z` vertices from an ASCII PLY file.
///
/// Only the `vertex` element is read; additional elements (faces, edges, …)
/// are ignored. Properties other than `x`, `y`, `z` are silently skipped.
pub fn load_ply_points(path: &Path) -> Result<Vec<Point3D>, PlyError> {
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    // --- Parse header ---
    let first = lines
        .next()
        .ok_or_else(|| PlyError::Header("empty file".into()))??;
    if first.trim() != "ply" {
        return Err(PlyError::Header("missing 'ply' magic".into()));
    }

    let mut vertex_count: usize = 0;
    let mut x_idx: Option<usize> = None;
    let mut y_idx: Option<usize> = None;
    let mut z_idx: Option<usize> = None;
    let mut prop_idx: usize = 0;
    let mut in_vertex = false;

    loop {
        let line = lines
            .next()
            .ok_or_else(|| PlyError::Header("unexpected end of header".into()))??;
        let line = line.trim();

        if line == "end_header" {
            break;
        }

        if line.starts_with("element vertex") {
            in_vertex = true;
            vertex_count = line
                .split_whitespace()
                .nth(2)
                .ok_or_else(|| PlyError::Header("missing vertex count".into()))?
                .parse::<usize>()
                .map_err(|e| PlyError::Header(e.to_string()))?;
            prop_idx = 0;
        } else if line.starts_with("element ") {
            in_vertex = false;
        } else if line.starts_with("property ") && in_vertex {
            let name = line.split_whitespace().nth(2).unwrap_or("");
            match name {
                "x" => x_idx = Some(prop_idx),
                "y" => y_idx = Some(prop_idx),
                "z" => z_idx = Some(prop_idx),
                _ => {}
            }
            prop_idx += 1;
        }
    }

    let x_col = x_idx.ok_or_else(|| PlyError::Header("no 'x' property in vertex element".into()))?;
    let y_col = y_idx.ok_or_else(|| PlyError::Header("no 'y' property in vertex element".into()))?;
    let z_col = z_idx.ok_or_else(|| PlyError::Header("no 'z' property in vertex element".into()))?;

    // --- Parse vertex data ---
    let mut points = Vec::with_capacity(vertex_count);

    for (i, line_result) in lines.take(vertex_count).enumerate() {
        let line = line_result?;
        let parts: Vec<&str> = line.split_whitespace().collect();

        let parse = |col: usize| -> Result<f32, PlyError> {
            parts
                .get(col)
                .ok_or_else(|| PlyError::Parse {
                    index: i,
                    msg: format!("missing column {col}"),
                })?
                .parse::<f32>()
                .map_err(|e| PlyError::Parse { index: i, msg: e.to_string() })
        };

        points.push(Point3D { x: parse(x_col)?, y: parse(y_col)?, z: parse(z_col)? });
    }

    Ok(points)
}

/// Write `x y z` vertices to an ASCII PLY file.
///
/// Used in tests to generate fixture scan files.
pub fn write_ply_points(path: &Path, points: &[Point3D]) -> Result<(), PlyError> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;
    writeln!(f, "ply")?;
    writeln!(f, "format ascii 1.0")?;
    writeln!(f, "element vertex {}", points.len())?;
    writeln!(f, "property float x")?;
    writeln!(f, "property float y")?;
    writeln!(f, "property float z")?;
    writeln!(f, "end_header")?;
    for p in points {
        writeln!(f, "{} {} {}", p.x, p.y, p.z)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn roundtrip_ascii_ply() {
        let pts = vec![
            Point3D { x: 1.0, y: 2.0, z: 3.0 },
            Point3D { x: 4.5, y: -1.0, z: 0.0 },
        ];
        let f = NamedTempFile::new().unwrap();
        write_ply_points(f.path(), &pts).unwrap();
        let loaded = load_ply_points(f.path()).unwrap();
        assert_eq!(loaded.len(), 2);
        assert!((loaded[0].x - 1.0).abs() < 1e-5);
        assert!((loaded[1].z - 0.0).abs() < 1e-5);
    }

    #[test]
    fn missing_magic_returns_error() {
        let f = NamedTempFile::new().unwrap();
        std::fs::write(f.path(), "not-a-ply-file\n").unwrap();
        assert!(load_ply_points(f.path()).is_err());
    }

    #[test]
    fn extra_properties_are_skipped() {
        let content = "ply\nformat ascii 1.0\nelement vertex 1\nproperty float x\nproperty float y\nproperty float z\nproperty float intensity\nend_header\n1.0 2.0 3.0 0.5\n";
        let f = NamedTempFile::new().unwrap();
        std::fs::write(f.path(), content).unwrap();
        let pts = load_ply_points(f.path()).unwrap();
        assert_eq!(pts.len(), 1);
        assert!((pts[0].x - 1.0).abs() < 1e-5);
    }
}
