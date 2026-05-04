use edgesentry_types::{Entity, EntityClass, Vec2};

/// 3D straight-line distance between two entities in metres.
///
/// Falls back to 2D (`euclidean_distance`) when either entity lacks a z-position.
pub fn euclidean_distance_3d(a: &Entity, b: &Entity) -> f32 {
    match (a.position_z, b.position_z) {
        (Some(az), Some(bz)) => {
            let dx = b.position.x - a.position.x;
            let dy = b.position.y - a.position.y;
            let dz = bz - az;
            (dx * dx + dy * dy + dz * dz).sqrt()
        }
        _ => euclidean_distance(a, b),
    }
}

/// Straight-line distance between two entities in metres.
pub fn euclidean_distance(a: &Entity, b: &Entity) -> f32 {
    (&b.position - &a.position).length()
}

/// Rate at which entity `b` is closing with entity `a`, in m/s.
///
/// Positive  → approaching (gap is shrinking).
/// Negative  → receding.
/// Zero      → perpendicular movement or identical velocities.
pub fn relative_velocity(a: &Entity, b: &Entity) -> f32 {
    let direction = &b.position - &a.position;
    if direction.length() < f32::EPSILON {
        return 0.0;
    }
    let unit = direction.normalize();
    // Project a's velocity onto the approach axis; subtract b's component.
    let rel = &a.velocity - &b.velocity;
    rel.dot(&unit)
}

/// Minimum stopping distance at `speed_ms` (m/s) for the given entity class, in metres.
///
/// Uses kinematic formula: d = v² / (2a).
/// Person is modelled as stopping instantly (returns 0.0 — conservative).
pub fn braking_distance(speed_ms: f32, entity_class: &EntityClass) -> f32 {
    let decel = entity_class.deceleration_ms2();
    if decel == f32::INFINITY {
        return 0.0; // Person: instant stop
    }
    if decel <= 0.0 || speed_ms <= 0.0 {
        return 0.0;
    }
    (speed_ms * speed_ms) / (2.0 * decel)
}

/// Time to collision given current gap and approach rate, in seconds.
///
/// Returns `f32::INFINITY` when entities are not approaching.
pub fn time_to_collision(distance_m: f32, approach_rate_ms: f32) -> f32 {
    if approach_rate_ms <= 0.0 {
        return f32::INFINITY;
    }
    distance_m / approach_rate_ms
}

/// Returns `true` if `pos` lies inside the polygon defined by `vertices`.
///
/// Uses the ray-casting algorithm. The polygon is treated as closed
/// (last vertex implicitly connects back to the first).
/// Returns `false` for degenerate polygons with fewer than 3 vertices.
pub fn zone_membership(pos: Vec2, polygon: &[Vec2]) -> bool {
    let n = polygon.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let vi = &polygon[i];
        let vj = &polygon[j];
        let crosses_y = (vi.y > pos.y) != (vj.y > pos.y);
        let x_intersect = (vj.x - vi.x) * (pos.y - vi.y) / (vj.y - vi.y) + vi.x;
        if crosses_y && pos.x < x_intersect {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgesentry_types::{Entity, EntityClass, Vec2};

    fn entity(x: f32, y: f32, vx: f32, vy: f32) -> Entity {
        Entity {
            id: "t".into(),
            class: EntityClass::Forklift,
            position: Vec2::new(x, y),
            position_z: None,
            velocity: Vec2::new(vx, vy),
            velocity_z: None,
            timestamp_ms: 0,
            sensor: None,
            computed_confidence: None,
        }
    }

    // ── euclidean_distance_3d ─────────────────────────────────────────────

    fn entity_3d(x: f32, y: f32, z: f32, vx: f32, vy: f32) -> Entity {
        let mut e = entity(x, y, vx, vy);
        e.position_z = Some(z);
        e
    }

    #[test]
    fn distance_3d_unit_cube_diagonal() {
        let a = entity_3d(0.0, 0.0, 0.0, 0.0, 0.0);
        let b = entity_3d(1.0, 1.0, 1.0, 0.0, 0.0);
        let d = euclidean_distance_3d(&a, &b);
        assert!((d - 3f32.sqrt()).abs() < 1e-5, "got {d}");
    }

    #[test]
    fn distance_3d_falls_back_to_2d_when_z_missing() {
        let a = entity(0.0, 0.0, 0.0, 0.0); // no position_z
        let b = entity_3d(3.0, 4.0, 5.0, 0.0, 0.0);
        // fallback: 2D distance = 5.0
        let d = euclidean_distance_3d(&a, &b);
        assert!((d - 5.0).abs() < 1e-5, "got {d}");
    }

    #[test]
    fn distance_3d_same_xy_different_z() {
        let a = entity_3d(0.0, 0.0, 0.0, 0.0, 0.0);
        let b = entity_3d(0.0, 0.0, 3.0, 0.0, 0.0);
        let d = euclidean_distance_3d(&a, &b);
        assert!((d - 3.0).abs() < 1e-5, "got {d}");
    }

    // ── euclidean_distance ────────────────────────────────────────────────

    #[test]
    fn distance_3_4_5_triangle() {
        let a = entity(0.0, 0.0, 0.0, 0.0);
        let b = entity(3.0, 4.0, 0.0, 0.0);
        assert!((euclidean_distance(&a, &b) - 5.0).abs() < 1e-5);
    }

    #[test]
    fn distance_same_position_is_zero() {
        let a = entity(2.0, 3.0, 0.0, 0.0);
        assert!((euclidean_distance(&a, &a)).abs() < 1e-5);
    }

    // ── relative_velocity ─────────────────────────────────────────────────

    #[test]
    fn approaching_head_on() {
        // a moves right at 2 m/s toward stationary b at x=10
        let a = entity(0.0, 0.0, 2.0, 0.0);
        let b = entity(10.0, 0.0, 0.0, 0.0);
        let rv = relative_velocity(&a, &b);
        assert!((rv - 2.0).abs() < 1e-4, "got {rv}");
    }

    #[test]
    fn receding_is_negative() {
        let a = entity(0.0, 0.0, -2.0, 0.0);
        let b = entity(10.0, 0.0, 0.0, 0.0);
        assert!(relative_velocity(&a, &b) < 0.0);
    }

    #[test]
    fn perpendicular_motion_is_zero() {
        // a moves up, b is to the right — no approach component
        let a = entity(0.0, 0.0, 0.0, 2.0);
        let b = entity(10.0, 0.0, 0.0, 0.0);
        let rv = relative_velocity(&a, &b);
        assert!(rv.abs() < 1e-4, "got {rv}");
    }

    #[test]
    fn same_position_returns_zero() {
        let a = entity(5.0, 5.0, 1.0, 0.0);
        let b = entity(5.0, 5.0, 0.0, 0.0);
        assert_eq!(relative_velocity(&a, &b), 0.0);
    }

    // ── braking_distance ─────────────────────────────────────────────────

    #[test]
    fn forklift_10kmh() {
        // 10 km/h = 2.778 m/s, decel = 1.5 m/s² → d = 7.716/3.0 ≈ 2.572 m
        let d = braking_distance(2.778, &EntityClass::Forklift);
        assert!((d - 2.572).abs() < 0.01, "got {d}");
    }

    #[test]
    fn person_stops_instantly() {
        assert_eq!(braking_distance(1.5, &EntityClass::Person), 0.0);
    }

    #[test]
    fn zero_speed_is_zero_distance() {
        assert_eq!(braking_distance(0.0, &EntityClass::Forklift), 0.0);
    }

    #[test]
    fn vessel_long_stopping_distance() {
        // Vessel at 3 knots ≈ 1.54 m/s, decel = 0.05 m/s² → d ≈ 23.7 m
        let d = braking_distance(1.54, &EntityClass::Vessel);
        assert!(d > 20.0, "expected >20 m, got {d}");
    }

    // ── time_to_collision ─────────────────────────────────────────────────

    #[test]
    fn ttc_basic() {
        assert!((time_to_collision(10.0, 5.0) - 2.0).abs() < 1e-5);
    }

    #[test]
    fn ttc_not_approaching_is_infinity() {
        assert_eq!(time_to_collision(10.0, 0.0), f32::INFINITY);
        assert_eq!(time_to_collision(10.0, -1.0), f32::INFINITY);
    }

    // ── zone_membership ───────────────────────────────────────────────────

    fn unit_square() -> Vec<Vec2> {
        vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ]
    }

    #[test]
    fn centre_is_inside() {
        assert!(zone_membership(Vec2::new(5.0, 5.0), &unit_square()));
    }

    #[test]
    fn outside_is_false() {
        assert!(!zone_membership(Vec2::new(15.0, 5.0), &unit_square()));
        assert!(!zone_membership(Vec2::new(-1.0, 5.0), &unit_square()));
    }

    #[test]
    fn degenerate_polygon_is_false() {
        let poly = vec![Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0)];
        assert!(!zone_membership(Vec2::new(0.5, 0.0), &poly));
    }

    #[test]
    fn triangle_inside_and_outside() {
        // Right triangle with vertices (0,0), (10,0), (0,10)
        let tri = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(0.0, 10.0),
        ];
        assert!(zone_membership(Vec2::new(1.0, 1.0), &tri));
        assert!(!zone_membership(Vec2::new(8.0, 8.0), &tri));
    }

    #[test]
    fn empty_polygon_is_false() {
        assert!(!zone_membership(Vec2::new(0.0, 0.0), &[]));
    }

    // ── Edge cases ────────────────────────────────────────────────────────

    #[test]
    fn both_entities_moving_toward_each_other() {
        // a at 0 moving right at 1, b at 10 moving left at 1 → approach rate = 2
        let a = entity(0.0, 0.0, 1.0, 0.0);
        let b = entity(10.0, 0.0, -1.0, 0.0);
        let rv = relative_velocity(&a, &b);
        assert!((rv - 2.0).abs() < 1e-4, "got {rv}");
    }

    #[test]
    fn both_entities_moving_same_direction_same_speed() {
        let a = entity(0.0, 0.0, 2.0, 0.0);
        let b = entity(10.0, 0.0, 2.0, 0.0);
        // No closing speed — parallel movement
        let rv = relative_velocity(&a, &b);
        assert!(rv.abs() < 1e-4, "got {rv}");
    }

    #[test]
    fn distance_is_symmetric() {
        let a = entity(1.0, 2.0, 0.0, 0.0);
        let b = entity(4.0, 6.0, 0.0, 0.0);
        assert!((euclidean_distance(&a, &b) - euclidean_distance(&b, &a)).abs() < 1e-5);
    }

    #[test]
    fn braking_distance_reach_stacker_vs_forklift() {
        // Reach stacker decelerates slower → longer stopping distance at same speed
        let speed = 3.0;
        let forklift = braking_distance(speed, &EntityClass::Forklift);
        let stacker = braking_distance(speed, &EntityClass::ReachStacker);
        assert!(stacker > forklift, "stacker {stacker} should be > forklift {forklift}");
    }

    #[test]
    fn ttc_scales_with_distance() {
        // Double the distance → double the TTC
        let ttc_near = time_to_collision(5.0, 2.5);
        let ttc_far = time_to_collision(10.0, 2.5);
        assert!((ttc_far - 2.0 * ttc_near).abs() < 1e-5);
    }

    // ── Realistic scenario: MPA clearance breach ──────────────────────────
    //
    // Forklift FL-01 at (0, 0) moving toward Worker W-03 at (3.2, 0) at 1.4 m/s.
    // A typical site safety rule requires ≥ 5.0 m clearance.
    //
    // Expected outputs from the roadmap demo:
    //   distance          = 3.2 m   (< 5.0 m threshold → rule fires)
    //   approach_rate     ≈ 1.4 m/s
    //   braking_distance  ≈ 0.65 m  (at 1.4 m/s, Forklift decel 1.5 m/s²)
    //   TTC               ≈ 2.3 s

    #[test]
    fn scenario_mpa_clearance_breach_distance() {
        let forklift = entity(0.0, 0.0, 1.4, 0.0);
        let worker = entity(3.2, 0.0, 0.0, 0.0);
        let dist = euclidean_distance(&forklift, &worker);
        assert!((dist - 3.2).abs() < 1e-4);
        assert!(dist < 5.0, "clearance {dist}m breaches 5.0m threshold");
    }

    #[test]
    fn scenario_mpa_clearance_breach_approach_rate() {
        let forklift = entity(0.0, 0.0, 1.4, 0.0);
        let worker = entity(3.2, 0.0, 0.0, 0.0);
        let rv = relative_velocity(&forklift, &worker);
        assert!((rv - 1.4).abs() < 1e-4, "approach rate {rv} m/s");
    }

    #[test]
    fn scenario_mpa_clearance_breach_ttc() {
        // distance=3.2m, approach_rate=1.4 m/s → TTC = 3.2/1.4 ≈ 2.286 s
        let ttc = time_to_collision(3.2, 1.4);
        assert!((ttc - 2.286).abs() < 0.01, "TTC {ttc} s");
    }

    #[test]
    fn scenario_mpa_clearance_breach_braking_distance() {
        // At 1.4 m/s with decel 1.5 m/s²: d = 1.96/3.0 ≈ 0.653 m
        let bd = braking_distance(1.4, &EntityClass::Forklift);
        assert!((bd - 0.653).abs() < 0.01, "braking distance {bd} m");
    }

    #[test]
    fn scenario_safe_pass_no_breach() {
        // Forklift passes at 6.0 m clearance — above the 5.0 m threshold, no rule fire
        let forklift = entity(0.0, 0.0, 1.0, 0.0);
        let worker = entity(6.0, 0.0, 0.0, 0.0);
        let dist = euclidean_distance(&forklift, &worker);
        assert!(dist >= 5.0, "6 m clearance should not breach 5 m threshold");
    }
}
