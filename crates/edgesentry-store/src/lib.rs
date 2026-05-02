use edgesentry_evaluate::RiskEvent;

pub trait EventStore: Send + Sync {
    fn push(&mut self, event: RiskEvent);
    fn events(&self) -> &[RiskEvent];
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool { self.len() == 0 }
    fn clear(&mut self);
}

pub struct InMemoryStore {
    events: Vec<RiskEvent>,
}

impl InMemoryStore {
    pub fn new() -> Self { Self { events: Vec::new() } }
}

impl Default for InMemoryStore {
    fn default() -> Self { Self::new() }
}

impl EventStore for InMemoryStore {
    fn push(&mut self, event: RiskEvent) { self.events.push(event); }
    fn events(&self) -> &[RiskEvent] { &self.events }
    fn len(&self) -> usize { self.events.len() }
    fn clear(&mut self) { self.events.clear(); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgesentry_evaluate::{RiskEvent, Severity};

    fn make_event(rule_id: &str, ts: u64) -> RiskEvent {
        RiskEvent {
            rule_id: rule_id.to_string(),
            severity: Severity::High,
            regulation: "test".to_string(),
            entity_ids: vec!["A".to_string()],
            measured_value: 1.0,
            threshold: 5.0,
            timestamp_ms: ts,
            confidence_cv: 1.0,
            evidence_quality: edgesentry_evaluate::EvidenceQuality::Certified,
        }
    }

    #[test]
    fn new_store_is_empty() {
        let store = InMemoryStore::new();
        assert!(store.is_empty());
    }

    #[test]
    fn push_increases_len() {
        let mut store = InMemoryStore::new();
        store.push(make_event("A", 1000));
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn events_returns_pushed_items() {
        let mut store = InMemoryStore::new();
        store.push(make_event("PROXIMITY_ALERT", 1000));
        store.push(make_event("TTC_ALERT", 2000));
        assert_eq!(store.events().len(), 2);
        assert_eq!(store.events()[0].rule_id, "PROXIMITY_ALERT");
    }

    #[test]
    fn clear_empties_store() {
        let mut store = InMemoryStore::new();
        store.push(make_event("A", 1000));
        store.clear();
        assert!(store.is_empty());
    }
}
