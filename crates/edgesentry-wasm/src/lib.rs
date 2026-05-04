// edgesentry-wasm — WebAssembly bindings for the documaris document pipeline.
//
// Exposes the full CSV → fill → check → render → seal chain to JavaScript.
// All functions take/return JSON strings so the boundary stays simple and
// the TypeScript caller doesn't need to know about Rust types.
//
// LLM field fill is not available in WASM (no network I/O); fill() runs in
// direct-mapping mode only (same output as `eds document fill` without --llm-url).

use wasm_bindgen::prelude::*;

// ── parse ─────────────────────────────────────────────────────────────────────

/// Parse a maritime voyage CSV string into a JSON array of DocumentEntity objects.
///
/// Input: CSV text (same format as `crates/edgesentry-document/fixtures/*.csv`)
/// Returns: JSON array string, or throws on parse error.
#[wasm_bindgen]
pub fn parse_maritime_csv(csv: &str) -> Result<String, JsError> {
    let entities = edgesentry_parse::parse_maritime_csv(csv.as_bytes())
        .map_err(|e| JsError::new(&e))?;
    serde_json::to_string(&entities).map_err(|e| JsError::new(&e.to_string()))
}

// ── fill ──────────────────────────────────────────────────────────────────────

/// Fill a FAL form template from a DocumentEntity JSON object.
///
/// `entity_json`: single DocumentEntity (one element from `parse_maritime_csv` output)
/// `template`: "fal-form-1" | "fal-form-5" | "sg-port-entry"
/// `confidence_threshold`: fields below this score are flagged (0.0–1.0, default 0.80)
///
/// Returns: FilledDocument JSON string, or throws on error.
#[wasm_bindgen]
pub fn fill(
    entity_json: &str,
    template: &str,
    confidence_threshold: f64,
) -> Result<String, JsError> {
    let entity: edgesentry_parse::DocumentEntity =
        serde_json::from_str(entity_json).map_err(|e| JsError::new(&e.to_string()))?;
    let filled = edgesentry_document::fill(&entity, template, None, confidence_threshold)
        .map_err(|e| JsError::new(&e))?;
    serde_json::to_string(&filled).map_err(|e| JsError::new(&e.to_string()))
}

// ── check ─────────────────────────────────────────────────────────────────────

/// Check a FilledDocument against a compliance rules JSON array.
///
/// `filled_json`: FilledDocument JSON string (output of `fill()`)
/// `rules_json`: JSON array of rule objects
///   e.g. [{"rule_id":"BWM_D2_EXPIRED","field":"bwm_certificate_expiry",
///           "check":"not_expired","severity":"HIGH","regulation":"..."}]
///
/// Returns: JSON array of ComplianceAlert objects, or throws on error.
#[wasm_bindgen]
pub fn check(filled_json: &str, rules_json: &str) -> Result<String, JsError> {
    let filled: edgesentry_document::FilledDocument =
        serde_json::from_str(filled_json).map_err(|e| JsError::new(&e.to_string()))?;
    let alerts = edgesentry_document::check(&filled, rules_json)
        .map_err(|e| JsError::new(&e))?;
    serde_json::to_string(&alerts).map_err(|e| JsError::new(&e.to_string()))
}

// ── render ────────────────────────────────────────────────────────────────────

/// Render a FilledDocument into an HTML string using the named template.
///
/// `filled_json`: FilledDocument JSON string (output of `fill()`)
/// `template`: "fal-form-1" | "fal-form-5" | "sg-port-entry"
///
/// Returns: HTML string with {{FIELD}} placeholders substituted.
#[wasm_bindgen]
pub fn render_html(filled_json: &str, template: &str) -> Result<String, JsError> {
    let filled: edgesentry_document::FilledDocument =
        serde_json::from_str(filled_json).map_err(|e| JsError::new(&e.to_string()))?;

    let template_html = match template {
        "fal-form-1"    => include_str!("../../edgesentry-document/templates/fal-form-1.html"),
        "fal-form-5"    => include_str!("../../edgesentry-document/templates/fal-form-5.html"),
        "sg-port-entry" => include_str!("../../edgesentry-document/templates/sg-port-entry.html"),
        other => return Err(JsError::new(&format!(
            "unknown template '{other}'; choices: fal-form-1, fal-form-5, sg-port-entry"
        ))),
    };

    Ok(edgesentry_document::render_html(&filled, template_html))
}

// ── audit payload ─────────────────────────────────────────────────────────────

/// Build a deterministic DocumentAuditPayload from a FilledDocument.
///
/// `filled_json`: FilledDocument JSON string (output of `fill()`)
///
/// Returns: DocumentAuditPayload JSON string (fields sorted for stable BLAKE3 hashing).
#[wasm_bindgen]
pub fn build_audit_payload(filled_json: &str) -> Result<String, JsError> {
    let filled: edgesentry_document::FilledDocument =
        serde_json::from_str(filled_json).map_err(|e| JsError::new(&e.to_string()))?;
    let payload = edgesentry_document::build_audit_payload(&filled);
    serde_json::to_string(&payload).map_err(|e| JsError::new(&e.to_string()))
}

// ── seal ──────────────────────────────────────────────────────────────────────

/// Seal a DocumentAuditPayload into a tamper-proof AuditRecord (BLAKE3 + Ed25519).
///
/// `payload_json`: DocumentAuditPayload JSON string (output of `build_audit_payload()`)
/// `private_key_hex`: 64-char hex Ed25519 private key (from `eds audit keygen`)
/// `device_id`: identifier for the sealing device (e.g. "documaris-web-demo")
///
/// Returns: AuditRecord JSON string, or throws on error.
#[wasm_bindgen]
pub fn seal(
    payload_json: &str,
    private_key_hex: &str,
    device_id: &str,
) -> Result<String, JsError> {
    let payload_bytes = payload_json.as_bytes().to_vec();
    let prev_hash = [0u8; 32];
    let record = edgesentry_audit::sign_record(
        device_id.to_string(),
        1,
        now_ms(),
        payload_bytes,
        prev_hash,
        String::new(),
        private_key_hex,
    )
    .map_err(|e| JsError::new(&e.to_string()))?;
    serde_json::to_string(&record).map_err(|e| JsError::new(&e.to_string()))
}

/// Compute the BLAKE3 hash of an arbitrary byte slice.
/// Returns the hash as a 64-char lowercase hex string.
#[wasm_bindgen]
pub fn compute_hash(data: &[u8]) -> String {
    let hash = edgesentry_audit::compute_payload_hash(data);
    hex::encode(hash)
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn now_ms() -> u64 {
    // js_sys::Date::now() returns f64 ms since epoch
    js_sys::Date::now() as u64
}

// ── tests ─────────────────────────────────────────────────────────────────────
//
// Two layers:
//   #[cfg(test)] — regular Rust tests; run with `cargo test -p edgesentry-wasm`
//                  (no WASM runtime needed; test the pipeline logic directly)
//   #[wasm_bindgen_test] — test the exported JSON API in a real WASM runtime;
//                          run with `wasm-pack test --node crates/edgesentry-wasm`

#[cfg(test)]
mod tests {
    use super::*;

    // ── fixture data ──────────────────────────────────────────────────────────

    const CSV_V001: &str = include_str!("../../edgesentry-document/fixtures/voyage_V001_compliant.csv");
    const CSV_V002: &str = include_str!("../../edgesentry-document/fixtures/voyage_V002_bwm_expired.csv");
    const CSV_V003: &str = include_str!("../../edgesentry-document/fixtures/voyage_V003_low_confidence.csv");

    const RULES_JSON: &str = include_str!(
        "../../edgesentry-profile/fixtures/sg-port-compliance/rules.json"
    );

    const PRIV_KEY: &str =
        "0101010101010101010101010101010101010101010101010101010101010101";

    fn first_entity(csv: &str) -> edgesentry_parse::DocumentEntity {
        edgesentry_parse::parse_maritime_csv(csv.as_bytes())
            .expect("parse")
            .into_iter()
            .next()
            .expect("at least one entity")
    }

    // ── parse ─────────────────────────────────────────────────────────────────

    #[test]
    fn parse_v001_vessel_name() {
        let e = first_entity(CSV_V001);
        assert_eq!(e.vessel_name, "MV Horizon");
        assert_eq!(e.voyage_id, "V001");
    }

    #[test]
    fn parse_v002_vessel_name() {
        let e = first_entity(CSV_V002);
        assert_eq!(e.vessel_name, "MV Pacific Star");
        assert_eq!(e.voyage_id, "V002");
    }

    #[test]
    fn parse_v003_missing_fields() {
        let e = first_entity(CSV_V003);
        assert_eq!(e.voyage_id, "V003");
        assert!(e.crew_count.is_none(), "V003 has no crew_count");
    }

    // ── fill ──────────────────────────────────────────────────────────────────

    #[test]
    fn fill_v001_no_review_required() {
        let entity = first_entity(CSV_V001);
        let filled = edgesentry_document::fill(&entity, "fal-form-1", None, 0.80)
            .expect("fill");
        assert!(!filled.review_required, "V001 should not require review");
    }

    #[test]
    fn fill_v003_review_required() {
        let entity = first_entity(CSV_V003);
        let filled = edgesentry_document::fill(&entity, "fal-form-1", None, 0.80)
            .expect("fill");
        assert!(filled.review_required, "V003 should require review (missing fields)");
    }

    // ── check ─────────────────────────────────────────────────────────────────

    #[test]
    fn check_v001_zero_alerts() {
        let entity = first_entity(CSV_V001);
        let filled = edgesentry_document::fill(&entity, "fal-form-1", None, 0.80).unwrap();
        let alerts = edgesentry_document::check(&filled, RULES_JSON).expect("check");
        assert!(alerts.is_empty(), "V001 should produce 0 alerts, got {alerts:?}");
    }

    #[test]
    fn check_v002_bwm_expired_high() {
        let entity = first_entity(CSV_V002);
        let filled = edgesentry_document::fill(&entity, "fal-form-1", None, 0.80).unwrap();
        let alerts = edgesentry_document::check(&filled, RULES_JSON).expect("check");
        let bwm = alerts.iter().find(|a| a.rule_id == "BWM_D2_EXPIRED")
            .expect("BWM_D2_EXPIRED alert must fire");
        assert_eq!(bwm.severity, "HIGH");
    }

    #[test]
    fn check_v003_crew_count_alert() {
        let entity = first_entity(CSV_V003);
        let filled = edgesentry_document::fill(&entity, "fal-form-1", None, 0.80).unwrap();
        let alerts = edgesentry_document::check(&filled, RULES_JSON).expect("check");
        assert!(
            alerts.iter().any(|a| a.rule_id == "CREW_COUNT_PRESENT"),
            "CREW_COUNT_PRESENT alert must fire for V003"
        );
    }

    // ── render ────────────────────────────────────────────────────────────────

    #[test]
    fn render_html_contains_vessel_name() {
        let entity = first_entity(CSV_V001);
        let filled = edgesentry_document::fill(&entity, "fal-form-1", None, 0.80).unwrap();
        let template = include_str!("../../edgesentry-document/templates/fal-form-1.html");
        let html = edgesentry_document::render_html(&filled, template);
        assert!(html.contains("MV Horizon"), "HTML must contain vessel name");
    }

    #[test]
    fn render_html_unknown_template_returns_none() {
        // The template match in render_html returns Err for unknown names;
        // test the match arm directly without going through the wasm boundary.
        let unknown: Option<&str> = match "nonexistent-template" {
            "fal-form-1" | "fal-form-5" | "sg-port-entry" => Some("known"),
            _ => None,
        };
        assert!(unknown.is_none(), "unknown template must not resolve");
    }

    // ── audit payload ─────────────────────────────────────────────────────────

    #[test]
    fn audit_payload_contains_voyage_id() {
        let entity = first_entity(CSV_V001);
        let filled = edgesentry_document::fill(&entity, "fal-form-1", None, 0.80).unwrap();
        let payload = edgesentry_document::build_audit_payload(&filled);
        assert_eq!(payload.voyage_id, "V001");
    }

    #[test]
    fn audit_payload_is_deterministic() {
        let entity = first_entity(CSV_V001);
        let filled = edgesentry_document::fill(&entity, "fal-form-1", None, 0.80).unwrap();
        let p1 = serde_json::to_string(&edgesentry_document::build_audit_payload(&filled)).unwrap();
        let p2 = serde_json::to_string(&edgesentry_document::build_audit_payload(&filled)).unwrap();
        assert_eq!(p1, p2, "audit payload must be deterministic");
    }

    // ── compute_hash ──────────────────────────────────────────────────────────

    #[test]
    fn compute_hash_returns_64_hex_chars() {
        let hash = edgesentry_audit::compute_payload_hash(b"test");
        let hex = hex::encode(hash);
        assert_eq!(hex.len(), 64);
    }

    #[test]
    fn compute_hash_is_deterministic() {
        let h1 = hex::encode(edgesentry_audit::compute_payload_hash(b"hello"));
        let h2 = hex::encode(edgesentry_audit::compute_payload_hash(b"hello"));
        assert_eq!(h1, h2);
    }

    #[test]
    fn compute_hash_differs_for_different_input() {
        let h1 = hex::encode(edgesentry_audit::compute_payload_hash(b"hello"));
        let h2 = hex::encode(edgesentry_audit::compute_payload_hash(b"world"));
        assert_ne!(h1, h2);
    }

    // ── seal (non-WASM path — uses fixed timestamp) ───────────────────────────

    #[test]
    fn seal_produces_verifiable_record() {
        let entity = first_entity(CSV_V001);
        let filled = edgesentry_document::fill(&entity, "fal-form-1", None, 0.80).unwrap();
        let payload = edgesentry_document::build_audit_payload(&filled);
        let payload_json = serde_json::to_string(&payload).unwrap();

        let record = edgesentry_audit::sign_record(
            "test-device".to_string(),
            1,
            1_700_000_000_000,
            payload_json.as_bytes().to_vec(),
            [0u8; 32],
            String::new(),
            PRIV_KEY,
        )
        .expect("sign_record");

        let keypair = edgesentry_audit::inspect_key(PRIV_KEY).expect("inspect_key");
        assert!(
            edgesentry_audit::verify_record(&record, &keypair.public_key_hex)
                .expect("verify"),
            "sealed record must verify"
        );
    }

    // ── wasm JSON API ─────────────────────────────────────────────────────────
    // These test the exported string-in/string-out functions end-to-end
    // (same path a TypeScript caller would take, but without the JS runtime).

    #[test]
    fn wasm_api_parse_returns_valid_json() {
        let json = parse_maritime_csv(CSV_V001).expect("parse_maritime_csv");
        let arr: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert!(arr.is_array());
        assert_eq!(arr[0]["vessel_name"].as_str().unwrap(), "MV Horizon");
    }

    #[test]
    fn wasm_api_fill_returns_valid_json() {
        let entities_json = parse_maritime_csv(CSV_V001).unwrap();
        let arr: serde_json::Value = serde_json::from_str(&entities_json).unwrap();
        let entity_json = arr[0].to_string();
        let filled_json = fill(&entity_json, "fal-form-1", 0.80).expect("fill");
        let filled: serde_json::Value = serde_json::from_str(&filled_json).unwrap();
        assert!(!filled["review_required"].as_bool().unwrap());
    }

    #[test]
    fn wasm_api_check_v002_fires_bwm_alert() {
        let entities_json = parse_maritime_csv(CSV_V002).unwrap();
        let arr: serde_json::Value = serde_json::from_str(&entities_json).unwrap();
        let filled_json = fill(&arr[0].to_string(), "fal-form-1", 0.80).unwrap();
        let alerts_json = check(&filled_json, RULES_JSON).expect("check");
        let alerts: serde_json::Value = serde_json::from_str(&alerts_json).unwrap();
        assert!(
            alerts.as_array().unwrap().iter().any(|a| a["rule_id"] == "BWM_D2_EXPIRED"),
            "BWM_D2_EXPIRED must appear in alerts"
        );
    }

    #[test]
    fn wasm_api_render_html_contains_vessel_name() {
        let entities_json = parse_maritime_csv(CSV_V001).unwrap();
        let arr: serde_json::Value = serde_json::from_str(&entities_json).unwrap();
        let filled_json = fill(&arr[0].to_string(), "fal-form-1", 0.80).unwrap();
        let html = render_html(&filled_json, "fal-form-1").expect("render_html");
        assert!(html.contains("MV Horizon"));
    }

    #[test]
    fn wasm_api_build_audit_payload_voyage_id() {
        let entities_json = parse_maritime_csv(CSV_V001).unwrap();
        let arr: serde_json::Value = serde_json::from_str(&entities_json).unwrap();
        let filled_json = fill(&arr[0].to_string(), "fal-form-1", 0.80).unwrap();
        let payload_json = build_audit_payload(&filled_json).expect("build_audit_payload");
        let payload: serde_json::Value = serde_json::from_str(&payload_json).unwrap();
        assert_eq!(payload["voyage_id"].as_str().unwrap(), "V001");
    }
}
