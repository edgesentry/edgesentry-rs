use std::collections::HashMap;
use std::io::BufWriter;

use edgesentry_assess::{Assessment, EntityCorrelation, RiskTrend};
use edgesentry_evaluate::{EvidenceQuality, RiskEvent, Severity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExplanationEntry {
    pub rule_id: String,
    pub timestamp_ms: u64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportConfig {
    pub site_name: Option<String>,
    pub report_period: Option<String>,
    pub chain_valid: Option<bool>,
    pub executive_summary: Option<String>,
    #[serde(default)]
    pub explanations: Vec<ExplanationEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvidenceQualitySummary {
    pub certified: usize,
    pub degraded: usize,
    pub rejected: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSummary {
    pub total: usize,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub evidence_quality: EvidenceQualitySummary,
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
    pub executive_summary: Option<String>,
    #[serde(default)]
    pub explanations: Vec<ExplanationEntry>,
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
    let mut eq = EvidenceQualitySummary::default();

    for e in events {
        match e.severity {
            Severity::Critical => critical += 1,
            Severity::High => high += 1,
            Severity::Medium => medium += 1,
            Severity::Low => low += 1,
        }
        match e.evidence_quality {
            EvidenceQuality::Certified => eq.certified += 1,
            EvidenceQuality::Degraded  => eq.degraded  += 1,
            EvidenceQuality::Rejected  => eq.rejected  += 1,
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
        event_summary: EventSummary { total, critical, high, medium, low, evidence_quality: eq },
        rule_frequencies,
        entity_correlations,
        trend: assessment.trend.clone(),
        chain_valid: config.chain_valid,
        executive_summary: config.executive_summary,
        explanations: config.explanations,
    }
}

/// Format a Unix millisecond timestamp as "DD Mon YYYY HH:MM UTC" without
/// depending on an external date crate.
fn fmt_timestamp_ms(ms: u64) -> String {
    let secs = ms / 1000;
    // Days since Unix epoch
    let days      = secs / 86400;
    let rem_secs  = secs % 86400;
    let hours     = rem_secs / 3600;
    let minutes   = (rem_secs % 3600) / 60;

    // Gregorian calendar calculation (valid for 2001–2099)
    let mut y = 1970u64;
    let mut d = days;
    loop {
        let days_in_year = if y.is_multiple_of(4) { 366 } else { 365 };
        if d < days_in_year { break; }
        d -= days_in_year;
        y += 1;
    }
    let leap = y.is_multiple_of(4);
    let month_days = [31u64, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let month_names = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"];
    let mut m = 0usize;
    let mut day = d;
    for (i, &md) in month_days.iter().enumerate() {
        if day < md { m = i; break; }
        day -= md;
    }
    format!("{:02} {} {} {:02}:{:02} UTC", day + 1, month_names[m], y, hours, minutes)
}

pub fn render_markdown(report: &Report) -> String {
    let mut out = String::new();

    out.push_str("# EdgeSentry Safety Report\n\n");
    out.push_str(
        "_This report is generated by the EdgeSentry physics-based safety monitoring system. \
        Each risk event reflects a deterministic rule violation — a measured physical value \
        (distance, time-to-collision, or zone membership) that breached a threshold \
        encoded from the applicable safety regulation. Events are cryptographically sealed \
        and cannot be altered after generation._\n\n"
    );

    if let Some(ref site) = report.site_name {
        out.push_str(&format!("**Site:** {}\n\n", site));
    }
    if let Some(ref period) = report.report_period {
        out.push_str(&format!("**Period:** {}\n\n", period));
    }
    out.push_str(&format!("**Generated:** {}\n\n", fmt_timestamp_ms(report.generated_at_ms)));

    if let Some(ref summary) = report.executive_summary {
        out.push_str("## Executive Summary\n\n");
        out.push_str(summary);
        out.push_str("\n\n");
    }

    out.push_str("## Summary\n\n");
    out.push_str(
        "Total number of safety rule violations detected during this period, grouped by severity. \
        Each event represents a moment when a monitored entity crossed a regulatory threshold.\n\n"
    );
    out.push_str("| Severity | Count | Meaning |\n");
    out.push_str("|----------|-------|--------|\n");
    out.push_str(&format!("| Critical | {} | Immediate danger — breach requires immediate stop-work or intervention |\n", report.event_summary.critical));
    out.push_str(&format!("| High     | {} | Significant breach — corrective action required |\n", report.event_summary.high));
    out.push_str(&format!("| Medium   | {} | Moderate breach — monitor and review |\n", report.event_summary.medium));
    out.push_str(&format!("| Low      | {} | Minor breach — logged for trend analysis |\n", report.event_summary.low));
    out.push_str(&format!("| **Total**| **{}** | |\n\n", report.event_summary.total));

    let eq = &report.event_summary.evidence_quality;
    if report.event_summary.total > 0 {
        out.push_str("## Evidence Quality\n\n");
        out.push_str(
            "Each event carries a machine-verifiable quality score derived from the CV model \
            confidence. Certified events carry full actuarial weight; Degraded events carry \
            reduced weight; Rejected events are recorded but not admissible as evidence.\n\n"
        );
        out.push_str("| Quality | Count | Actuarial weight |\n");
        out.push_str("|---------|-------|------------------|\n");
        out.push_str(&format!("| Certified  | {} | Full |\n", eq.certified));
        out.push_str(&format!("| Degraded   | {} | Reduced |\n", eq.degraded));
        out.push_str(&format!("| Rejected   | {} | None (recorded only) |\n\n", eq.rejected));
    }

    out.push_str("## Risk Events by Rule\n\n");
    out.push_str(
        "Each row shows how many times a specific safety rule was violated. \
        The regulation column cites the exact clause that defines the threshold — \
        the same clause an MOM inspector or underwriter would reference.\n\n"
    );
    out.push_str("| Rule | Events | Severity | Regulation |\n");
    out.push_str("|------|--------|----------|------------|\n");
    for row in &report.rule_frequencies {
        out.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            row.rule_id, row.count, row.severity_str, row.regulation
        ));
    }
    out.push('\n');

    out.push_str("## Trend Analysis\n\n");
    let (trend_label, trend_note) = match report.trend {
        RiskTrend::Stable  => ("Stable",  "The rate of rule violations is consistent across the period — no escalation detected. Stable does not mean safe; it means the risk level is not worsening."),
        RiskTrend::Rising  => ("Rising",  "The rate of rule violations is increasing over the period. This indicates escalating risk and requires management attention."),
        RiskTrend::Falling => ("Falling", "The rate of rule violations is decreasing over the period. This indicates improving safety conditions."),
    };
    out.push_str(&format!("**{}** — {}\n\n", trend_label, trend_note));

    if !report.entity_correlations.is_empty() {
        out.push_str("## Entity Involvement\n\n");
        out.push_str(
            "The entities below were involved in the most rule violations during this period. \
            High involvement by a specific equipment-person pair may indicate a systemic \
            workflow issue — for example, a forklift route that repeatedly conflicts with \
            a pedestrian path.\n\n"
        );
        out.push_str("| Entity / Pair | Events Involved In | Interpretation |\n");
        out.push_str("|---------------|-------------------|----------------|\n");
        for row in &report.entity_correlations {
            let entities = row.entity_ids.join(" + ");
            let interpretation = if row.entity_ids.len() > 1 {
                format!("These entities were in proximity during {} events — possible conflicting routes", row.event_count)
            } else {
                format!("This entity was involved in {} events", row.event_count)
            };
            out.push_str(&format!("| {} | {} | {} |\n", entities, row.event_count, interpretation));
        }
        out.push('\n');
    }

    if !report.explanations.is_empty() {
        out.push_str("## Event Explanations\n\n");
        out.push_str(
            "_Each explanation below was generated by a local LLM (llama.cpp) at the time of the event. \
            The explanation interprets the physics measurement in plain language against the cited regulation._\n\n"
        );
        for (i, entry) in report.explanations.iter().enumerate() {
            out.push_str(&format!(
                "### {i}. {} — {}\n\n{}\n\n",
                entry.rule_id,
                fmt_timestamp_ms(entry.timestamp_ms),
                entry.text
            ));
        }
    }

    if let Some(valid) = report.chain_valid {
        out.push_str("## Audit Chain Integrity\n\n");
        if valid {
            out.push_str(
                "**PASS** — All event records in the audit chain have been verified. \
                Each record's BLAKE3 hash matches its content and is linked to the previous \
                record via `prev_record_hash`. No tampering, deletion, or reordering has occurred. \
                This chain can be presented as independent third-party evidence in an MOM \
                inspection or insurance claim.\n\n"
            );
        } else {
            out.push_str(
                "**FAIL** — The audit chain did not pass integrity verification. \
                One or more records may have been modified, deleted, or reordered. \
                This chain cannot be used as tamper-proof evidence.\n\n"
            );
        }
    }

    out
}

/// Render `report` as a minimal A4 PDF and return the raw bytes.
pub fn render_pdf(report: &Report) -> Vec<u8> {
    use printpdf::{BuiltinFont, Mm, PdfDocument};

    let (doc, page1, layer1) =
        PdfDocument::new("EdgeSentry Safety Report", Mm(210.0_f32), Mm(297.0_f32), "Layer 1");
    let current_layer = doc.get_page(page1).get_layer(layer1);

    let font = doc.add_builtin_font(BuiltinFont::Helvetica).unwrap();
    let font_bold = doc.add_builtin_font(BuiltinFont::HelveticaBold).unwrap();

    // Cursor starts near top of page; y decreases as we add lines.
    let mut y = 277.0_f32;
    let left = 15.0_f32;

    // Helper: write one text line (font, size, x, y in mm).
    macro_rules! line {
        ($fnt:expr, $size:expr, $x:expr, $y:expr, $text:expr) => {
            current_layer.use_text($text, $size as f32, Mm($x), Mm($y), &$fnt);
        };
    }

    // Title
    line!(font_bold, 18.0_f32, left, y, "EdgeSentry Safety Report");
    y -= 10.0;

    // Metadata
    let site   = report.site_name.clone().unwrap_or_else(|| "-".to_string());
    let period = report.report_period.clone().unwrap_or_else(|| "-".to_string());
    line!(font, 10.0_f32, left, y, format!("Site: {site}"));
    y -= 6.0;
    line!(font, 10.0_f32, left, y, format!("Period: {period}"));
    y -= 6.0;
    line!(font, 10.0_f32, left, y, format!("Generated: {}", fmt_timestamp_ms(report.generated_at_ms)));
    y -= 6.0;
    line!(font, 9.0_f32, left, y,
        "All events are tamper-evident and cannot be altered after recording.");
    y -= 10.0;

    // Executive Summary
    if let Some(ref summary) = report.executive_summary {
        line!(font_bold, 13.0_f32, left, y, "Executive Summary");
        y -= 7.0;
        let words: Vec<&str> = summary.split_whitespace().collect();
        let mut line_buf = String::new();
        for word in &words {
            if line_buf.len() + word.len() + 1 > 95 {
                line!(font, 9.5_f32, left, y, line_buf.as_str());
                y -= 5.5;
                line_buf = word.to_string();
                if y < 25.0 { break; }
            } else {
                if !line_buf.is_empty() { line_buf.push(' '); }
                line_buf.push_str(word);
            }
        }
        if !line_buf.is_empty() && y >= 25.0 {
            line!(font, 9.5_f32, left, y, line_buf.as_str());
            y -= 5.5;
        }
        y -= 6.0;
    }

    // Summary
    line!(font_bold, 14.0_f32, left, y, "Summary of Rule Violations");
    y -= 8.0;
    line!(font, 9.0_f32, left, y,
        "Number of times a monitored entity crossed a regulatory safety threshold:");
    y -= 7.0;
    line!(font, 10.0_f32, left, y,
        format!("Critical (immediate stop-work required):  {}", report.event_summary.critical));
    y -= 6.0;
    line!(font, 10.0_f32, left, y,
        format!("High     (corrective action required):     {}", report.event_summary.high));
    y -= 6.0;
    line!(font, 10.0_f32, left, y,
        format!("Medium   (monitor and review):             {}", report.event_summary.medium));
    y -= 6.0;
    line!(font, 10.0_f32, left, y,
        format!("Low      (logged for trend analysis):      {}", report.event_summary.low));
    y -= 6.0;
    line!(font_bold, 10.0_f32, left, y,
        format!("Total:                                     {}", report.event_summary.total));
    y -= 10.0;

    // Evidence Quality
    if report.event_summary.total > 0 {
        let eq = &report.event_summary.evidence_quality;
        line!(font_bold, 14.0_f32, left, y, "Evidence Quality");
        y -= 7.0;
        line!(font, 9.0_f32, left, y,
            "CV model confidence per event: Certified (>=0.8) / Degraded (0.5-0.8) / Rejected (<0.5).");
        y -= 7.0;
        line!(font, 10.0_f32, left, y, format!("Certified  (full actuarial weight):  {}", eq.certified));
        y -= 6.0;
        line!(font, 10.0_f32, left, y, format!("Degraded   (reduced weight):          {}", eq.degraded));
        y -= 6.0;
        line!(font, 10.0_f32, left, y, format!("Rejected   (recorded, not admissible):{}", eq.rejected));
        y -= 10.0;
    }

    // Risk Events by Rule
    line!(font_bold, 14.0_f32, left, y, "Violations by Safety Rule");
    y -= 7.0;
    line!(font, 9.0_f32, left, y,
        "Regulation column cites the exact clause that defines the threshold.");
    y -= 8.0;
    for row in &report.rule_frequencies {
        line!(font_bold, 10.0_f32, left, y,
            format!("{} — {} violation(s) — {}", row.rule_id, row.count, row.severity_str));
        y -= 5.5;
        // Wrap regulation text at ~90 chars
        let reg = &row.regulation;
        if reg.len() > 90 {
            line!(font, 9.0_f32, left + 4.0, y, &reg[..90]);
            y -= 5.0;
            line!(font, 9.0_f32, left + 4.0, y, &reg[90..]);
        } else {
            line!(font, 9.0_f32, left + 4.0, y, reg.as_str());
        }
        y -= 7.0;
        if y < 25.0 { break; }
    }
    y -= 3.0;

    // Trend
    if y > 25.0 {
        line!(font_bold, 14.0_f32, left, y, "Risk Trend");
        y -= 8.0;
        let (trend_label, trend_desc) = match report.trend {
            RiskTrend::Stable  => ("Stable",  "Violation rate is consistent — no escalation detected."),
            RiskTrend::Rising  => ("Rising",  "Violation rate is increasing — escalating risk, action required."),
            RiskTrend::Falling => ("Falling", "Violation rate is decreasing — safety conditions improving."),
        };
        line!(font_bold, 10.0_f32, left, y, trend_label);
        y -= 6.0;
        line!(font, 9.0_f32, left, y, trend_desc);
        y -= 10.0;
    }

    // Entity Involvement
    if !report.entity_correlations.is_empty() && y > 25.0 {
        line!(font_bold, 14.0_f32, left, y, "Entity Involvement");
        y -= 7.0;
        line!(font, 9.0_f32, left, y,
            "Entities or pairs with highest violation counts — may indicate systemic workflow issues.");
        y -= 8.0;
        for row in &report.entity_correlations {
            let label = if row.entity_ids.len() > 1 {
                format!("{} (pair)", row.entity_ids.join(" + "))
            } else {
                row.entity_ids.join(", ")
            };
            let desc = if row.entity_ids.len() > 1 {
                format!("{} events — possible conflicting routes", row.event_count)
            } else {
                format!("{} events", row.event_count)
            };
            line!(font, 10.0_f32, left, y, format!("{label}: {desc}"));
            y -= 6.0;
            if y < 25.0 { break; }
        }
    }

    // Explanations — new page if any
    if !report.explanations.is_empty() {
        let (page2, layer2) = doc.add_page(Mm(210.0_f32), Mm(297.0_f32), "Explanations");
        let exp_layer = doc.get_page(page2).get_layer(layer2);
        let mut ey = 277.0_f32;

        macro_rules! eline {
            ($fnt:expr, $size:expr, $x:expr, $ey:expr, $text:expr) => {
                exp_layer.use_text($text, $size as f32, Mm($x), Mm($ey), &$fnt);
            };
        }

        eline!(font_bold, 16.0_f32, left, ey, "Event Explanations (LLM)");
        ey -= 7.0;
        eline!(font, 9.0_f32, left, ey,
            "Generated by local LLM at event time. Interprets physics measurements against cited regulation.");
        ey -= 10.0;

        for (i, entry) in report.explanations.iter().enumerate() {
            if ey < 30.0 { break; }
            eline!(font_bold, 11.0_f32, left, ey,
                format!("{}. {} — {}", i + 1, entry.rule_id, fmt_timestamp_ms(entry.timestamp_ms)));
            ey -= 6.0;
            // Word-wrap at ~95 chars per line
            let words: Vec<&str> = entry.text.split_whitespace().collect();
            let mut line_buf = String::new();
            for word in &words {
                if line_buf.len() + word.len() + 1 > 95 {
                    eline!(font, 9.5_f32, left + 3.0, ey, line_buf.as_str());
                    ey -= 5.5;
                    line_buf = word.to_string();
                    if ey < 30.0 { break; }
                } else {
                    if !line_buf.is_empty() { line_buf.push(' '); }
                    line_buf.push_str(word);
                }
            }
            if !line_buf.is_empty() && ey >= 30.0 {
                eline!(font, 9.5_f32, left + 3.0, ey, line_buf.as_str());
                ey -= 5.5;
            }
            ey -= 5.0;
        }
    }

    let mut buf = BufWriter::new(Vec::new());
    doc.save(&mut buf).expect("PDF save failed");
    buf.into_inner().expect("BufWriter flush failed")
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
            confidence_cv: 1.0,
            evidence_quality: EvidenceQuality::Certified,
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
            executive_summary: None,
            explanations: vec![],
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
        let config = ReportConfig { site_name: None, report_period: None, chain_valid: None, executive_summary: None, explanations: vec![] };
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
    fn render_pdf_returns_non_empty_bytes() {
        let events = vec![make_event("RULE_X", Severity::High)];
        let assessment = assess(&events, None);
        let config = ReportConfig { site_name: Some("Site A".to_string()), report_period: Some("2026-Q2".to_string()), chain_valid: None, executive_summary: None, explanations: vec![] };
        let report = generate_report(&events, &assessment, config);
        let bytes = render_pdf(&report);
        assert!(!bytes.is_empty(), "PDF bytes should not be empty");
        // PDF files start with the %PDF magic bytes
        assert!(bytes.starts_with(b"%PDF"), "PDF should start with %PDF header");
    }

    #[test]
    fn render_markdown_shows_executive_summary_before_summary_table() {
        let events = vec![make_event("RULE_X", Severity::High)];
        let assessment = assess(&events, None);
        let config = ReportConfig {
            site_name: None,
            report_period: None,
            chain_valid: None,
            executive_summary: Some("Three proximity breaches recorded. Recommend refresher training.".to_string()),
            explanations: vec![],
        };
        let report = generate_report(&events, &assessment, config);
        let md = render_markdown(&report);
        assert!(md.contains("## Executive Summary"), "markdown should contain Executive Summary heading");
        assert!(md.contains("Three proximity breaches"), "markdown should contain summary text");
        // Executive Summary must appear before the event table
        let exec_pos = md.find("## Executive Summary").unwrap();
        let summary_pos = md.find("## Summary").unwrap();
        assert!(exec_pos < summary_pos, "Executive Summary should precede the Summary table");
    }

    #[test]
    fn render_pdf_includes_executive_summary() {
        let events = vec![make_event("RULE_X", Severity::High)];
        let assessment = assess(&events, None);
        let config = ReportConfig {
            site_name: None,
            report_period: None,
            chain_valid: None,
            executive_summary: Some("Summary for PDF test.".to_string()),
            explanations: vec![],
        };
        let report = generate_report(&events, &assessment, config);
        let bytes = render_pdf(&report);
        assert!(!bytes.is_empty());
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn render_markdown_no_executive_summary_section_when_none() {
        let events = vec![make_event("RULE_X", Severity::Low)];
        let assessment = assess(&events, None);
        let config = ReportConfig { site_name: None, report_period: None, chain_valid: None, executive_summary: None, explanations: vec![] };
        let report = generate_report(&events, &assessment, config);
        let md = render_markdown(&report);
        assert!(!md.contains("## Executive Summary"), "should not render section when None");
    }

    #[test]
    fn render_markdown_shows_chain_valid_when_some() {
        let events = vec![make_event("RULE_X", Severity::Low)];
        let assessment = assess(&events, None);
        let config = ReportConfig { site_name: None, report_period: None, chain_valid: Some(false), executive_summary: None, explanations: vec![] };
        let report = generate_report(&events, &assessment, config);
        let md = render_markdown(&report);
        assert!(md.contains("## Audit Chain"));
        assert!(md.contains("**FAIL**"));
    }
}
