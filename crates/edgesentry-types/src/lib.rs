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

/// A tracked entity in the physical space.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Entity {
    pub id: String,
    pub class: EntityClass,
    /// Position in metres relative to site origin.
    pub position: Vec2,
    /// Velocity in m/s.
    pub velocity: Vec2,
    /// Unix timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// CV model confidence for this detection (0.0–1.0). None means not provided (treated as 1.0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
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
