use edgesentry_evaluate::{RiskEvent, Severity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assessment {
    pub timestamp_ms: u64,
    /// Rules that fired more than once in the window.
    pub repeated_rules: Vec<RuleFrequency>,
    /// Entity pairs that appeared in multiple events.
    pub correlated_entities: Vec<EntityCorrelation>,
    /// Overall risk trend for the window.
    pub trend: RiskTrend,
    /// Total events analysed (current + history within window).
    pub event_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleFrequency {
    pub rule_id: String,
    pub count: usize,
    pub severity: Severity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityCorrelation {
    pub entity_ids: Vec<String>,
    pub event_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskTrend {
    Stable,
    Rising,
    Falling,
}

/// Analyse a set of RiskEvents and produce an Assessment.
///
/// Events are sorted by timestamp before analysis.
/// `window_sec`: if Some(n), only events within the last n seconds of the
///   newest event are included. If None, all events are included.
pub fn assess(events: &[RiskEvent], window_sec: Option<u64>) -> Assessment {
    // Sort by timestamp
    let mut sorted: Vec<&RiskEvent> = events.iter().collect();
    sorted.sort_by_key(|e| e.timestamp_ms);

    // Apply time window
    let windowed: Vec<&RiskEvent> = if let Some(w) = window_sec {
        let newest_ts = sorted.last().map(|e| e.timestamp_ms).unwrap_or(0);
        let cutoff_ms = newest_ts.saturating_sub(w * 1000);
        sorted.iter().copied().filter(|e| e.timestamp_ms >= cutoff_ms).collect()
    } else {
        sorted.clone()
    };

    let event_count = windowed.len();
    let timestamp_ms = windowed.last().map(|e| e.timestamp_ms).unwrap_or(0);

    // Rule frequency
    let mut rule_counts: std::collections::HashMap<String, (usize, Severity)> =
        std::collections::HashMap::new();
    for e in &windowed {
        let entry = rule_counts
            .entry(e.rule_id.clone())
            .or_insert((0, e.severity.clone()));
        entry.0 += 1;
    }
    let mut repeated_rules: Vec<RuleFrequency> = rule_counts
        .into_iter()
        .filter(|(_, (count, _))| *count > 1)
        .map(|(rule_id, (count, severity))| RuleFrequency { rule_id, count, severity })
        .collect();
    repeated_rules.sort_by_key(|r| std::cmp::Reverse(r.count));

    // Entity correlation
    let mut entity_counts: std::collections::HashMap<Vec<String>, usize> =
        std::collections::HashMap::new();
    for e in &windowed {
        let mut ids = e.entity_ids.clone();
        ids.sort();
        *entity_counts.entry(ids).or_insert(0) += 1;
    }
    let mut correlated_entities: Vec<EntityCorrelation> = entity_counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(entity_ids, event_count)| EntityCorrelation { entity_ids, event_count })
        .collect();
    correlated_entities.sort_by_key(|r| std::cmp::Reverse(r.event_count));

    // Trend: compare event rate in first half vs second half of window
    let trend = compute_trend(&windowed);

    Assessment {
        timestamp_ms,
        repeated_rules,
        correlated_entities,
        trend,
        event_count,
    }
}

fn compute_trend(events: &[&RiskEvent]) -> RiskTrend {
    if events.len() < 4 {
        return RiskTrend::Stable;
    }
    let mid = events.len() / 2;
    let first_half = &events[..mid];
    let second_half = &events[mid..];

    let first_span = span_ms(first_half);
    let second_span = span_ms(second_half);

    if first_span == 0 || second_span == 0 {
        return RiskTrend::Stable;
    }

    // events per millisecond
    let first_rate = first_half.len() as f64 / first_span as f64;
    let second_rate = second_half.len() as f64 / second_span as f64;

    let ratio = second_rate / first_rate;
    if ratio > 1.2 {
        RiskTrend::Rising
    } else if ratio < 0.8 {
        RiskTrend::Falling
    } else {
        RiskTrend::Stable
    }
}

fn span_ms(events: &[&RiskEvent]) -> u64 {
    match (events.first(), events.last()) {
        (Some(first), Some(last)) => last.timestamp_ms.saturating_sub(first.timestamp_ms),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgesentry_evaluate::Severity;

    fn make_event(rule_id: &str, ts: u64, entity_ids: Vec<String>) -> RiskEvent {
        RiskEvent {
            rule_id: rule_id.to_string(),
            severity: Severity::High,
            regulation: "test".to_string(),
            entity_ids,
            measured_value: 1.0,
            threshold: 5.0,
            timestamp_ms: ts,
            confidence_cv: 1.0,
            evidence_quality: edgesentry_evaluate::EvidenceQuality::Certified,
        }
    }

    #[test]
    fn empty_events_returns_empty_assessment() {
        let assessment = assess(&[], None);
        assert_eq!(assessment.event_count, 0);
        assert!(assessment.repeated_rules.is_empty());
        assert!(assessment.correlated_entities.is_empty());
        assert_eq!(assessment.trend, RiskTrend::Stable);
        assert_eq!(assessment.timestamp_ms, 0);
    }

    #[test]
    fn single_repeated_rule_is_detected() {
        let events = vec![
            make_event("PROXIMITY_ALERT", 1000, vec!["A".to_string(), "B".to_string()]),
            make_event("PROXIMITY_ALERT", 2000, vec!["A".to_string(), "B".to_string()]),
            make_event("PROXIMITY_ALERT", 3000, vec!["A".to_string(), "B".to_string()]),
        ];
        let assessment = assess(&events, None);
        assert_eq!(assessment.repeated_rules.len(), 1);
        assert_eq!(assessment.repeated_rules[0].rule_id, "PROXIMITY_ALERT");
        assert_eq!(assessment.repeated_rules[0].count, 3);
    }

    #[test]
    fn time_window_filters_old_events() {
        let events = vec![
            make_event("PROXIMITY_ALERT", 0, vec!["A".to_string()]),
            make_event("PROXIMITY_ALERT", 1000, vec!["A".to_string()]),
            // These are within 2 seconds of newest (10000 ms)
            make_event("PROXIMITY_ALERT", 8500, vec!["A".to_string()]),
            make_event("PROXIMITY_ALERT", 9000, vec!["A".to_string()]),
            make_event("PROXIMITY_ALERT", 10000, vec!["A".to_string()]),
        ];
        // window of 2 seconds from newest (10000 ms) → cutoff at 8000 ms
        let assessment = assess(&events, Some(2));
        assert_eq!(assessment.event_count, 3); // 8500, 9000, 10000
    }

    #[test]
    fn rising_trend_detected() {
        // 4 events: first half spread over 10000ms, second half spread over 1000ms
        // first_rate = 2/10000, second_rate = 2/1000 → ratio = 20x → Rising
        let events = vec![
            make_event("R", 0, vec!["A".to_string()]),
            make_event("R", 10000, vec!["A".to_string()]),
            make_event("R", 11000, vec!["A".to_string()]),
            make_event("R", 12000, vec!["A".to_string()]),
        ];
        let assessment = assess(&events, None);
        assert_eq!(assessment.trend, RiskTrend::Rising);
    }

    #[test]
    fn entity_correlation_detected() {
        let events = vec![
            make_event("R1", 1000, vec!["FL-01".to_string(), "W-03".to_string()]),
            make_event("R2", 2000, vec!["FL-01".to_string(), "W-03".to_string()]),
            make_event("R3", 3000, vec!["FL-01".to_string(), "W-03".to_string()]),
        ];
        let assessment = assess(&events, None);
        assert_eq!(assessment.correlated_entities.len(), 1);
        assert_eq!(assessment.correlated_entities[0].event_count, 3);
        let mut ids = assessment.correlated_entities[0].entity_ids.clone();
        ids.sort();
        assert_eq!(ids, vec!["FL-01", "W-03"]);
    }

    #[test]
    fn no_repeated_rules_when_all_distinct() {
        let events = vec![
            make_event("RULE_A", 1000, vec!["A".to_string()]),
            make_event("RULE_B", 2000, vec!["B".to_string()]),
            make_event("RULE_C", 3000, vec!["C".to_string()]),
        ];
        let assessment = assess(&events, None);
        assert!(assessment.repeated_rules.is_empty());
    }

    #[test]
    fn assessment_is_serializable() {
        let events = vec![
            make_event("PROXIMITY_ALERT", 1000, vec!["A".to_string(), "B".to_string()]),
            make_event("PROXIMITY_ALERT", 2000, vec!["A".to_string(), "B".to_string()]),
        ];
        let assessment = assess(&events, None);
        let json = serde_json::to_string(&assessment).unwrap();
        let decoded: Assessment = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.event_count, assessment.event_count);
        assert_eq!(decoded.repeated_rules.len(), assessment.repeated_rules.len());
    }
}
