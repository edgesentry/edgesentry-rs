use edgesentry_types::{Entity, SourceType};

/// Maximum age (ms) before an entity's position reading is considered stale.
fn max_age_ms(source: &SourceType) -> f32 {
    match source {
        SourceType::Ais => 30_000.0,
        SourceType::Lidar | SourceType::Uwb => 5_000.0,
        SourceType::ComputerVision | SourceType::Radar => 10_000.0,
        SourceType::PointSensor => 10_000.0,
        SourceType::Simulation => f32::INFINITY,
    }
}

/// Base confidence when no sensor-provided score is available.
fn base_confidence(source: &SourceType, detection_confidence: Option<f32>) -> f32 {
    match (source, detection_confidence) {
        (SourceType::ComputerVision, Some(c)) => c,
        (SourceType::Radar, Some(c))          => c,
        (SourceType::Ais, _)                  => 1.00,
        (SourceType::Lidar, _)                => 0.95,
        (SourceType::Uwb, _)                  => 0.90,
        (SourceType::PointSensor, _)          => 0.80,
        // CV/Radar without a score, or Simulation
        _                                     => 0.30,
    }
}

/// Context required to compute entity confidence.
///
/// `now_ms` is the current pipeline clock in milliseconds.
/// `drift_score` is the sensor's homography/calibration drift in [0.0, 1.0];
/// 0.0 = perfectly calibrated, 1.0 = fully drifted / uncalibrated.
#[derive(Debug, Clone)]
pub struct ConfidenceContext {
    pub now_ms: u64,
    pub drift_score: f32,
}

/// Compute a unified confidence score (0.0–1.0) for a single entity.
///
/// Formula:
/// ```text
/// computed = clamp((base + stddev_adj) × freshness × calib_mult, 0.0, 1.0)
/// ```
///
/// Returns `None` for `Simulation` entities — evidence quality concept does not apply.
pub fn compute_entity_confidence(entity: &Entity, ctx: &ConfidenceContext) -> Option<f32> {
    let reading = entity.sensor.as_ref()?;

    if reading.source_type == SourceType::Simulation {
        return None;
    }

    let base = base_confidence(&reading.source_type, reading.detection_confidence);

    // stddev adjustment
    let stddev_adj = match reading.position_stddev_m {
        Some(s) if s < 0.1 => 0.05,
        Some(s) if s > 2.0 => -0.20,
        _ => 0.0,
    };

    // freshness: fraction of max age remaining
    let max_age = max_age_ms(&reading.source_type);
    let elapsed = (ctx.now_ms.saturating_sub(entity.timestamp_ms)) as f32;
    let freshness = (1.0 - elapsed / max_age).max(0.0);

    // calibration multiplier
    let calib_mult = if ctx.drift_score >= 0.6 {
        0.7
    } else if ctx.drift_score >= 0.3 {
        0.9
    } else {
        1.0
    };

    let score = ((base + stddev_adj) * freshness * calib_mult).clamp(0.0, 1.0);
    Some(score)
}

/// Calibration state derived from sensor drift and average computed confidence.
#[derive(Debug, Clone, PartialEq)]
pub enum CalibrationStatus {
    Valid,
    Degraded,
    Uncalibrated,
}

/// Classify calibration state from drift score and recent average computed confidence.
///
/// High drift or low average confidence both indicate the sensor needs recalibration.
pub fn calibration_status(drift_score: f32, avg_computed_confidence: f32) -> CalibrationStatus {
    if drift_score >= 0.6 || avg_computed_confidence < 0.5 {
        CalibrationStatus::Uncalibrated
    } else if drift_score >= 0.3 || avg_computed_confidence < 0.7 {
        CalibrationStatus::Degraded
    } else {
        CalibrationStatus::Valid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgesentry_types::{Entity, EntityClass, SensorReading, Vec2};

    fn ctx(now_ms: u64, drift: f32) -> ConfidenceContext {
        ConfidenceContext { now_ms, drift_score: drift }
    }

    fn entity_with_reading(reading: SensorReading, timestamp_ms: u64) -> Entity {
        Entity {
            id: "t".into(),
            class: EntityClass::Vessel,
            position: Vec2::new(0.0, 0.0),
            position_z: None,
            velocity: Vec2::new(0.0, 0.0),
            velocity_z: None,
            timestamp_ms,
            sensor: Some(reading),
            computed_confidence: None,
        }
    }

    // ── base confidence ───────────────────────────────────────────────────

    #[test]
    fn ais_fresh_no_drift_is_one() {
        let e = entity_with_reading(SensorReading::ais(), 1000);
        let score = compute_entity_confidence(&e, &ctx(1000, 0.0)).unwrap();
        assert!((score - 1.0).abs() < 1e-4, "got {score}");
    }

    #[test]
    fn lidar_fresh_no_drift() {
        let reading = SensorReading {
            source_type: SourceType::Lidar,
            dimensions: 3,
            detection_confidence: None,
            position_stddev_m: None,
            position_stddev_z_m: None,
        };
        let e = entity_with_reading(reading, 1000);
        let score = compute_entity_confidence(&e, &ctx(1000, 0.0)).unwrap();
        assert!((score - 0.95).abs() < 1e-4, "got {score}");
    }

    #[test]
    fn cv_with_score_used_directly() {
        let e = entity_with_reading(SensorReading::cv(0.87), 1000);
        let score = compute_entity_confidence(&e, &ctx(1000, 0.0)).unwrap();
        assert!((score - 0.87).abs() < 1e-4, "got {score}");
    }

    #[test]
    fn cv_without_score_is_low() {
        let reading = SensorReading {
            source_type: SourceType::ComputerVision,
            dimensions: 2,
            detection_confidence: None,
            position_stddev_m: None,
            position_stddev_z_m: None,
        };
        let e = entity_with_reading(reading, 1000);
        let score = compute_entity_confidence(&e, &ctx(1000, 0.0)).unwrap();
        assert!((score - 0.30).abs() < 1e-4, "got {score}");
    }

    #[test]
    fn no_sensor_returns_none() {
        let e = Entity {
            id: "t".into(),
            class: EntityClass::Forklift,
            position: Vec2::new(0.0, 0.0),
            position_z: None,
            velocity: Vec2::new(0.0, 0.0),
            velocity_z: None,
            timestamp_ms: 1000,
            sensor: None,
            computed_confidence: None,
        };
        assert!(compute_entity_confidence(&e, &ctx(1000, 0.0)).is_none());
    }

    #[test]
    fn simulation_returns_none() {
        let e = entity_with_reading(SensorReading::simulation(), 1000);
        assert!(compute_entity_confidence(&e, &ctx(1000, 0.0)).is_none());
    }

    // ── stddev adjustment ─────────────────────────────────────────────────

    #[test]
    fn tight_stddev_adds_bonus() {
        let reading = SensorReading {
            source_type: SourceType::Lidar,
            dimensions: 3,
            detection_confidence: None,
            position_stddev_m: Some(0.05),
            position_stddev_z_m: None,
        };
        let e = entity_with_reading(reading, 1000);
        let score = compute_entity_confidence(&e, &ctx(1000, 0.0)).unwrap();
        assert!((score - 1.0).abs() < 1e-4, "0.95 + 0.05 = 1.0, got {score}");
    }

    #[test]
    fn wide_stddev_subtracts_penalty() {
        let reading = SensorReading {
            source_type: SourceType::Lidar,
            dimensions: 2,
            detection_confidence: None,
            position_stddev_m: Some(3.0),
            position_stddev_z_m: None,
        };
        let e = entity_with_reading(reading, 1000);
        let score = compute_entity_confidence(&e, &ctx(1000, 0.0)).unwrap();
        assert!((score - 0.75).abs() < 1e-4, "0.95 - 0.20 = 0.75, got {score}");
    }

    // ── freshness ─────────────────────────────────────────────────────────

    #[test]
    fn ais_stale_at_30s_is_zero() {
        let e = entity_with_reading(SensorReading::ais(), 0);
        // now = 30001 ms → elapsed > 30 s max_age → freshness = 0
        let score = compute_entity_confidence(&e, &ctx(30_001, 0.0)).unwrap();
        assert!(score < 1e-4, "stale AIS should be ~0, got {score}");
    }

    #[test]
    fn ais_half_age_half_score() {
        let e = entity_with_reading(SensorReading::ais(), 0);
        // elapsed = 15 s → freshness = 0.5
        let score = compute_entity_confidence(&e, &ctx(15_000, 0.0)).unwrap();
        assert!((score - 0.5).abs() < 1e-4, "got {score}");
    }

    // ── drift ─────────────────────────────────────────────────────────────

    #[test]
    fn high_drift_reduces_score() {
        let e = entity_with_reading(SensorReading::ais(), 1000);
        let good  = compute_entity_confidence(&e, &ctx(1000, 0.0)).unwrap();
        let drift = compute_entity_confidence(&e, &ctx(1000, 0.7)).unwrap();
        assert!(drift < good, "drift={drift} should be < good={good}");
        assert!((drift - 0.7).abs() < 1e-4, "1.0 × 0.7 = 0.70, got {drift}");
    }

    #[test]
    fn medium_drift_gives_0_9_multiplier() {
        let e = entity_with_reading(SensorReading::ais(), 1000);
        let score = compute_entity_confidence(&e, &ctx(1000, 0.4)).unwrap();
        assert!((score - 0.9).abs() < 1e-4, "1.0 × 0.9 = 0.90, got {score}");
    }

    // ── score clamped to [0, 1] ───────────────────────────────────────────

    #[test]
    fn score_never_exceeds_one() {
        let reading = SensorReading {
            source_type: SourceType::Lidar,
            dimensions: 3,
            detection_confidence: None,
            position_stddev_m: Some(0.05), // +0.05 bonus
            position_stddev_z_m: None,
        };
        let e = entity_with_reading(reading, 1000);
        let score = compute_entity_confidence(&e, &ctx(1000, 0.0)).unwrap();
        assert!(score <= 1.0, "score {score} must not exceed 1.0");
    }

    #[test]
    fn score_never_below_zero() {
        let reading = SensorReading {
            source_type: SourceType::ComputerVision,
            dimensions: 2,
            detection_confidence: Some(0.1),
            position_stddev_m: Some(5.0), // -0.20 penalty
            position_stddev_z_m: None,
        };
        let e = entity_with_reading(reading, 0);
        // Stale + penalty + low base — clamp ensures ≥ 0
        let score = compute_entity_confidence(&e, &ctx(30_000, 0.9)).unwrap();
        assert!(score >= 0.0, "score {score} must not go below 0.0");
    }

    // ── calibration_status ────────────────────────────────────────────────

    #[test]
    fn valid_when_low_drift_high_confidence() {
        assert_eq!(calibration_status(0.1, 0.9), CalibrationStatus::Valid);
    }

    #[test]
    fn degraded_on_medium_drift() {
        assert_eq!(calibration_status(0.4, 0.9), CalibrationStatus::Degraded);
    }

    #[test]
    fn degraded_on_low_confidence() {
        assert_eq!(calibration_status(0.1, 0.6), CalibrationStatus::Degraded);
    }

    #[test]
    fn uncalibrated_on_high_drift() {
        assert_eq!(calibration_status(0.7, 0.9), CalibrationStatus::Uncalibrated);
    }

    #[test]
    fn uncalibrated_on_very_low_confidence() {
        assert_eq!(calibration_status(0.1, 0.3), CalibrationStatus::Uncalibrated);
    }
}
