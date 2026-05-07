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

pub fn fill_bca(
    entity: &edgesentry_parse::BcaOutletEntity,
    confidence_threshold: f64,
) -> Result<FilledDocument, String> {
    let mut fields: HashMap<String, FieldValue> = HashMap::new();

    fields.insert("OUTLET_ID".to_string(), FieldValue {
        value: entity.outlet_id.clone(),
        confidence: 0.95,
        source: FieldSource::Direct,
        flagged: false,
    });
    fields.insert("BUILDING_NAME".to_string(), FieldValue {
        value: entity.building_name.clone(),
        confidence: 0.95,
        source: FieldSource::Direct,
        flagged: false,
    });
    fields.insert("BUILDING_TYPE".to_string(), make_field(entity.building_type.clone(), confidence_threshold));
    fields.insert("PERIOD_START".to_string(), make_field(entity.period_start.clone(), confidence_threshold));
    fields.insert("PERIOD_END".to_string(), make_field(entity.period_end.clone(), confidence_threshold));
    fields.insert(
        "GROSS_FLOOR_AREA_M2".to_string(),
        make_field(entity.gross_floor_area_m2.map(|v| v.to_string()), confidence_threshold),
    );
    fields.insert(
        "EUI_KWH_M2".to_string(),
        make_field(entity.eui_kwh_m2.map(|v| v.to_string()), confidence_threshold),
    );
    fields.insert(
        "CHILLER_COP".to_string(),
        make_field(entity.chiller_cop.map(|v| v.to_string()), confidence_threshold),
    );
    fields.insert(
        "LPD_W_M2".to_string(),
        make_field(entity.lpd_w_m2.map(|v| v.to_string()), confidence_threshold),
    );
    fields.insert(
        "WATER_L_M2".to_string(),
        make_field(entity.water_l_m2.map(|v| v.to_string()), confidence_threshold),
    );
    fields.insert("GREEN_MARK_TARGET".to_string(), make_field(entity.green_mark_target.clone(), confidence_threshold));
    fields.insert("CERTIFYING_BODY".to_string(), make_field(entity.certifying_body.clone(), confidence_threshold));

    let review_required = fields.values().any(|f| f.flagged);

    Ok(FilledDocument {
        voyage_id: entity.outlet_id.clone(),
        template: "sg-bca-greenmark".to_string(),
        fields,
        review_required,
    })
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
    threshold: Option<f64>,
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
            // Fires when the numeric field value is strictly above the threshold.
            // Missing or empty fields do not fire (use not_null for that separately).
            "above_threshold" => {
                match (rule.threshold, field_val) {
                    (Some(threshold), Some(fv)) if !fv.value.is_empty() => {
                        fv.value.trim().parse::<f64>().map(|v| v > threshold).unwrap_or(false)
                    }
                    _ => false,
                }
            }
            other => {
                return Err(format!("unknown check type: '{other}'"));
            }
        };

        if fires {
            let field_val_str = field_val.map(|fv| fv.value.as_str()).unwrap_or("");
            let message = match rule.check.as_str() {
                "above_threshold" => {
                    if let Some(threshold) = rule.threshold {
                        format!(
                            "{} {} exceeds target of ≤ {}",
                            rule.field, field_val_str, threshold
                        )
                    } else {
                        format!("{} exceeds threshold", rule.field)
                    }
                }
                "not_expired" => format!("{} has expired ({})", rule.field, field_val_str),
                "not_null" => format!("{} is missing or empty", rule.field),
                "not_true" => format!("{} must not be set to true", rule.field),
                _ => format!("Rule '{}' failed on field '{}'", rule.rule_id, rule.field),
            };
            alerts.push(ComplianceAlert {
                rule_id: rule.rule_id.clone(),
                severity: rule.severity.clone(),
                field: rule.field.clone(),
                message,
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

    // ── BCA fill tests ────────────────────────────────────────────────────────

    fn make_bca_entity(outlet_id: &str, eui: Option<f64>) -> edgesentry_parse::BcaOutletEntity {
        edgesentry_parse::BcaOutletEntity {
            outlet_id: outlet_id.to_string(),
            building_name: "Test Building".to_string(),
            building_type: Some("Retail".to_string()),
            period_start: Some("2025-01-01".to_string()),
            period_end: Some("2025-12-31".to_string()),
            gross_floor_area_m2: Some(3200.0),
            eui_kwh_m2: eui,
            chiller_cop: Some(0.61),
            lpd_w_m2: Some(13.2),
            water_l_m2: Some(380.0),
            green_mark_target: Some("Platinum".to_string()),
            certifying_body: Some("BCA".to_string()),
        }
    }

    #[test]
    fn fill_bca_compliant_outlet_no_review_required() {
        let entity = make_bca_entity("B001", Some(108.5));
        let doc = fill_bca(&entity, 0.80).unwrap();
        assert!(!doc.review_required, "B001 should not require review");
        assert_eq!(doc.template, "sg-bca-greenmark");
        assert_eq!(doc.voyage_id, "B001");
    }

    #[test]
    fn fill_bca_missing_eui_flags_review() {
        let entity = make_bca_entity("B002", None);
        let doc = fill_bca(&entity, 0.80).unwrap();
        assert!(doc.review_required, "B002 should require review (EUI missing)");
        let eui_field = doc.fields.get("EUI_KWH_M2").unwrap();
        assert!(eui_field.flagged);
        assert!((eui_field.confidence - 0.0).abs() < 1e-9);
    }

    #[test]
    fn build_audit_payload_bca_template_id() {
        let entity = make_bca_entity("B001", Some(108.5));
        let doc = fill_bca(&entity, 0.80).unwrap();
        let payload = build_audit_payload(&doc);
        assert_eq!(payload.template_id, "sg-bca-greenmark");
        assert_eq!(payload.voyage_id, "B001");
        assert!(!payload.review_required);
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

    // ── above_threshold check type ────────────────────────────────────────────

    fn make_bca_doc_with_eui(eui: &str) -> FilledDocument {
        let mut fields = std::collections::HashMap::new();
        fields.insert("EUI_KWH_M2".to_string(), FieldValue {
            value: eui.to_string(),
            confidence: 0.95,
            source: FieldSource::Direct,
            flagged: false,
        });
        FilledDocument {
            voyage_id: "MCH-OUTLET-001".to_string(),
            template: "sg-bca-greenmark".to_string(),
            fields,
            review_required: false,
        }
    }

    const BCA_THRESHOLD_RULE: &str = r#"[
      {"rule_id":"EUI_PLATINUM_EXCEEDED","field":"eui_kwh_m2","check":"above_threshold","threshold":115.0,"severity":"HIGH","regulation":"BCA Green Mark 2021"}
    ]"#;

    #[test]
    fn above_threshold_fires_when_value_exceeds() {
        let doc = make_bca_doc_with_eui("122.5");
        let alerts = check(&doc, BCA_THRESHOLD_RULE).unwrap();
        assert!(alerts.iter().any(|a| a.rule_id == "EUI_PLATINUM_EXCEEDED"),
            "should fire when EUI 122.5 > 115");
    }

    #[test]
    fn above_threshold_does_not_fire_when_value_at_or_below() {
        let doc = make_bca_doc_with_eui("114.9");
        let alerts = check(&doc, BCA_THRESHOLD_RULE).unwrap();
        assert!(!alerts.iter().any(|a| a.rule_id == "EUI_PLATINUM_EXCEEDED"),
            "should not fire when EUI 114.9 ≤ 115");
    }

    #[test]
    fn above_threshold_does_not_fire_on_missing_field() {
        let doc = make_bca_doc_with_eui("");
        let alerts = check(&doc, BCA_THRESHOLD_RULE).unwrap();
        assert!(!alerts.iter().any(|a| a.rule_id == "EUI_PLATINUM_EXCEEDED"),
            "missing value should not fire above_threshold");
    }
}
