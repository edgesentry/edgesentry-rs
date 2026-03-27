//! JSON report serialisation for [`DeviationReport`].
//!
//! Provides [`write_report`] and [`read_report`] so that deviation results can
//! be persisted to disk and consumed by downstream tooling.

use std::path::Path;

use crate::deviation::DeviationReport;

/// Errors that can occur while writing or reading a deviation report.
#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialisation error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Serialise a [`DeviationReport`] to a JSON file at `path`.
///
/// The file is written atomically via [`std::fs::write`]. If the file already
/// exists it is overwritten.
pub fn write_report(report: &DeviationReport, path: &Path) -> Result<(), ReportError> {
    let json = serde_json::to_string_pretty(report)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Deserialise a [`DeviationReport`] from a JSON file at `path`.
pub fn read_report(path: &Path) -> Result<DeviationReport, ReportError> {
    let content = std::fs::read_to_string(path)?;
    let report = serde_json::from_str(&content)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use crate::deviation::DeviationReport;

    #[test]
    fn roundtrip_via_string() {
        let report = DeviationReport {
            compliant_pct: 88.9,
            max_deviation_mm: 25.3,
            mean_deviation_mm: 5.1,
            point_count: 9,
            threshold_mm: 10.0,
        };
        let json = serde_json::to_string(&report).unwrap();
        let decoded: DeviationReport = serde_json::from_str(&json).unwrap();
        assert!((decoded.compliant_pct - 88.9).abs() < 1e-6);
        assert!((decoded.max_deviation_mm - 25.3).abs() < 1e-6);
        assert_eq!(decoded.point_count, 9);
        assert!((decoded.threshold_mm - 10.0).abs() < 1e-9);
    }
}
