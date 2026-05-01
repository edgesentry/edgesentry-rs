use edgesentry_types::Vec2;

const M_PER_DEG: f64 = 111_319.9;

/// Convert WGS-84 (lat, lon) to site-local (x=east, y=north) in metres.
/// Uses equirectangular approximation — accurate to < 1 m within port/terminal scale (< 5 km).
pub fn latlon_to_local(lat: f64, lon: f64, ref_lat: f64, ref_lon: f64) -> (f32, f32) {
    let dy = (lat - ref_lat) * M_PER_DEG;
    let dx = (lon - ref_lon) * M_PER_DEG * ref_lat.to_radians().cos();
    (dx as f32, dy as f32)
}

/// Convert COG (degrees true North) + SOG (knots) to velocity Vec2 (m/s, x=east y=north).
pub fn cog_sog_to_velocity(cog_deg: f32, sog_knots: f32) -> Vec2 {
    let sog_ms = sog_knots * 0.5144_f32;
    let rad = cog_deg.to_radians();
    Vec2::new(rad.sin() * sog_ms, rad.cos() * sog_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── latlon_to_local tests ─────────────────────────────────────────────

    #[test]
    fn latlon_to_local_at_reference_is_zero() {
        let (x, y) = latlon_to_local(1.2640, 103.8200, 1.2640, 103.8200);
        assert!(x.abs() < 1e-3, "x={x}");
        assert!(y.abs() < 1e-3, "y={y}");
    }

    #[test]
    fn latlon_to_local_one_degree_north() {
        // One degree north of reference ≈ 111320 m north
        let (x, y) = latlon_to_local(2.2640, 103.8200, 1.2640, 103.8200);
        assert!(x.abs() < 1.0, "x should be ~0, got {x}");
        assert!((y - 111_319.9_f32).abs() < 200.0, "y should be ~111319.9, got {y}");
    }

    #[test]
    fn latlon_to_local_zone_boundary_positions() {
        let ref_lat = 1.2640_f64;
        let ref_lon = 103.8200_f64;

        // 1.2615°N → approximately -278 m south
        let (_, y1) = latlon_to_local(1.2615, ref_lon, ref_lat, ref_lon);
        assert!(y1 < -200.0, "y1={y1}: expected ~-278m");
        assert!(y1 > -350.0, "y1={y1}: should not be too far south");

        // 1.2658°N → approximately +200 m north (zone boundary)
        let (_, y2) = latlon_to_local(1.2658, ref_lon, ref_lat, ref_lon);
        assert!(y2 > 150.0, "y2={y2}: expected ~+200m");
        assert!(y2 < 250.0, "y2={y2}: should be near zone boundary");
    }

    // ── cog_sog_to_velocity tests ─────────────────────────────────────────

    #[test]
    fn cog_sog_velocity_north() {
        // Heading 0° (north), 1 knot → (0, 0.5144) m/s
        let v = cog_sog_to_velocity(0.0, 1.0);
        assert!(v.x.abs() < 1e-4, "vx should be ~0, got {}", v.x);
        assert!((v.y - 0.5144).abs() < 1e-3, "vy should be ~0.5144, got {}", v.y);
    }

    #[test]
    fn cog_sog_velocity_east() {
        // Heading 90° (east), 1 knot → (0.5144, ~0) m/s
        let v = cog_sog_to_velocity(90.0, 1.0);
        assert!((v.x - 0.5144).abs() < 1e-3, "vx should be ~0.5144, got {}", v.x);
        assert!(v.y.abs() < 1e-4, "vy should be ~0, got {}", v.y);
    }

    #[test]
    fn cog_sog_velocity_south() {
        // Heading 180° (south), 1 knot → (0, -0.5144) m/s
        let v = cog_sog_to_velocity(180.0, 1.0);
        assert!(v.x.abs() < 1e-3, "vx should be ~0, got {}", v.x);
        assert!((v.y + 0.5144).abs() < 1e-3, "vy should be ~-0.5144, got {}", v.y);
    }
}
