//! IFC (Industry Foundation Classes) STEP-21 loader.
//!
//! Parses `IFCCARTESIANPOINT` entries from an IFC file and returns them as a
//! `Vec<Point3D>`. Only 3D points (three coordinate values) are included;
//! 2D points are silently skipped.

use std::path::Path;

use trilink_core::Point3D;

/// Errors that can occur while loading an IFC file.
#[derive(Debug, thiserror::Error)]
pub enum IfcError {
    #[error("I/O error reading IFC file: {0}")]
    Io(#[from] std::io::Error),

    #[error("no 3D IFCCARTESIANPOINT entries found in file")]
    NoPoints,
}

/// Load all 3-dimensional `IFCCARTESIANPOINT` entries from an IFC STEP-21 file.
///
/// Each matching line has the form:
/// ```text
/// #123= IFCCARTESIANPOINT((1.0,2.0,3.0));
/// ```
/// Only entries with exactly three coordinate values are returned.
pub fn load_ifc_points(path: &Path) -> Result<Vec<Point3D>, IfcError> {
    let content = std::fs::read_to_string(path)?;
    let points = parse_ifc_points(&content);
    if points.is_empty() {
        return Err(IfcError::NoPoints);
    }
    Ok(points)
}

/// Parse `IFCCARTESIANPOINT` entries from an in-memory IFC string.
///
/// Exposed as a separate function so tests can avoid touching the filesystem.
pub fn parse_ifc_points(content: &str) -> Vec<Point3D> {
    let mut points = Vec::new();

    for line in content.lines() {
        let upper = line.trim().to_uppercase();
        // Look for lines containing IFCCARTESIANPOINT((…))
        if let Some(start) = upper.find("IFCCARTESIANPOINT((") {
            let rest = &line.trim()[start + "IFCCARTESIANPOINT((".len()..];
            if let Some(end) = rest.find("))") {
                let coords_str = &rest[..end];
                let coords: Vec<f32> = coords_str
                    .split(',')
                    .filter_map(|s| s.trim().parse::<f64>().ok().map(|v| v as f32))
                    .collect();
                if coords.len() == 3 {
                    points.push(Point3D { x: coords[0], y: coords[1], z: coords[2] });
                }
            }
        }
    }

    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_3d_points() {
        let ifc = r#"
ISO-10303-21;
HEADER;
ENDSEC;
DATA;
#1= IFCCARTESIANPOINT((1.0,2.0,3.0));
#2= IFCCARTESIANPOINT((4.0,5.0,6.0));
ENDSEC;
END-ISO-10303-21;
"#;
        let pts = parse_ifc_points(ifc);
        assert_eq!(pts.len(), 2);
        assert!((pts[0].x - 1.0).abs() < 1e-5);
        assert!((pts[0].y - 2.0).abs() < 1e-5);
        assert!((pts[0].z - 3.0).abs() < 1e-5);
        assert!((pts[1].x - 4.0).abs() < 1e-5);
    }

    #[test]
    fn skips_2d_points() {
        let ifc = r#"
#1= IFCCARTESIANPOINT((1.0,2.0));
#2= IFCCARTESIANPOINT((4.0,5.0,6.0));
"#;
        let pts = parse_ifc_points(ifc);
        assert_eq!(pts.len(), 1);
        assert!((pts[0].x - 4.0).abs() < 1e-5);
    }

    #[test]
    fn empty_file_returns_empty() {
        let pts = parse_ifc_points("ISO-10303-21;\nHEADER;\nENDSEC;\n");
        assert!(pts.is_empty());
    }
}
