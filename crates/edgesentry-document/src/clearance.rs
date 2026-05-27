//! Port Cyber Clearance certificate — fill from indago `*_facts.json` (Cap Vista W5).

use std::collections::HashMap;

use serde::Deserialize;

use crate::{FieldSource, FieldValue, FilledDocument};

/// indago `write_evaluation_artifacts` facts payload.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ClearanceFacts {
    pub vessel_key: String,
    pub port_call_id: String,
    pub outcome: String,
    pub decision_hash: String,
    #[serde(default)]
    pub rules_fired: Vec<ClearanceRuleHit>,
    #[serde(default)]
    pub paths: Vec<ClearancePath>,
    #[serde(default)]
    pub cve_ids: Vec<String>,
    pub disclaimer: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ClearanceRuleHit {
    pub id: String,
    pub title: String,
    pub severity: String,
    #[serde(default)]
    pub requirements: Vec<String>,
    #[serde(default)]
    pub evidence: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ClearancePath {
    #[serde(default)]
    pub rule_ids: Vec<String>,
    #[serde(default)]
    pub nodes: Vec<String>,
    pub summary: String,
}

pub const TEMPLATE_ID: &str = "port-cyber-clearance";

pub const TEMPLATE_HTML: &str = include_str!("../templates/port-cyber-clearance.html");

fn direct(value: impl Into<String>) -> FieldValue {
    FieldValue {
        value: value.into(),
        confidence: 1.0,
        source: FieldSource::Direct,
        flagged: false,
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn rules_table_rows(rules: &[ClearanceRuleHit]) -> String {
    if rules.is_empty() {
        return "<tr><td colspan=\"4\" class=\"muted\">No rules fired — clearance pass</td></tr>".to_string();
    }
    rules
        .iter()
        .map(|r| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                escape_html(&r.id),
                escape_html(&r.title),
                escape_html(&r.severity),
                escape_html(&r.requirements.join(", ")),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn paths_table_rows(paths: &[ClearancePath]) -> String {
    if paths.is_empty() {
        return "<tr><td colspan=\"3\" class=\"muted\">No cited vulnerability paths</td></tr>".to_string();
    }
    paths
        .iter()
        .map(|p| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                escape_html(&p.rule_ids.join(", ")),
                escape_html(&p.summary),
                escape_html(&p.nodes.join(" → ")),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn summary_paragraph(facts: &ClearanceFacts) -> String {
    let outcome_upper = facts.outcome.to_uppercase();
    if facts.outcome == "hold" {
        format!(
            "Vessel <strong>{}</strong> on port call <strong>{}</strong> received clearance outcome \
             <strong>{outcome_upper}</strong>. {} rule(s) fired across {} cited path(s). \
             Berth recommendation: review cyber exposure before port entry.",
            escape_html(&facts.vessel_key),
            escape_html(&facts.port_call_id),
            facts.rules_fired.len(),
            facts.paths.len(),
        )
    } else {
        format!(
            "Vessel <strong>{}</strong> on port call <strong>{}</strong> received clearance outcome \
             <strong>{outcome_upper}</strong>. No blocking rules fired on the pinned CVE snapshot and SBOM fixtures.",
            escape_html(&facts.vessel_key),
            escape_html(&facts.port_call_id),
        )
    }
}

/// Map indago facts + verify URL into a `FilledDocument` for `port-cyber-clearance.html`.
pub fn fill_clearance(facts: &ClearanceFacts, verify_url: &str) -> FilledDocument {
    let outcome_upper = facts.outcome.to_uppercase();
    let outcome_class = if facts.outcome == "hold" {
        "outcome-hold"
    } else {
        "outcome-pass"
    };
    let cve_line = if facts.cve_ids.is_empty() {
        "None identified on evaluation paths".to_string()
    } else {
        facts.cve_ids.join(", ")
    };
    let generated_date = "2026-05-27";

    let mut fields: HashMap<String, FieldValue> = HashMap::new();
    fields.insert("VESSEL_KEY".to_string(), direct(&facts.vessel_key));
    fields.insert("PORT_CALL_ID".to_string(), direct(&facts.port_call_id));
    fields.insert("OUTCOME".to_string(), direct(outcome_upper));
    fields.insert("OUTCOME_CLASS".to_string(), direct(outcome_class));
    fields.insert("DECISION_HASH".to_string(), direct(&facts.decision_hash));
    fields.insert("GENERATED_DATE".to_string(), direct(generated_date));
    fields.insert("VERIFY_URL".to_string(), direct(verify_url));
    fields.insert("CVE_IDS".to_string(), direct(cve_line));
    fields.insert("DISCLAIMER".to_string(), direct(&facts.disclaimer));
    fields.insert("SUMMARY_HTML".to_string(), direct(summary_paragraph(facts)));
    fields.insert("RULES_TABLE_ROWS".to_string(), direct(rules_table_rows(&facts.rules_fired)));
    fields.insert("PATHS_TABLE_ROWS".to_string(), direct(paths_table_rows(&facts.paths)));
    fields.insert(
        "RULES_COUNT".to_string(),
        direct(facts.rules_fired.len().to_string()),
    );

    let voyage_id = format!("{}_{}", facts.vessel_key, facts.port_call_id);

    FilledDocument {
        voyage_id,
        template: TEMPLATE_ID.to_string(),
        fields,
        review_required: facts.outcome == "hold",
    }
}

pub fn parse_clearance_facts_json(content: &str) -> Result<ClearanceFacts, String> {
    serde_json::from_str(content).map_err(|e| format!("facts JSON parse error: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render_html;

    const HOLD_FACTS: &str = include_str!("../fixtures/clearance/vessel-hold_facts.json");
    const CLEAN_FACTS: &str = include_str!("../fixtures/clearance/vessel-clean_facts.json");

    #[test]
    fn fill_clearance_hold_marks_review_required() {
        let facts = parse_clearance_facts_json(HOLD_FACTS).unwrap();
        let doc = fill_clearance(&facts, "https://verify.example/clearance/demo");
        assert!(doc.review_required);
        assert_eq!(doc.template, TEMPLATE_ID);
        assert_eq!(doc.fields.get("OUTCOME").unwrap().value, "HOLD");
    }

    #[test]
    fn fill_clearance_pass_no_review() {
        let facts = parse_clearance_facts_json(CLEAN_FACTS).unwrap();
        let doc = fill_clearance(&facts, "https://verify.example/clearance/demo");
        assert!(!doc.review_required);
        assert_eq!(doc.fields.get("OUTCOME").unwrap().value, "PASS");
    }

    #[test]
    fn render_clearance_html_contains_outcome_and_verify() {
        let facts = parse_clearance_facts_json(HOLD_FACTS).unwrap();
        let doc = fill_clearance(&facts, "https://verify.example/clearance/abc123");
        let html = render_html(&doc, TEMPLATE_HTML);
        assert!(html.contains("HOLD"));
        assert!(html.contains("vessel-hold"));
        assert!(html.contains("https://verify.example/clearance/abc123"));
        assert!(html.contains("SG-CC-001"));
        assert!(!html.contains("{{OUTCOME}}"));
    }
}
