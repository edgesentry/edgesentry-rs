use std::collections::HashMap;

use edgesentry_parse::DocumentEntity;
use serde::{Deserialize, Serialize};

/// Payload sealed into the audit chain for a generated compliance document.
///
/// Constructed from a [`FilledDocument`] immediately before signing.
/// Contains no raw sensor data or document content — only structured metadata
/// sufficient to prove what was generated and with what confidence.
///
/// `timestamp_ms` is intentionally absent: it is a property of the signing event
/// stored in [`AuditRecord::timestamp_ms`], not of the document content. This
/// ensures the same payload hash can be recomputed from the same [`FilledDocument`]
/// at any point in time for verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentAuditPayload {
    pub record_type: String,
    pub voyage_id: String,
    pub template_id: String,
    /// AI-generated or direct value for each field, sorted by key for determinism.
    pub ai_field_values: Vec<(String, String)>,
    /// Confidence score (0.0–1.0) per field, sorted by key for determinism.
    pub confidence_flags: Vec<(String, f64)>,
    /// Fields below the confidence threshold that required human review.
    pub fields_flagged: Vec<String>,
    pub review_required: bool,
}

/// Build a [`DocumentAuditPayload`] from a [`FilledDocument`].
///
/// All map fields are sorted by key to ensure deterministic serialisation —
/// the same [`FilledDocument`] always produces the same BLAKE3 hash.
pub fn build_audit_payload(doc: &FilledDocument) -> DocumentAuditPayload {
    let mut ai_field_values: Vec<(String, String)> = doc
        .fields
        .iter()
        .map(|(k, v)| (k.clone(), v.value.clone()))
        .collect();
    ai_field_values.sort_by(|a, b| a.0.cmp(&b.0));

    let mut confidence_flags: Vec<(String, f64)> = doc
        .fields
        .iter()
        .map(|(k, v)| (k.clone(), v.confidence))
        .collect();
    confidence_flags.sort_by(|a, b| a.0.cmp(&b.0));

    let mut fields_flagged: Vec<String> = doc
        .fields
        .iter()
        .filter(|(_, v)| v.flagged)
        .map(|(k, _)| k.clone())
        .collect();
    fields_flagged.sort();

    DocumentAuditPayload {
        record_type: "document".to_string(),
        voyage_id: doc.voyage_id.clone(),
        template_id: doc.template.clone(),
        ai_field_values,
        confidence_flags,
        fields_flagged,
        review_required: doc.review_required,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldSource {
    Direct,
    Llm,
    Derived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldValue {
    pub value: String,
    pub confidence: f64,
    pub source: FieldSource,
    pub flagged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilledDocument {
    pub voyage_id: String,
    pub template: String,
    pub fields: HashMap<String, FieldValue>,
    pub review_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceAlert {
    pub rule_id: String,
    pub severity: String,
    pub field: String,
    pub message: String,
    pub regulation: String,
    pub voyage_id: String,
}

fn make_field(value: Option<String>, threshold: f64) -> FieldValue {
    match value {
        Some(v) => FieldValue {
            value: v,
            confidence: 0.95,
            source: FieldSource::Direct,
            flagged: false,
        },
        None => FieldValue {
            value: String::new(),
            confidence: 0.0,
            source: FieldSource::Direct,
            flagged: 0.0 < threshold,
        },
    }
}

pub fn fill(
    entity: &DocumentEntity,
    template: &str,
    llm_url: Option<&str>,
    confidence_threshold: f64,
) -> Result<FilledDocument, String> {
    let _ = llm_url;

    let mut fields: HashMap<String, FieldValue> = HashMap::new();

    fields.insert("VESSEL_NAME".to_string(), FieldValue {
        value: entity.vessel_name.clone(),
        confidence: 0.95,
        source: FieldSource::Direct,
        flagged: false,
    });
    fields.insert("VESSEL_IMO".to_string(), make_field(entity.vessel_imo.clone(), confidence_threshold));
    fields.insert("FLAG_STATE".to_string(), make_field(entity.flag_state.clone(), confidence_threshold));
    fields.insert("PORT_OF_ARRIVAL".to_string(), make_field(entity.port_of_arrival.clone(), confidence_threshold));
    fields.insert("ARRIVAL_DATE".to_string(), make_field(entity.arrival_date.clone(), confidence_threshold));
    fields.insert("CARGO_DESCRIPTION".to_string(), make_field(entity.cargo_description.clone(), confidence_threshold));
    fields.insert("CARGO_HS_CODE".to_string(), make_field(entity.cargo_hs_code.clone(), confidence_threshold));
    fields.insert(
        "CREW_COUNT".to_string(),
        make_field(entity.crew_count.map(|c| c.to_string()), confidence_threshold),
    );
    fields.insert(
        "GROSS_TONNAGE".to_string(),
        make_field(entity.gross_tonnage.map(|g| g.to_string()), confidence_threshold),
    );
    fields.insert("BWM_CERTIFICATE_EXPIRY".to_string(), make_field(entity.bwm_certificate_expiry.clone(), confidence_threshold));
    fields.insert(
        "DANGEROUS_GOODS".to_string(),
        make_field(entity.dangerous_goods.map(|b| b.to_string()), confidence_threshold),
    );
    fields.insert("QUARANTINE_STATUS".to_string(), make_field(entity.quarantine_status.clone(), confidence_threshold));

    let review_required = fields.values().any(|f| f.flagged);

    Ok(FilledDocument {
        voyage_id: entity.voyage_id.clone(),
        template: template.to_string(),
        fields,
        review_required,
    })
}

#[derive(Debug, Deserialize)]
struct RuleSpec {
    rule_id: String,
    field: String,
    check: String,
    severity: String,
    regulation: String,
}

const DEMO_TODAY: &str = "2026-06-15";

fn date_is_expired(date_str: &str) -> bool {
    date_str.trim() < DEMO_TODAY
}

pub fn check(doc: &FilledDocument, rules_json: &str) -> Result<Vec<ComplianceAlert>, String> {
    let rules: Vec<RuleSpec> =
        serde_json::from_str(rules_json).map_err(|e| format!("rules JSON parse error: {e}"))?;

    let mut alerts = Vec::new();

    for rule in &rules {
        let field_key = rule.field.to_uppercase().replace('-', "_");
        let field_val = doc.fields.get(&field_key);

        let fires = match rule.check.as_str() {
            "not_expired" => {
                match field_val {
                    None => true,
                    Some(fv) if fv.value.is_empty() || fv.flagged => true,
                    Some(fv) => date_is_expired(&fv.value),
                }
            }
            "not_null" => {
                match field_val {
                    None => true,
                    Some(fv) => fv.value.is_empty() || fv.flagged,
                }
            }
            "not_true" => {
                match field_val {
                    None => false,
                    Some(fv) => fv.value.trim().to_lowercase() == "true",
                }
            }
            other => {
                return Err(format!("unknown check type: '{other}'"));
            }
        };

        if fires {
            alerts.push(ComplianceAlert {
                rule_id: rule.rule_id.clone(),
                severity: rule.severity.clone(),
                field: rule.field.clone(),
                message: format!(
                    "Rule '{}' failed check '{}' on field '{}'",
                    rule.rule_id, rule.check, rule.field
                ),
                regulation: rule.regulation.clone(),
                voyage_id: doc.voyage_id.clone(),
            });
        }
    }

    Ok(alerts)
}

pub fn render_html(doc: &FilledDocument, template_html: &str) -> String {
    let mut out = template_html.to_string();
    for (key, fv) in &doc.fields {
        let placeholder = format!("{{{{{}}}}}", key);
        out = out.replace(&placeholder, &fv.value);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgesentry_parse::DocumentEntity;

    fn make_entity(voyage_id: &str, bwm_expiry: Option<&str>, crew: Option<u32>) -> DocumentEntity {
        DocumentEntity {
            voyage_id: voyage_id.to_string(),
            vessel_name: "MV Test".to_string(),
            vessel_imo: Some("IMO1234567".to_string()),
            flag_state: Some("SGP".to_string()),
            port_of_arrival: Some("SGSIN".to_string()),
            arrival_date: Some("2026-06-15".to_string()),
            cargo_description: Some("Test cargo".to_string()),
            cargo_hs_code: Some("8428".to_string()),
            crew_count: crew,
            gross_tonnage: Some(30000.0),
            bwm_certificate_expiry: bwm_expiry.map(|s| s.to_string()),
            dangerous_goods: Some(false),
            quarantine_status: Some("CLEAR".to_string()),
            crew_nationalities: None,
        }
    }

    const RULES_JSON: &str = r#"[
      {"rule_id":"BWM_D2_EXPIRED","field":"bwm_certificate_expiry","check":"not_expired","severity":"HIGH","regulation":"BWM Convention"},
      {"rule_id":"CREW_DOC_VALIDITY","field":"crew_count","check":"not_null","severity":"MEDIUM","regulation":"MLC 2006"}
    ]"#;

    #[test]
    fn fill_compliant_entity_no_flags() {
        let entity = make_entity("V001", Some("2027-03-01"), Some(23));
        let doc = fill(&entity, "fal-form-1", None, 0.5).unwrap();
        assert!(!doc.review_required);
        assert_eq!(doc.voyage_id, "V001");
    }

    #[test]
    fn fill_missing_crew_flags_review() {
        let entity = make_entity("V003", Some("2027-03-01"), None);
        let doc = fill(&entity, "fal-form-1", None, 0.5).unwrap();
        assert!(doc.review_required);
        let crew_field = doc.fields.get("CREW_COUNT").unwrap();
        assert!(crew_field.flagged);
        assert!((crew_field.confidence - 0.0).abs() < 1e-9);
    }

    #[test]
    fn check_expired_bwm_fires_alert() {
        let entity = make_entity("V002", Some("2026-04-30"), Some(18));
        let doc = fill(&entity, "fal-form-1", None, 0.5).unwrap();
        let alerts = check(&doc, RULES_JSON).unwrap();
        assert!(alerts.iter().any(|a| a.rule_id == "BWM_D2_EXPIRED"));
    }

    #[test]
    fn check_valid_bwm_no_alert() {
        let entity = make_entity("V001", Some("2027-03-01"), Some(23));
        let doc = fill(&entity, "fal-form-1", None, 0.5).unwrap();
        let alerts = check(&doc, RULES_JSON).unwrap();
        assert!(!alerts.iter().any(|a| a.rule_id == "BWM_D2_EXPIRED"));
    }

    #[test]
    fn render_html_replaces_placeholders() {
        let entity = make_entity("V001", Some("2027-03-01"), Some(23));
        let doc = fill(&entity, "fal-form-1", None, 0.5).unwrap();
        let tmpl = "<p>{{VESSEL_NAME}}</p><p>{{CREW_COUNT}}</p>";
        let rendered = render_html(&doc, tmpl);
        assert!(rendered.contains("MV Test"));
        assert!(rendered.contains("23"));
        assert!(!rendered.contains("{{VESSEL_NAME}}"));
    }

    #[test]
    fn build_audit_payload_voyage_and_template() {
        let entity = make_entity("V001", Some("2027-03-01"), Some(23));
        let doc = fill(&entity, "fal-form-1", None, 0.5).unwrap();
        let payload = build_audit_payload(&doc);
        assert_eq!(payload.record_type, "document");
        assert_eq!(payload.voyage_id, "V001");
        assert_eq!(payload.template_id, "fal-form-1");
        assert!(!payload.review_required);
        assert!(payload.fields_flagged.is_empty());
    }

    #[test]
    fn build_audit_payload_fields_sorted_for_determinism() {
        let entity = make_entity("V001", Some("2027-03-01"), Some(23));
        let doc = fill(&entity, "fal-form-1", None, 0.5).unwrap();
        let p1 = build_audit_payload(&doc);
        let p2 = build_audit_payload(&doc);
        // Same input must produce identical serialisation every call.
        let b1 = serde_json::to_vec(&p1).unwrap();
        let b2 = serde_json::to_vec(&p2).unwrap();
        assert_eq!(b1, b2, "payload serialisation must be deterministic");
        // Keys must be sorted.
        let keys: Vec<&str> = p1.ai_field_values.iter().map(|(k, _)| k.as_str()).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted, "ai_field_values must be sorted by key");
    }

    #[test]
    fn build_audit_payload_flags_low_confidence_fields() {
        let entity = make_entity("V003", Some("2027-03-01"), None); // crew_count missing
        let doc = fill(&entity, "fal-form-1", None, 0.5).unwrap();
        let payload = build_audit_payload(&doc);
        assert!(payload.review_required);
        assert!(payload.fields_flagged.contains(&"CREW_COUNT".to_string()));
        // Confidence for flagged field is 0.0
        let crew_conf = payload.confidence_flags.iter()
            .find(|(k, _)| k == "CREW_COUNT")
            .map(|(_, v)| *v);
        assert_eq!(crew_conf, Some(0.0));
    }

    #[test]
    fn build_audit_payload_is_json_serialisable() {
        let entity = make_entity("V001", Some("2027-03-01"), Some(23));
        let doc = fill(&entity, "fal-form-1", None, 0.5).unwrap();
        let payload = build_audit_payload(&doc);
        let json = serde_json::to_string(&payload).unwrap();
        let round_tripped: DocumentAuditPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(round_tripped.voyage_id, payload.voyage_id);
        assert_eq!(round_tripped.template_id, payload.template_id);
    }
}
