use std::collections::HashMap;
use std::path::Path;

use edgesentry_ingest::csv_replay::EntityFrame;
use edgesentry_types::{Entity, EntityClass, Vec2};
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

/// Read a Parquet file produced by maridb and return `DocumentEntity` records.
///
/// Expected schema (column names match the CSV header exactly):
/// voyage_id, vessel_name, vessel_imo, flag_state, port_of_arrival,
/// arrival_date, cargo_description, cargo_hs_code, crew_count,
/// gross_tonnage, bwm_certificate_expiry, dangerous_goods, quarantine_status
pub fn parse_maritime_parquet(path: &Path) -> Result<Vec<DocumentEntity>, String> {
    use parquet::file::reader::{FileReader, SerializedFileReader};
    use parquet::record::Field;
    use std::fs::File;

    let file = File::open(path).map_err(|e| format!("cannot open '{}': {e}", path.display()))?;
    let reader = SerializedFileReader::new(file)
        .map_err(|e| format!("parquet open error: {e}"))?;

    let mut entities = Vec::new();

    for (i, row_result) in reader
        .get_row_iter(None)
        .map_err(|e| format!("row iter error: {e}"))?
        .enumerate()
    {
        let row = row_result.map_err(|e| format!("row {i}: {e}"))?;

        let mut fields: HashMap<String, &Field> = HashMap::new();
        for (name, field) in row.get_column_iter() {
            fields.insert(name.to_string(), field);
        }

        let get_str = |key: &str| -> Option<String> {
            match *fields.get(key)? {
                Field::Str(ref s) => if s.is_empty() { None } else { Some(s.clone()) },
                _ => None,
            }
        };
        let get_u32 = |key: &str| -> Option<u32> {
            match *fields.get(key)? {
                Field::Long(ref n) => Some(*n as u32),
                Field::Int(ref n)  => Some(*n as u32),
                _ => None,
            }
        };
        let get_f64 = |key: &str| -> Option<f64> {
            match *fields.get(key)? {
                Field::Double(ref f) => Some(*f),
                Field::Float(ref f)  => Some(*f as f64),
                Field::Long(ref n)   => Some(*n as f64),
                Field::Int(ref n)    => Some(*n as f64),
                _ => None,
            }
        };
        let get_bool = |key: &str| -> Option<bool> {
            match *fields.get(key)? {
                Field::Bool(ref b) => Some(*b),
                _ => None,
            }
        };

        let voyage_id   = get_str("voyage_id")
            .ok_or_else(|| format!("row {i}: voyage_id is null or empty"))?;
        let vessel_name = get_str("vessel_name")
            .ok_or_else(|| format!("row {i}: vessel_name is null or empty"))?;

        entities.push(DocumentEntity {
            voyage_id,
            vessel_name,
            vessel_imo:             get_str("vessel_imo"),
            flag_state:             get_str("flag_state"),
            port_of_arrival:        get_str("port_of_arrival"),
            arrival_date:           get_str("arrival_date"),
            cargo_description:      get_str("cargo_description"),
            cargo_hs_code:          get_str("cargo_hs_code"),
            crew_count:             get_u32("crew_count"),
            gross_tonnage:          get_f64("gross_tonnage"),
            bwm_certificate_expiry: get_str("bwm_certificate_expiry"),
            dangerous_goods:        get_bool("dangerous_goods"),
            quarantine_status:      get_str("quarantine_status"),
            crew_nationalities:     None,
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

    // ── Parquet round-trip ────────────────────────────────────────────────────

    /// Write SAMPLE_CSV rows as a Parquet file, then read them back via
    /// `parse_maritime_parquet` and assert the result matches the CSV parse.
    #[test]
    fn parquet_round_trip_matches_csv() {
        use parquet::column::writer::ColumnWriter;
        use parquet::file::properties::WriterProperties;
        use parquet::file::writer::SerializedFileWriter;
        use parquet::schema::parser::parse_message_type;
        use std::sync::Arc;

        let tmp = std::env::temp_dir().join("voyage_test.parquet");

        let schema_str = "
            message schema {
                REQUIRED BYTE_ARRAY voyage_id (UTF8);
                REQUIRED BYTE_ARRAY vessel_name (UTF8);
                OPTIONAL BYTE_ARRAY vessel_imo (UTF8);
                OPTIONAL BYTE_ARRAY flag_state (UTF8);
                OPTIONAL BYTE_ARRAY port_of_arrival (UTF8);
                OPTIONAL BYTE_ARRAY arrival_date (UTF8);
                OPTIONAL BYTE_ARRAY cargo_description (UTF8);
                OPTIONAL BYTE_ARRAY cargo_hs_code (UTF8);
                OPTIONAL INT64 crew_count;
                OPTIONAL DOUBLE gross_tonnage;
                OPTIONAL BYTE_ARRAY bwm_certificate_expiry (UTF8);
                OPTIONAL BOOLEAN dangerous_goods;
                OPTIONAL BYTE_ARRAY quarantine_status (UTF8);
            }
        ";
        let schema = Arc::new(parse_message_type(schema_str).unwrap());
        let props = Arc::new(WriterProperties::builder().build());
        let file = std::fs::File::create(&tmp).unwrap();
        let mut writer = SerializedFileWriter::new(file, schema, props).unwrap();

        let s = |v: &str| -> parquet::data_type::ByteArray { v.into() };
        let def2 = [1i16, 1];

        let mut rg = writer.next_row_group().unwrap();

        // Helper: write a BYTE_ARRAY (UTF8) column
        fn write_str(rg: &mut parquet::file::writer::SerializedRowGroupWriter<std::fs::File>,
                     vals: &[parquet::data_type::ByteArray], def: &[i16]) {
            let mut cw = rg.next_column().unwrap().unwrap();
            if let ColumnWriter::ByteArrayColumnWriter(ref mut w) = cw.untyped() {
                w.write_batch(vals, Some(def), None).unwrap();
            }
            cw.close().unwrap();
        }

        let v2 = |a: &str, b: &str| vec![s(a), s(b)];
        let v1 = |a: &str| vec![s(a)];

        write_str(&mut rg, &v2("V001","V002"),                    &def2);
        write_str(&mut rg, &v2("MV Horizon","MV Star"),           &def2);
        write_str(&mut rg, &v1("IMO9876543"),                     &[1i16,0]);
        write_str(&mut rg, &v2("SGP","MYS"),                      &def2);
        write_str(&mut rg, &v2("SGSIN","SGSIN"),                  &def2);
        write_str(&mut rg, &v2("2026-06-15","2026-06-18"),        &def2);
        write_str(&mut rg, &v2("General machinery","Steel coils"),&def2);
        write_str(&mut rg, &v2("8428","7208"),                    &def2);
        {
            let mut cw = rg.next_column().unwrap().unwrap();
            if let ColumnWriter::Int64ColumnWriter(ref mut w) = cw.untyped() {
                w.write_batch(&[23i64], Some(&[1i16,0]), None).unwrap();
            }
            cw.close().unwrap();
        }
        {
            let mut cw = rg.next_column().unwrap().unwrap();
            if let ColumnWriter::DoubleColumnWriter(ref mut w) = cw.untyped() {
                w.write_batch(&[45000.0f64, 32000.0], Some(&def2), None).unwrap();
            }
            cw.close().unwrap();
        }
        write_str(&mut rg, &v2("2027-03-01","2026-04-30"),        &def2);
        {
            let mut cw = rg.next_column().unwrap().unwrap();
            if let ColumnWriter::BoolColumnWriter(ref mut w) = cw.untyped() {
                w.write_batch(&[false, true], Some(&def2), None).unwrap();
            }
            cw.close().unwrap();
        }
        write_str(&mut rg, &v2("CLEAR","QUARANTINE"),             &def2);

        rg.close().unwrap();
        writer.close().unwrap();

        let parquet_result = parse_maritime_parquet(&tmp).unwrap();
        let csv_result = parse_maritime_csv(SAMPLE_CSV.as_bytes()).unwrap();

        assert_eq!(parquet_result.len(), csv_result.len());
        assert_eq!(parquet_result[0].voyage_id,   csv_result[0].voyage_id);
        assert_eq!(parquet_result[0].vessel_name, csv_result[0].vessel_name);
        assert_eq!(parquet_result[0].vessel_imo,  csv_result[0].vessel_imo);
        assert_eq!(parquet_result[0].crew_count,  csv_result[0].crew_count);
        assert_eq!(parquet_result[1].voyage_id,   csv_result[1].voyage_id);
        assert_eq!(parquet_result[1].vessel_imo,  None);
        assert_eq!(parquet_result[1].crew_count,  None);

        let _ = std::fs::remove_file(&tmp);
    }
}
