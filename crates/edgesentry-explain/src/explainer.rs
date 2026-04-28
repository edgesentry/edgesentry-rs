use edgesentry_evaluate::RiskEvent;

use crate::kb::KnowledgeBase;
use crate::llm::LlmClient;

/// The explanation produced for a single RiskEvent.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Explanation {
    pub rule_id: String,
    pub kb_snippet: String,
    pub text: String,
    /// False if the LLM output cited a regulation clause absent from `kb_snippet`.
    pub grounded: bool,
    /// Unix timestamp in milliseconds (from the source RiskEvent).
    pub timestamp_ms: u64,
}

pub struct Explainer {
    kb: KnowledgeBase,
    llm: LlmClient,
}

impl Explainer {
    pub fn new(kb: KnowledgeBase, llm: LlmClient) -> Self {
        Self { kb, llm }
    }

    /// Generate a plain-language explanation for a RiskEvent.
    ///
    /// If the KB has no entry for the rule, returns an explanation with the snippet set to
    /// "No KB entry" and grounded=false.
    /// If the LLM is unavailable, returns Err with the connection error.
    pub fn explain(&self, event: &RiskEvent) -> Result<Explanation, String> {
        let kb_snippet = match self.kb.get(&event.rule_id) {
            Some(s) => s.to_string(),
            None => {
                return Ok(Explanation {
                    rule_id: event.rule_id.clone(),
                    kb_snippet: "No KB entry".to_string(),
                    text: format!(
                        "Rule {} fired (measured {:.2}, threshold {:.2}). No regulatory KB entry found.",
                        event.rule_id, event.measured_value, event.threshold
                    ),
                    grounded: false,
                    timestamp_ms: event.timestamp_ms,
                });
            }
        };

        let entity_desc = match event.entity_ids.as_slice() {
            [a, b] => format!("{a} and {b}"),
            [a] => a.clone(),
            ids => ids.join(", "),
        };

        let prompt = build_prompt(event, &entity_desc, &kb_snippet);
        let raw = self.llm.generate(&prompt)?;
        let text = raw.trim().to_string();
        let grounded = is_grounded(&text, &kb_snippet);

        Ok(Explanation {
            rule_id: event.rule_id.clone(),
            kb_snippet,
            text,
            grounded,
            timestamp_ms: event.timestamp_ms,
        })
    }
}

fn build_prompt(event: &RiskEvent, entity_desc: &str, kb_snippet: &str) -> String {
    format!(
        "Event: {rule_id} fired. Measured: {value:.2}. Threshold: {threshold:.2}.\n\
         Entities involved: {entities}.\n\
         Regulation: {snippet}\n\n\
         Generate a one-paragraph plain-language alert for a safety officer. \
         Only cite regulation text provided above. Do not add any regulation references \
         not present in the text above.",
        rule_id = event.rule_id,
        value = event.measured_value,
        threshold = event.threshold,
        entities = entity_desc,
        snippet = kb_snippet,
    )
}

/// Heuristic grounding check: confirm the LLM output doesn't reference regulation
/// section numbers (§X.X) that are absent from the KB snippet.
fn is_grounded(text: &str, kb_snippet: &str) -> bool {
    // Extract all §N.N style references from LLM output
    let llm_refs = extract_section_refs(text);
    let kb_refs = extract_section_refs(kb_snippet);
    // All refs in LLM output must appear in KB snippet
    llm_refs.iter().all(|r| kb_refs.contains(r))
}

fn extract_section_refs(text: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '§' {
            let mut s = String::new();
            for nc in chars.by_ref() {
                if nc.is_ascii_digit() || nc == '.' {
                    s.push(nc);
                } else {
                    break;
                }
            }
            if !s.is_empty() {
                refs.push(s);
            }
        }
    }
    refs
}

/// Pick N events from the input list based on the given strategy.
pub enum PickStrategy {
    Severity,
    Time,
    Random,
}

pub fn pick_events<'a>(events: &'a [RiskEvent], n: usize, strategy: PickStrategy) -> Vec<&'a RiskEvent> {
    let mut sorted: Vec<&RiskEvent> = events.iter().collect();
    match strategy {
        PickStrategy::Severity => {
            sorted.sort_by(|a, b| {
                severity_rank(&b.severity).cmp(&severity_rank(&a.severity))
            });
        }
        PickStrategy::Time => {
            sorted.sort_by(|a, b| b.timestamp_ms.cmp(&a.timestamp_ms)); // newest first
        }
        PickStrategy::Random => {
            // simple deterministic shuffle for testing — no rand dep needed
            for i in (1..sorted.len()).rev() {
                let j = (i.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)) % (i + 1);
                sorted.swap(i, j);
            }
        }
    }
    sorted.into_iter().take(n).collect()
}

fn severity_rank(s: &edgesentry_evaluate::Severity) -> u8 {
    use edgesentry_evaluate::Severity::*;
    match s { Critical => 3, High => 2, Medium => 1, Low => 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgesentry_evaluate::{RiskEvent, Severity};

    fn make_event(rule_id: &str, severity: Severity, ts: u64) -> RiskEvent {
        RiskEvent {
            rule_id: rule_id.to_string(),
            severity,
            regulation: "test".to_string(),
            entity_ids: vec!["FL-01".to_string(), "W-03".to_string()],
            measured_value: 3.2,
            threshold: 5.0,
            timestamp_ms: ts,
        }
    }

    #[test]
    fn extract_refs_finds_section_numbers() {
        let refs = extract_section_refs("See §3.1 and §4.2 for details.");
        assert_eq!(refs, vec!["3.1", "4.2"]);
    }

    #[test]
    fn extract_refs_empty_when_none() {
        assert!(extract_section_refs("No refs here.").is_empty());
    }

    #[test]
    fn grounded_when_all_refs_in_kb() {
        let kb = "Site Safety §3.1 requires 5 m clearance.";
        let llm = "According to §3.1, clearance was breached.";
        assert!(is_grounded(llm, kb));
    }

    #[test]
    fn not_grounded_when_extra_ref_hallucinated() {
        let kb = "Site Safety §3.1 requires 5 m clearance.";
        let llm = "According to §3.1 and §7.4, clearance was breached.";
        assert!(!is_grounded(llm, kb));
    }

    #[test]
    fn grounded_when_no_refs_in_output() {
        let kb = "Site Safety §3.1 requires 5 m clearance.";
        let llm = "The clearance was breached. Immediate action required.";
        assert!(is_grounded(llm, kb));
    }

    #[test]
    fn build_prompt_contains_key_fields() {
        let event = make_event("PROXIMITY_ALERT", Severity::High, 1000);
        let prompt = build_prompt(&event, "FL-01 and W-03", "Minimum 5 m clearance.");
        assert!(prompt.contains("PROXIMITY_ALERT"));
        assert!(prompt.contains("3.20"));
        assert!(prompt.contains("5.00"));
        assert!(prompt.contains("FL-01 and W-03"));
        assert!(prompt.contains("Minimum 5 m clearance."));
    }

    #[test]
    fn no_kb_entry_returns_ungrounded_explanation() {
        use std::collections::HashMap;
        use crate::kb::KnowledgeBase;
        use crate::llm::LlmClient;

        let kb = KnowledgeBase::from_map(HashMap::new());
        let llm = LlmClient::new("http://localhost:8080", "test-model");
        let explainer = Explainer::new(kb, llm);

        let event = make_event("UNKNOWN_RULE", Severity::Medium, 5000);
        let explanation = explainer.explain(&event).unwrap();
        assert_eq!(explanation.rule_id, "UNKNOWN_RULE");
        assert_eq!(explanation.kb_snippet, "No KB entry");
        assert!(!explanation.grounded);
        assert_eq!(explanation.timestamp_ms, 5000);
    }

    #[test]
    fn pick_events_severity_orders_highest_first() {
        let events = vec![
            make_event("R1", Severity::Low, 1000),
            make_event("R2", Severity::Critical, 2000),
            make_event("R3", Severity::Medium, 3000),
            make_event("R4", Severity::High, 4000),
        ];
        let picked = pick_events(&events, 2, PickStrategy::Severity);
        assert_eq!(picked.len(), 2);
        assert_eq!(picked[0].rule_id, "R2"); // Critical
        assert_eq!(picked[1].rule_id, "R4"); // High
    }

    #[test]
    fn pick_events_time_orders_newest_first() {
        let events = vec![
            make_event("R1", Severity::High, 1000),
            make_event("R2", Severity::High, 3000),
            make_event("R3", Severity::High, 2000),
        ];
        let picked = pick_events(&events, 2, PickStrategy::Time);
        assert_eq!(picked.len(), 2);
        assert_eq!(picked[0].timestamp_ms, 3000);
        assert_eq!(picked[1].timestamp_ms, 2000);
    }

    #[test]
    fn pick_events_returns_at_most_n() {
        let events = vec![
            make_event("R1", Severity::High, 1000),
            make_event("R2", Severity::High, 2000),
        ];
        let picked = pick_events(&events, 10, PickStrategy::Time);
        assert_eq!(picked.len(), 2); // only 2 available
    }

    #[test]
    fn pick_events_random_returns_n_items() {
        let events: Vec<RiskEvent> = (0..5).map(|i| make_event("R", Severity::High, i * 1000)).collect();
        let picked = pick_events(&events, 3, PickStrategy::Random);
        assert_eq!(picked.len(), 3);
    }
}
