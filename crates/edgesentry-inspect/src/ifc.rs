//! IFC (Industry Foundation Classes) STEP-21 loader.
//!
//! Parses `IFCCARTESIANPOINT` entries from an IFC file and returns them as a
//! `Vec<Point3D>`. Only 3D points (three coordinate values) are included;
//! 2D points are silently skipped.
//!
//! Remote IFC files can be fetched with [`fetch_ifc_url`] before loading.

use std::io::Write as _;
use std::path::Path;

use trilink_core::Point3D;

/// Errors that can occur while loading an IFC file.
#[derive(Debug, thiserror::Error)]
pub enum IfcError {
    #[error("I/O error reading IFC file: {0}")]
    Io(#[from] std::io::Error),

    #[error("no 3D IFCCARTESIANPOINT entries found in file")]
    NoPoints,

    #[error("HTTP fetch failed: {0}")]
    Fetch(String),
}

/// Fetch an IFC file from `url` into a secure temp file and return it.
///
/// The [`tempfile::NamedTempFile`] is automatically deleted when dropped, so
/// callers must keep it alive for as long as the path is needed.
///
/// # Authentication
///
/// Pass `token` to send `Authorization: Bearer <token>`.  Leave `None` for
/// unauthenticated or self-authenticating pre-signed S3 URLs.
pub fn fetch_ifc_url(url: &str, token: Option<&str>) -> Result<tempfile::NamedTempFile, IfcError> {
    let resp = {
        let req = ureq::get(url);
        if let Some(t) = token {
            req.set("Authorization", &format!("Bearer {t}"))
        } else {
            req
        }
        .call()
        .map_err(|e| IfcError::Fetch(e.to_string()))?
    };

    let mut tmp = tempfile::Builder::new()
        .prefix("edgesentry-ifc-")
        .suffix(".ifc")
        .tempfile()
        .map_err(IfcError::Io)?;

    std::io::copy(&mut resp.into_reader(), tmp.as_file_mut()).map_err(IfcError::Io)?;
    tmp.flush().map_err(IfcError::Io)?;

    Ok(tmp)
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

    // ------------------------------------------------------------------
    // fetch_ifc_url tests (require mockito)
    // ------------------------------------------------------------------

    const MINIMAL_IFC: &str = "#1= IFCCARTESIANPOINT((1.0,2.0,3.0));";

    #[test]
    fn fetch_ifc_url_downloads_content() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("GET", "/wall.ifc")
            .with_status(200)
            .with_body(MINIMAL_IFC)
            .create();

        let tmp = fetch_ifc_url(&format!("{}/wall.ifc", server.url()), None).unwrap();
        let content = std::fs::read_to_string(tmp.path()).unwrap();
        assert_eq!(content.trim(), MINIMAL_IFC);
    }

    #[test]
    fn fetch_ifc_url_sends_bearer_token() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("GET", "/wall.ifc")
            .match_header("authorization", "Bearer test-token-abc")
            .with_status(200)
            .with_body(MINIMAL_IFC)
            .create();

        fetch_ifc_url(&format!("{}/wall.ifc", server.url()), Some("test-token-abc")).unwrap();
    }

    #[test]
    fn fetch_ifc_url_404_returns_error() {
        let mut server = mockito::Server::new();
        let _m = server.mock("GET", "/missing.ifc").with_status(404).create();

        let result = fetch_ifc_url(&format!("{}/missing.ifc", server.url()), None);
        assert!(matches!(result, Err(IfcError::Fetch(_))));
    }
}
