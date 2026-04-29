use std::collections::BTreeMap;
use std::thread;
use std::time::Duration;

/// Configuration for scenario generation.
pub struct ScenarioConfig {
    /// Number of entities to simulate (default 2).
    pub entity_count: usize,
    /// Number of frames to generate (default 10).
    pub frame_count: usize,
    /// Frames per second (default 10).
    pub fps: u32,
    /// Field size in metres (default 20.0).
    pub bounds: f32,
}

impl Default for ScenarioConfig {
    fn default() -> Self {
        Self {
            entity_count: 2,
            frame_count: 10,
            fps: 10,
            bounds: 20.0,
        }
    }
}

/// Advance an LCG seed one step.
fn lcg_next(seed: u64) -> u64 {
    seed.wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407)
}

/// Map a seed to a float in [0.0, 1.0).
fn lcg_f32(seed: u64) -> f32 {
    // Use upper 32 bits for better distribution.
    ((seed >> 32) as f32) / (u32::MAX as f32 + 1.0)
}

/// Generate synthetic entity CSV content.
///
/// Entities start at random positions within bounds and move linearly.
/// Returns CSV string (header + rows).
///
/// Header: `timestamp_ms,entity_id,entity_type,x,y,vx,vy`
pub fn generate_entity_csv(config: &ScenarioConfig, seed: u64) -> String {
    let mut s = seed;
    let frame_interval_ms: u64 = if config.fps == 0 { 100 } else { 1000 / config.fps as u64 };

    struct EntityState {
        id: String,
        entity_type: &'static str,
        x: f32,
        y: f32,
        vx: f32,
        vy: f32,
    }

    let mut entities = Vec::with_capacity(config.entity_count);
    for i in 0..config.entity_count {
        s = lcg_next(s);
        let x = lcg_f32(s) * config.bounds;
        s = lcg_next(s);
        let y = lcg_f32(s) * config.bounds;
        s = lcg_next(s);
        // Velocity in [-1.5, 1.5] m/s
        let vx = (lcg_f32(s) - 0.5) * 3.0;
        s = lcg_next(s);
        let vy = (lcg_f32(s) - 0.5) * 3.0;

        let entity_type = if i % 2 == 0 { "Forklift" } else { "Person" };
        entities.push(EntityState {
            id: format!("E-{:02}", i + 1),
            entity_type,
            x,
            y,
            vx,
            vy,
        });
    }

    let mut out = String::from("timestamp_ms,entity_id,entity_type,x,y,vx,vy\n");

    for frame in 0..config.frame_count {
        let timestamp_ms = frame as u64 * frame_interval_ms;
        let t = frame as f32 * (frame_interval_ms as f32 / 1000.0);
        for e in &entities {
            let x = e.x + e.vx * t;
            let y = e.y + e.vy * t;
            out.push_str(&format!(
                "{},{},{},{:.4},{:.4},{:.4},{:.4}\n",
                timestamp_ms, e.id, e.entity_type, x, y, e.vx, e.vy
            ));
        }
    }

    out
}

/// Simulate: read CSV, send each frame as JSON over UDP.
///
/// Uses `std::net::UdpSocket` — no async, no tokio.
///
/// Parse CSV row by row, group rows by `timestamp_ms` into frames.
/// For each frame: serialize the entities as JSON array and send via UDP.
/// Sleeps between frames to match fps.
/// Returns the number of frames sent.
pub fn simulate_from_csv(csv: &str, target_addr: &str, fps: u32) -> Result<usize, String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0")
        .map_err(|e| format!("bind failed: {e}"))?;

    // Build frame map from CSV.
    let mut lines = csv.lines();
    let header = lines.next().ok_or_else(|| "CSV has no header".to_string())?;
    // Find column indices.
    let cols: Vec<&str> = header.split(',').collect();
    let idx = |name: &str| -> Result<usize, String> {
        cols.iter()
            .position(|&c| c.trim() == name)
            .ok_or_else(|| format!("column '{name}' not found in header"))
    };
    let ts_idx = idx("timestamp_ms")?;
    let id_idx = idx("entity_id")?;
    let type_idx = idx("entity_type")?;
    let x_idx = idx("x")?;
    let y_idx = idx("y")?;
    let vx_idx = idx("vx")?;
    let vy_idx = idx("vy")?;

    let mut frames: BTreeMap<u64, Vec<serde_json::Value>> = BTreeMap::new();

    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        let max_idx = [ts_idx, id_idx, type_idx, x_idx, y_idx, vx_idx, vy_idx]
            .iter()
            .copied()
            .max()
            .unwrap_or(0);
        if fields.len() <= max_idx {
            continue;
        }
        let ts: u64 = fields[ts_idx].trim().parse().unwrap_or(0);
        let entity_id = fields[id_idx].trim().to_string();
        let entity_type = fields[type_idx].trim().to_string();
        let x: f64 = fields[x_idx].trim().parse().unwrap_or(0.0);
        let y: f64 = fields[y_idx].trim().parse().unwrap_or(0.0);
        let vx: f64 = fields[vx_idx].trim().parse().unwrap_or(0.0);
        let vy: f64 = fields[vy_idx].trim().parse().unwrap_or(0.0);

        frames.entry(ts).or_default().push(serde_json::json!({
            "id": entity_id,
            "class": entity_type,
            "x": x,
            "y": y,
            "vx": vx,
            "vy": vy,
            "timestamp_ms": ts
        }));
    }

    let frame_interval_ms = if fps == 0 { 100u64 } else { 1000 / fps as u64 };
    let mut sent = 0usize;

    for entities in frames.values() {
        let packet = serde_json::json!({ "entities": entities });
        let payload =
            serde_json::to_vec(&packet).map_err(|e| format!("JSON serialize error: {e}"))?;
        socket
            .send_to(&payload, target_addr)
            .map_err(|e| format!("UDP send error: {e}"))?;
        sent += 1;
        thread::sleep(Duration::from_millis(frame_interval_ms));
    }

    Ok(sent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_entity_csv_correct_row_count() {
        let config = ScenarioConfig { entity_count: 2, frame_count: 10, fps: 10, bounds: 20.0 };
        let csv = generate_entity_csv(&config, 42);
        let lines: Vec<&str> = csv.lines().collect();
        // 1 header + entity_count * frame_count data rows
        assert_eq!(lines.len(), 1 + 2 * 10);
    }

    #[test]
    fn generate_entity_csv_has_header() {
        let config = ScenarioConfig::default();
        let csv = generate_entity_csv(&config, 0);
        assert!(csv.starts_with("timestamp_ms,entity_id,entity_type,x,y,vx,vy\n"));
    }

    #[test]
    fn generate_entity_csv_entity_types_alternate() {
        let config = ScenarioConfig { entity_count: 2, frame_count: 1, fps: 10, bounds: 20.0 };
        let csv = generate_entity_csv(&config, 1);
        let lines: Vec<&str> = csv.lines().skip(1).collect();
        // Entity 0 (even index) should be forklift, entity 1 (odd) pedestrian.
        assert!(lines[0].contains("Forklift"));
        assert!(lines[1].contains("Person"));
    }

    #[test]
    fn generate_entity_csv_different_seeds_differ() {
        let config = ScenarioConfig::default();
        let csv1 = generate_entity_csv(&config, 1);
        let csv2 = generate_entity_csv(&config, 999);
        assert_ne!(csv1, csv2);
    }
}
