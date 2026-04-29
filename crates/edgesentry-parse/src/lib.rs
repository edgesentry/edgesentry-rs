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
}
