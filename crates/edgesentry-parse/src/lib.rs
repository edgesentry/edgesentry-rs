use std::collections::HashMap;

use edgesentry_ingest::csv_replay::EntityFrame;
use edgesentry_ingest::entity::{Entity, EntityClass, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentEntity {
    pub voyage_id: String,
    pub vessel_name: String,
    pub vessel_imo: Option<String>,
    pub flag_state: Option<String>,
    pub port_of_arrival: Option<String>,
    pub arrival_date: Option<String>,
    pub cargo_description: Option<String>,
    pub cargo_hs_code: Option<String>,
    pub crew_count: Option<u32>,
    pub gross_tonnage: Option<f64>,
    pub bwm_certificate_expiry: Option<String>,
    pub dangerous_goods: Option<bool>,
    pub quarantine_status: Option<String>,
    pub crew_nationalities: Option<Vec<String>>,
}

fn opt_str(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() { None } else { Some(t.to_string()) }
}

fn opt_u32(s: &str) -> Option<u32> {
    let t = s.trim();
    if t.is_empty() { return None; }
    t.parse().ok()
}

fn opt_f64(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() { return None; }
    t.parse().ok()
}

fn opt_bool(s: &str) -> Option<bool> {
    let t = s.trim().to_lowercase();
    if t.is_empty() { return None; }
    match t.as_str() {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        _ => None,
    }
}

pub fn parse_maritime_csv(reader: impl std::io::Read) -> Result<Vec<DocumentEntity>, String> {
    let mut bytes = Vec::new();
    let mut r = reader;
    std::io::Read::read_to_end(&mut r, &mut bytes).map_err(|e| format!("read error: {e}"))?;
    let content = String::from_utf8(bytes).map_err(|e| format!("UTF-8 error: {e}"))?;

    let mut lines = content.lines();
    let header = lines.next().ok_or_else(|| "CSV has no header".to_string())?;
    let _ = header;

    let mut entities = Vec::new();
    for (lineno, line) in lines.enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.splitn(13, ',').collect();
        if fields.len() < 13 {
            return Err(format!("line {}: expected 13 fields, got {}", lineno + 2, fields.len()));
        }

        let voyage_id = fields[0].trim().to_string();
        let vessel_name = fields[1].trim().to_string();
        if voyage_id.is_empty() {
            return Err(format!("line {}: voyage_id is required", lineno + 2));
        }
        if vessel_name.is_empty() {
            return Err(format!("line {}: vessel_name is required", lineno + 2));
        }

        entities.push(DocumentEntity {
            voyage_id,
            vessel_name,
            vessel_imo: opt_str(fields[2]),
            flag_state: opt_str(fields[3]),
            port_of_arrival: opt_str(fields[4]),
            arrival_date: opt_str(fields[5]),
            cargo_description: opt_str(fields[6]),
            cargo_hs_code: opt_str(fields[7]),
            crew_count: opt_u32(fields[8]),
            gross_tonnage: opt_f64(fields[9]),
            bwm_certificate_expiry: opt_str(fields[10]),
            dangerous_goods: opt_bool(fields[11]),
            quarantine_status: opt_str(fields[12]),
            crew_nationalities: None,
        });
    }

    Ok(entities)
}

/// A structured document — key-value pairs extracted from any source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDocument {
    pub source: String,
    pub fields: HashMap<String, serde_json::Value>,
}

/// Parse a JSON document file into a ParsedDocument.
///
/// Reads a JSON object from reader, stores all fields in `fields`.
/// `source` is set to "json".
pub fn parse_document_json(reader: impl std::io::Read) -> Result<ParsedDocument, String> {
    let mut bytes = Vec::new();
    let mut r = reader;
    std::io::Read::read_to_end(&mut r, &mut bytes).map_err(|e| format!("read error: {e}"))?;
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| format!("JSON parse error: {e}"))?;
    let obj = value
        .as_object()
        .ok_or_else(|| "expected a JSON object at top level".to_string())?;
    let fields: HashMap<String, serde_json::Value> = obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    Ok(ParsedDocument { source: "json".to_string(), fields })
}

/// Map a string entity type name to an `EntityClass`.
fn entity_class_from_str(s: &str) -> EntityClass {
    match s.to_lowercase().as_str() {
        "forklift" => EntityClass::Forklift,
        "reachstacker" | "reach_stacker" => EntityClass::ReachStacker,
        "terminaltractor" | "terminal_tractor" => EntityClass::TerminalTractor,
        "vessel" | "ship" => EntityClass::Vessel,
        _ => EntityClass::Person,
    }
}

/// Convert a ParsedDocument to EntityFrames for use with `eds evaluate`.
///
/// Looks for fields:
/// - `"entities"` (array of `{id, type, x, y, vx, vy, timestamp_ms}`)
/// - or individual entity fields `entity_id`, `x`, `y`, etc. for a single entity.
///
/// If neither is present, returns an empty `Vec`.
pub fn document_to_entity_frames(doc: &ParsedDocument) -> Vec<EntityFrame> {
    if let Some(arr) = doc.fields.get("entities").and_then(|v| v.as_array()) {
        // Group by timestamp_ms.
        let mut ts_map: std::collections::BTreeMap<u64, Vec<Entity>> = std::collections::BTreeMap::new();
        for item in arr {
            let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
            let entity_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("unknown");
            let x = item.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let y = item.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let vx = item.get("vx").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let vy = item.get("vy").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let timestamp_ms = item.get("timestamp_ms").and_then(|v| v.as_u64()).unwrap_or(0);
            let class = entity_class_from_str(entity_type);
            let entity = Entity {
                id,
                class,
                position: Vec2::new(x, y),
                velocity: Vec2::new(vx, vy),
                timestamp_ms,
            };
            ts_map.entry(timestamp_ms).or_default().push(entity);
        }
        return ts_map
            .into_iter()
            .map(|(ts, entities)| EntityFrame { timestamp_ms: ts, entities })
            .collect();
    }

    // Fall back to single-entity fields.
    if let Some(entity_id) = doc.fields.get("entity_id").and_then(|v| v.as_str()) {
        let entity_type = doc.fields.get("entity_type").and_then(|v| v.as_str()).unwrap_or("unknown");
        let x = doc.fields.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let y = doc.fields.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let vx = doc.fields.get("vx").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let vy = doc.fields.get("vy").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let timestamp_ms = doc.fields.get("timestamp_ms").and_then(|v| v.as_u64()).unwrap_or(0);
        let class = entity_class_from_str(entity_type);
        let entity = Entity {
            id: entity_id.to_string(),
            class,
            position: Vec2::new(x, y),
            velocity: Vec2::new(vx, vy),
            timestamp_ms,
        };
        let frame = EntityFrame { timestamp_ms, entities: vec![entity] };
        return vec![frame];
    }

    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_CSV: &str = "\
voyage_id,vessel_name,vessel_imo,flag_state,port_of_arrival,arrival_date,cargo_description,cargo_hs_code,crew_count,gross_tonnage,bwm_certificate_expiry,dangerous_goods,quarantine_status
V001,MV Horizon,IMO9876543,SGP,SGSIN,2026-06-15,General machinery,8428,23,45000,2027-03-01,false,CLEAR
V002,MV Star,,MYS,SGSIN,2026-06-18,Steel coils,7208,,32000,2026-04-30,true,QUARANTINE";

    #[test]
    fn parses_two_rows() {
        let result = parse_maritime_csv(SAMPLE_CSV.as_bytes()).unwrap();
        assert_eq!(result.len(), 2);

        let v001 = &result[0];
        assert_eq!(v001.voyage_id, "V001");
        assert_eq!(v001.vessel_name, "MV Horizon");
        assert_eq!(v001.vessel_imo, Some("IMO9876543".to_string()));
        assert_eq!(v001.flag_state, Some("SGP".to_string()));
        assert_eq!(v001.crew_count, Some(23));
        assert!((v001.gross_tonnage.unwrap() - 45000.0).abs() < 1e-5);
        assert_eq!(v001.dangerous_goods, Some(false));
        assert_eq!(v001.quarantine_status, Some("CLEAR".to_string()));

        let v002 = &result[1];
        assert_eq!(v002.voyage_id, "V002");
        assert_eq!(v002.vessel_imo, None);
        assert_eq!(v002.crew_count, None);
        assert_eq!(v002.dangerous_goods, Some(true));
    }

    const SAMPLE_DOCUMENT_JSON: &str = r#"{
  "site": "Demo Warehouse A",
  "recorded_at": "2026-04-30T09:00:00Z",
  "entities": [
    {"id": "FL-01", "type": "forklift", "x": 10.0, "y": 8.0, "vx": -1.0, "vy": 0.0, "timestamp_ms": 0},
    {"id": "W-03",  "type": "pedestrian", "x": 5.0, "y": 8.0, "vx": 0.0, "vy": 0.0, "timestamp_ms": 0}
  ]
}"#;

    #[test]
    fn parse_document_json_parses_sample() {
        let doc = parse_document_json(SAMPLE_DOCUMENT_JSON.as_bytes()).unwrap();
        assert_eq!(doc.source, "json");
        assert!(doc.fields.contains_key("site"));
        assert!(doc.fields.contains_key("entities"));
    }

    #[test]
    fn document_to_entity_frames_correct_count() {
        let doc = parse_document_json(SAMPLE_DOCUMENT_JSON.as_bytes()).unwrap();
        let frames = document_to_entity_frames(&doc);
        // Both entities share timestamp_ms = 0, so one frame with 2 entities.
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].entities.len(), 2);
    }

    #[test]
    fn document_to_entity_frames_entity_ids() {
        let doc = parse_document_json(SAMPLE_DOCUMENT_JSON.as_bytes()).unwrap();
        let frames = document_to_entity_frames(&doc);
        let ids: Vec<&str> = frames[0].entities.iter().map(|e| e.id.as_str()).collect();
        assert!(ids.contains(&"FL-01"));
        assert!(ids.contains(&"W-03"));
    }

    #[test]
    fn document_to_entity_frames_empty_when_no_entities() {
        let json = r#"{"site": "X"}"#;
        let doc = parse_document_json(json.as_bytes()).unwrap();
        let frames = document_to_entity_frames(&doc);
        assert!(frames.is_empty());
    }
}
