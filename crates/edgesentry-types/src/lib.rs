/// 2D position or velocity vector (metres or m/s).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len < f32::EPSILON {
            Self::new(0.0, 0.0)
        } else {
            Self::new(self.x / len, self.y / len)
        }
    }

    pub fn dot(&self, other: &Self) -> f32 {
        self.x * other.x + self.y * other.y
    }
}

impl std::ops::Sub for &Vec2 {
    type Output = Vec2;
    fn sub(self, other: &Vec2) -> Vec2 {
        Vec2::new(self.x - other.x, self.y - other.y)
    }
}

/// Physical class of an entity, used to look up braking parameters.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EntityClass {
    /// Counterbalanced forklift up to 3.5 T
    Forklift,
    /// Reach stacker / empty container handler
    ReachStacker,
    /// Terminal tractor / yard truck
    TerminalTractor,
    /// Vessel (ship) — very slow deceleration
    Vessel,
    /// Walking person — modelled as stopping instantly (conservative)
    Person,
    /// Synthetic entity emitted when an AIS vessel has not been heard for > threshold seconds.
    /// `velocity.x` encodes the gap duration in seconds; `velocity.y` is 0.
    AisGap,
}

impl EntityClass {
    /// Maximum service-brake deceleration in m/s².
    /// Person returns f32::INFINITY (stops instantly — safest assumption).
    /// AisGap is a synthetic marker and uses 0.0 (not applicable).
    pub fn deceleration_ms2(&self) -> f32 {
        match self {
            EntityClass::Forklift => 1.5,
            EntityClass::ReachStacker => 1.0,
            EntityClass::TerminalTractor => 2.0,
            EntityClass::Vessel => 0.05,
            EntityClass::Person => f32::INFINITY,
            EntityClass::AisGap => 0.0,
        }
    }
}

/// The type of sensor or data source that produced an entity reading.
/// Determines how `EvidenceQuality` is computed — some sources have meaningful
/// detection confidence (CV, Radar), others are inherently authoritative (AIS,
/// LiDAR), and simulation data is not applicable to evidence quality at all.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SourceType {
    /// Computer vision model (YOLO et al.) — detection_confidence is meaningful.
    ComputerVision,
    /// AIS NMEA — vessel self-reports its own GPS position; no CV involved.
    Ais,
    /// LiDAR — direct range measurement; sub-centimetre accuracy; no detection model.
    Lidar,
    /// Radar — has a detection probability; treat like CV if provided.
    Radar,
    /// UWB tag — RF positioning; high accuracy; no detection confidence needed.
    Uwb,
    /// Point sensor (light curtain, PIR, area sensor) — binary zone signal; no confidence.
    PointSensor,
    /// Simulation / synthetic data — evidence quality concept does not apply.
    Simulation,
}

fn default_dimensions() -> u8 { 2 }

/// Sensor reading metadata attached to an entity detection.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SensorReading {
    pub source_type: SourceType,
    /// Number of spatial dimensions this sensor operates in: 1, 2, or 3.
    /// Defaults to 2 for backward compatibility.
    #[serde(default = "default_dimensions")]
    pub dimensions: u8,
    /// Detection confidence (0.0–1.0). Meaningful for `ComputerVision` and `Radar` only;
    /// `None` for sources where detection confidence is not a relevant concept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detection_confidence: Option<f32>,
    /// Estimated 1-sigma horizontal position accuracy in metres, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position_stddev_m: Option<f32>,
    /// Estimated 1-sigma vertical (z-axis) position accuracy in metres.
    /// Only relevant when `dimensions == 3`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position_stddev_z_m: Option<f32>,
}

impl SensorReading {
    pub fn cv(confidence: f32) -> Self {
        Self { source_type: SourceType::ComputerVision, dimensions: 2, detection_confidence: Some(confidence), position_stddev_m: None, position_stddev_z_m: None }
    }
    pub fn ais() -> Self {
        Self { source_type: SourceType::Ais, dimensions: 2, detection_confidence: None, position_stddev_m: None, position_stddev_z_m: None }
    }
    pub fn simulation() -> Self {
        Self { source_type: SourceType::Simulation, dimensions: 2, detection_confidence: None, position_stddev_m: None, position_stddev_z_m: None }
    }
}

/// A tracked entity in the physical space.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Entity {
    pub id: String,
    pub class: EntityClass,
    /// Position in metres relative to site origin (horizontal plane).
    pub position: Vec2,
    /// Vertical position in metres above site datum. Only set for 3-D sensors (LiDAR, UWB 3D).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position_z: Option<f32>,
    /// Velocity in m/s (horizontal plane).
    pub velocity: Vec2,
    /// Vertical velocity in m/s. Only set for 3-D sensors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub velocity_z: Option<f32>,
    /// Unix timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Sensor reading metadata. `None` means the source is unknown → treated as `Degraded`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sensor: Option<SensorReading>,
    /// Confidence score computed by EdgeSentry from all available signals
    /// (sensor type, detection_confidence, position accuracy, calibration state, etc.).
    /// `None` until the compute stage populates it. Placeholder — calculation logic is TBD.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub computed_confidence: Option<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Vec2 ──────────────────────────────────────────────────────────────

    #[test]
    fn vec2_length_unit_x() {
        assert!((Vec2::new(1.0, 0.0).length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn vec2_length_3_4_is_5() {
        assert!((Vec2::new(3.0, 4.0).length() - 5.0).abs() < 1e-5);
    }

    #[test]
    fn vec2_length_zero() {
        assert_eq!(Vec2::new(0.0, 0.0).length(), 0.0);
    }

    #[test]
    fn vec2_normalize_unit_x() {
        let n = Vec2::new(5.0, 0.0).normalize();
        assert!((n.x - 1.0).abs() < 1e-6);
        assert!(n.y.abs() < 1e-6);
    }

    #[test]
    fn vec2_normalize_45_degrees() {
        let n = Vec2::new(1.0, 1.0).normalize();
        let expected = 1.0_f32 / 2.0_f32.sqrt();
        assert!((n.x - expected).abs() < 1e-6);
        assert!((n.y - expected).abs() < 1e-6);
    }

    #[test]
    fn vec2_normalize_zero_vector_is_zero() {
        let n = Vec2::new(0.0, 0.0).normalize();
        assert_eq!(n, Vec2::new(0.0, 0.0));
    }

    #[test]
    fn vec2_dot_orthogonal_is_zero() {
        let a = Vec2::new(1.0, 0.0);
        let b = Vec2::new(0.0, 1.0);
        assert!((a.dot(&b)).abs() < 1e-6);
    }

    #[test]
    fn vec2_dot_parallel() {
        let a = Vec2::new(3.0, 0.0);
        let b = Vec2::new(4.0, 0.0);
        assert!((a.dot(&b) - 12.0).abs() < 1e-5);
    }

    #[test]
    fn vec2_sub() {
        let a = Vec2::new(5.0, 3.0);
        let b = Vec2::new(2.0, 1.0);
        let r = &a - &b;
        assert_eq!(r, Vec2::new(3.0, 2.0));
    }

    // ── EntityClass ───────────────────────────────────────────────────────

    #[test]
    fn entity_class_deceleration_ordering() {
        // TerminalTractor brakes harder than Forklift, which brakes harder than Vessel
        assert!(
            EntityClass::TerminalTractor.deceleration_ms2()
                > EntityClass::Forklift.deceleration_ms2()
        );
        assert!(
            EntityClass::Forklift.deceleration_ms2()
                > EntityClass::Vessel.deceleration_ms2()
        );
    }

    #[test]
    fn entity_class_person_is_infinity() {
        assert_eq!(EntityClass::Person.deceleration_ms2(), f32::INFINITY);
    }

    #[test]
    fn entity_class_all_positive() {
        for class in &[
            EntityClass::Forklift,
            EntityClass::ReachStacker,
            EntityClass::TerminalTractor,
            EntityClass::Vessel,
        ] {
            assert!(
                class.deceleration_ms2() > 0.0,
                "{class:?} deceleration should be positive"
            );
        }
    }
}
