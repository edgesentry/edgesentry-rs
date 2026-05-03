use crate::entity::{Entity, EntityClass, SensorReading, Vec2};

/// A single time-slice of entity positions.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct EntityFrame {
    pub timestamp_ms: u64,
    pub entities: Vec<Entity>,
}

/// Replay entities from a CSV file.
///
/// CSV columns: `id,class,x,y,vx,vy,timestamp_ms`
pub struct FileReplayAdapter {
    frames: Vec<EntityFrame>,
    cursor: usize,
}

impl FileReplayAdapter {
    /// Load from a CSV file.  Header row is required and must match exactly:
    /// `id,class,x,y,vx,vy,timestamp_ms`
    pub fn from_csv(content: &str) -> Result<Self, String> {
        let mut lines = content.lines();
        let header = lines.next().ok_or("empty file")?;
        if header.trim() != "id,class,x,y,vx,vy,timestamp_ms" {
            return Err(format!("unexpected CSV header: {header}"));
        }

        let mut frames_map: std::collections::BTreeMap<u64, Vec<Entity>> =
            std::collections::BTreeMap::new();

        for (i, line) in lines.enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let cols: Vec<&str> = line.splitn(7, ',').collect();
            if cols.len() != 7 {
                return Err(format!("line {}: expected 7 columns, got {}", i + 2, cols.len()));
            }
            let id = cols[0].to_string();
            let class: EntityClass = serde_json::from_value(serde_json::Value::String(
                cols[1].to_string(),
            ))
            .map_err(|_| format!("line {}: unknown class '{}'", i + 2, cols[1]))?;
            let x: f32 = cols[2].parse().map_err(|_| format!("line {}: bad x", i + 2))?;
            let y: f32 = cols[3].parse().map_err(|_| format!("line {}: bad y", i + 2))?;
            let vx: f32 = cols[4].parse().map_err(|_| format!("line {}: bad vx", i + 2))?;
            let vy: f32 = cols[5].parse().map_err(|_| format!("line {}: bad vy", i + 2))?;
            let ts: u64 = cols[6].parse().map_err(|_| format!("line {}: bad timestamp", i + 2))?;

            frames_map.entry(ts).or_default().push(Entity {
                id,
                class,
                position: Vec2::new(x, y),
                velocity: Vec2::new(vx, vy),
                timestamp_ms: ts,
                sensor: Some(SensorReading::simulation()),
            });
        }

        let frames = frames_map
            .into_iter()
            .map(|(ts, entities)| EntityFrame { timestamp_ms: ts, entities })
            .collect();

        Ok(Self { frames, cursor: 0 })
    }

    /// Return the next frame as a `Vec<Entity>`, or `None` when all frames have been replayed.
    pub fn next_frame(&mut self) -> Option<Vec<Entity>> {
        if self.cursor >= self.frames.len() {
            return None;
        }
        let frame = self.frames[self.cursor].entities.clone();
        self.cursor += 1;
        Some(frame)
    }

    /// Return a slice of all `EntityFrame`s.
    pub fn frames(&self) -> &[EntityFrame] {
        &self.frames
    }

    /// Reset replay to the beginning.
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.cursor = 0;
    }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_CSV: &str = "id,class,x,y,vx,vy,timestamp_ms\n\
FL-01,Forklift,0.0,0.0,1.4,0.0,1000\n\
W-03,Person,3.2,0.0,0.0,0.0,1000\n\
FL-01,Forklift,0.14,0.0,1.4,0.0,2000\n";

    #[test]
    fn loads_two_frames() {
        let adapter = FileReplayAdapter::from_csv(SAMPLE_CSV).unwrap();
        assert_eq!(adapter.frame_count(), 2);
    }

    #[test]
    fn first_frame_has_two_entities() {
        let mut adapter = FileReplayAdapter::from_csv(SAMPLE_CSV).unwrap();
        let frame = adapter.next_frame().unwrap();
        assert_eq!(frame.len(), 2);
    }

    #[test]
    fn entity_fields_parsed_correctly() {
        let mut adapter = FileReplayAdapter::from_csv(SAMPLE_CSV).unwrap();
        let frame = adapter.next_frame().unwrap();
        let fl = frame.iter().find(|e| e.id == "FL-01").unwrap();
        assert_eq!(fl.class, crate::entity::EntityClass::Forklift);
        assert!((fl.velocity.x - 1.4).abs() < 1e-5);
        assert_eq!(fl.timestamp_ms, 1000);
    }

    #[test]
    fn returns_none_after_all_frames() {
        let mut adapter = FileReplayAdapter::from_csv(SAMPLE_CSV).unwrap();
        adapter.next_frame();
        adapter.next_frame();
        assert!(adapter.next_frame().is_none());
    }

    #[test]
    fn reset_replays_from_start() {
        let mut adapter = FileReplayAdapter::from_csv(SAMPLE_CSV).unwrap();
        adapter.next_frame();
        adapter.next_frame();
        adapter.reset();
        assert!(adapter.next_frame().is_some());
    }

    #[test]
    fn bad_header_returns_error() {
        let result = FileReplayAdapter::from_csv("wrong,header\n1,2\n");
        assert!(result.is_err());
    }

    #[test]
    fn unknown_class_returns_error() {
        let csv = "id,class,x,y,vx,vy,timestamp_ms\nX,Submarine,0,0,0,0,0\n";
        assert!(FileReplayAdapter::from_csv(csv).is_err());
    }

    #[test]
    fn frames_accessor_returns_all_frames() {
        let adapter = FileReplayAdapter::from_csv(SAMPLE_CSV).unwrap();
        let frames = adapter.frames();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].timestamp_ms, 1000);
        assert_eq!(frames[1].timestamp_ms, 2000);
    }

    #[test]
    fn entity_frame_is_serializable() {
        let adapter = FileReplayAdapter::from_csv(SAMPLE_CSV).unwrap();
        let frame = &adapter.frames()[0];
        let json = serde_json::to_string(frame).unwrap();
        let decoded: EntityFrame = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.timestamp_ms, frame.timestamp_ms);
        assert_eq!(decoded.entities.len(), frame.entities.len());
    }
}
