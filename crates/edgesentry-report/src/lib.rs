use std::collections::HashMap;

use edgesentry_assess::{Assessment, EntityCorrelation, RiskTrend};
use edgesentry_evaluate::{RiskEvent, Severity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportConfig {
    pub site_name: Option<String>,
    pub report_period: Option<String>,
    pub chain_valid: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSummary {
    pub total: usize,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleFrequencyRow {
    pub rule_id: String,
    pub count: usize,
    pub severity_str: String,
    pub regulation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityCorrelationRow {
    pub entity_ids: Vec<String>,
    pub event_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub site_name: Option<String>,
    pub report_period: Option<String>,
    pub generated_at_ms: u64,
    pub event_summary: EventSummary,
    pub rule_frequencies: Vec<RuleFrequencyRow>,
    pub entity_correlations: Vec<EntityCorrelationRow>,
    pub trend: RiskTrend,
    pub chain_valid: Option<bool>,
}

fn severity_str(sev: &Severity) -> String {
    let v = serde_json::to_value(sev).unwrap_or(serde_json::Value::String("UNKNOWN".to_string()));
    match v {
        serde_json::Value::String(s) => s,
        _ => "UNKNOWN".to_string(),
    }
}

pub fn generate_report(events: &[RiskEvent], assessment: &Assessment, config: ReportConfig) -> Report {
    let total = events.len();
    let mut critical = 0usize;
    let mut high = 0usize;
    let mut medium = 0usize;
    let mut low = 0usize;

    for e in events {
        match e.severity {
            Severity::Critical => critical += 1,
            Severity::High => high += 1,
            Severity::Medium => medium += 1,
            Severity::Low => low += 1,
        }
    }

    let mut rule_map: HashMap<String, (usize, String, String)> = HashMap::new();
    for e in events {
        let entry = rule_map.entry(e.rule_id.clone()).or_insert_with(|| {
            (0, severity_str(&e.severity), e.regulation.clone())
        });
        entry.0 += 1;
    }
    let mut rule_frequencies: Vec<RuleFrequencyRow> = rule_map
        .into_iter()
        .map(|(rule_id, (count, severity_str, regulation))| RuleFrequencyRow {
            rule_id,
            count,
            severity_str,
            regulation,
        })
        .collect();
    rule_frequencies.sort_by_key(|r| std::cmp::Reverse(r.count));

    let entity_correlations: Vec<EntityCorrelationRow> = assessment
        .correlated_entities
        .iter()
        .map(|ec: &EntityCorrelation| EntityCorrelationRow {
            entity_ids: ec.entity_ids.clone(),
            event_count: ec.event_count,
        })
        .collect();

    let generated_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    Report {
        site_name: config.site_name,
        report_period: config.report_period,
        generated_at_ms,
        event_summary: EventSummary { total, critical, high, medium, low },
        rule_frequencies,
        entity_correlations,
        trend: assessment.trend.clone(),
        chain_valid: config.chain_valid,
    }
}

pub fn render_markdown(report: &Report) -> String {
    let mut out = String::new();

    out.push_str("# EdgeSentry Safety Report\n\n");

    if let Some(ref site) = report.site_name {
        out.push_str(&format!("**Site:** {}\n\n", site));
    }
    if let Some(ref period) = report.report_period {
        out.push_str(&format!("**Period:** {}\n\n", period));
    }
    out.push_str(&format!("**Generated:** {} (UTC unix ms)\n\n", report.generated_at_ms));

    out.push_str("## Summary\n\n");
    out.push_str("| Severity | Count |\n");
    out.push_str("|----------|-------|\n");
    out.push_str(&format!("| Critical | {} |\n", report.event_summary.critical));
    out.push_str(&format!("| High     | {} |\n", report.event_summary.high));
    out.push_str(&format!("| Medium   | {} |\n", report.event_summary.medium));
    out.push_str(&format!("| Low      | {} |\n", report.event_summary.low));
    out.push_str(&format!("| **Total**| **{}** |\n\n", report.event_summary.total));

    out.push_str("## Risk Events by Rule\n\n");
    out.push_str("| Rule | Count | Severity | Regulation |\n");
    out.push_str("|------|-------|----------|------------|\n");
    for row in &report.rule_frequencies {
        out.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            row.rule_id, row.count, row.severity_str, row.regulation
        ));
    }
    out.push('\n');

    if !report.entity_correlations.is_empty() {
        out.push_str("## Entity Correlations\n\n");
        out.push_str("| Entities | Event Count |\n");
        out.push_str("|----------|-------------|\n");
        for row in &report.entity_correlations {
            out.push_str(&format!(
                "| {} | {} |\n",
                row.entity_ids.join(", "),
                row.event_count
            ));
        }
        out.push('\n');
    }

    out.push_str("## Trend Analysis\n\n");
    let (trend_label, trend_note) = match report.trend {
        RiskTrend::Stable => ("Stable", "Event rate is consistent — no escalation detected."),
        RiskTrend::Rising => ("Rising", "Event rate is increasing — escalating risk requires attention."),
        RiskTrend::Falling => ("Falling", "Event rate is decreasing — situation is improving."),
    };
    out.push_str(&format!("Risk trend: **{}**\n\n{}\n\n", trend_label, trend_note));

    if let Some(valid) = report.chain_valid {
        out.push_str("## Audit Chain\n\n");
        if valid {
            out.push_str("Chain integrity: **PASS**\n\n");
        } else {
            out.push_str("Chain integrity: **FAIL**\n\n");
        }
    }

    out
}

pub fn validate(events: &[RiskEvent], assessment: &Assessment) -> Result<(), String> {
    if events.is_empty() {
        return Err("no events provided".to_string());
    }
    if assessment.event_count == 0 {
        return Err("assessment has zero event count".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgesentry_assess::{assess, RiskTrend};
    use edgesentry_evaluate::Severity;

    fn make_event(rule_id: &str, severity: Severity) -> RiskEvent {
        RiskEvent {
            rule_id: rule_id.to_string(),
            severity,
            regulation: "Test Regulation".to_string(),
            entity_ids: vec!["A".to_string()],
            measured_value: 1.0,
            threshold: 5.0,
            timestamp_ms: 1000,
        }
    }

    #[test]
    fn empty_events_returns_err_from_validate() {
        let assessment = assess(&[], None);
        let result = validate(&[], &assessment);
        assert!(result.is_err());
    }

    #[test]
    fn non_empty_events_produces_report() {
        let events = vec![
            make_event("RULE_A", Severity::High),
            make_event("RULE_A", Severity::High),
            make_event("RULE_B", Severity::Critical),
        ];
        let assessment = assess(&events, None);
        let config = ReportConfig {
            site_name: Some("Test Site".to_string()),
            report_period: Some("2026-Q1".to_string()),
            chain_valid: Some(true),
        };
        let report = generate_report(&events, &assessment, config);
        assert_eq!(report.event_summary.total, 3);
        assert_eq!(report.event_summary.high, 2);
        assert_eq!(report.event_summary.critical, 1);
        assert_eq!(report.rule_frequencies.len(), 2);
        assert_eq!(report.rule_frequencies[0].rule_id, "RULE_A");
        assert_eq!(report.rule_frequencies[0].count, 2);
        assert_eq!(report.site_name, Some("Test Site".to_string()));
        assert_eq!(report.trend, RiskTrend::Stable);
    }

    #[test]
    fn render_markdown_on_minimal_report_compiles() {
        let events = vec![make_event("RULE_X", Severity::Low)];
        let assessment = assess(&events, None);
        let config = ReportConfig { site_name: None, report_period: None, chain_valid: None };
        let report = generate_report(&events, &assessment, config);
        let md = render_markdown(&report);
        assert!(md.contains("# EdgeSentry Safety Report"));
        assert!(md.contains("## Summary"));
        assert!(md.contains("## Risk Events by Rule"));
        assert!(md.contains("## Trend Analysis"));
        assert!(!md.contains("## Audit Chain"));
        assert!(!md.contains("## Entity Correlations"));
    }

    #[test]
    fn render_markdown_shows_chain_valid_when_some() {
        let events = vec![make_event("RULE_X", Severity::Low)];
        let assessment = assess(&events, None);
        let config = ReportConfig { site_name: None, report_period: None, chain_valid: Some(false) };
        let report = generate_report(&events, &assessment, config);
        let md = render_markdown(&report);
        assert!(md.contains("## Audit Chain"));
        assert!(md.contains("**FAIL**"));
    }
}
