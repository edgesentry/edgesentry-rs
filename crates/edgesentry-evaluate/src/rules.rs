use serde::Deserialize;

use edgesentry_ingest::entity::{Entity, EntityClass, Vec2};
use edgesentry_compute::{euclidean_distance, relative_velocity, time_to_collision, zone_membership};

// ── Public types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Deserialize, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// A evaluated condition type, parsed from the `condition` string in rules.json.
#[derive(Debug, Clone)]
pub enum Condition {
    /// Fire when the distance between any two entities drops below threshold (metres).
    DistanceLt(f32),
    /// Fire when the time-to-collision between any two approaching entities drops below threshold (s).
    TtcLt(f32),
    /// Fire when any entity's position is inside the given polygon.
    ZoneMember(Vec<Vec2>),
    /// Fire when an AisGap entity's gap duration (velocity.x in seconds) exceeds threshold.
    AisGapGt(f32),
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub rule_id: String,
    pub condition: Condition,
    pub severity: Severity,
    pub regulation: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RiskEvent {
    pub rule_id: String,
    pub severity: Severity,
    /// Exact regulation clause that was violated.
    pub regulation: String,
    /// IDs of the entities involved (two for proximity/TTC rules, one for zone rules).
    pub entity_ids: Vec<String>,
    /// The physical measurement that triggered the event (distance in m, TTC in s, or 1.0 for zone).
    pub measured_value: f32,
    /// The threshold that was breached.
    pub threshold: f32,
    pub timestamp_ms: u64,
}

// ── JSON loading ──────────────────────────────────────────────────────────────

/// Raw rule as it appears in rules.json — condition is still a string.
#[derive(Debug, Deserialize)]
struct RuleJson {
    rule_id: String,
    condition: String,
    severity: Severity,
    regulation: String,
    /// Required for `zone_member` conditions; polygon vertices as [x, y] pairs.
    zone: Option<Vec<[f32; 2]>>,
}

/// Load and parse rules from a JSON string (contents of rules.json).
///
/// # Errors
/// Returns an error string if the JSON is malformed or a condition cannot be parsed.
pub fn load_rules(json: &str) -> Result<Vec<Rule>, String> {
    let raws: Vec<RuleJson> =
        serde_json::from_str(json).map_err(|e| format!("JSON parse error: {e}"))?;
    raws.into_iter().map(rule_from_json).collect()
}

fn rule_from_json(raw: RuleJson) -> Result<Rule, String> {
    let condition = parse_condition(&raw.condition, raw.zone)?;
    Ok(Rule {
        rule_id: raw.rule_id,
        condition,
        severity: raw.severity,
        regulation: raw.regulation,
    })
}

fn parse_condition(s: &str, zone: Option<Vec<[f32; 2]>>) -> Result<Condition, String> {
    let s = s.trim();
    if s == "zone_member" {
        let verts = zone.ok_or_else(|| {
            "condition 'zone_member' requires a 'zone' polygon in the rule".to_string()
        })?;
        let polygon = verts.into_iter().map(|[x, y]| Vec2::new(x, y)).collect();
        return Ok(Condition::ZoneMember(polygon));
    }
    if let Some(rest) = s.strip_prefix("distance < ") {
        let t: f32 = rest.trim().parse().map_err(|_| format!("invalid threshold in '{s}'"))?;
        return Ok(Condition::DistanceLt(t));
    }
    if let Some(rest) = s.strip_prefix("ttc < ") {
        let t: f32 = rest.trim().parse().map_err(|_| format!("invalid threshold in '{s}'"))?;
        return Ok(Condition::TtcLt(t));
    }
    if let Some(rest) = s.strip_prefix("ais_gap > ") {
        let t: f32 = rest.trim().parse().map_err(|_| format!("invalid threshold in '{s}'"))?;
        return Ok(Condition::AisGapGt(t));
    }
    Err(format!("unknown condition expression: '{s}'"))
}

// ── Evaluation ────────────────────────────────────────────────────────────────

/// Evaluate all rules against the current entity snapshot.
/// Returns one `RiskEvent` per (rule, entity-pair or entity) that breaches a threshold.
pub fn evaluate(rules: &[Rule], entities: &[Entity], timestamp_ms: u64) -> Vec<RiskEvent> {
    let mut events = Vec::new();

    for rule in rules {
        match &rule.condition {
            Condition::DistanceLt(threshold) => {
                for i in 0..entities.len() {
                    for j in (i + 1)..entities.len() {
                        let dist = euclidean_distance(&entities[i], &entities[j]);
                        if dist < *threshold {
                            events.push(RiskEvent {
                                rule_id: rule.rule_id.clone(),
                                severity: rule.severity.clone(),
                                regulation: rule.regulation.clone(),
                                entity_ids: vec![
                                    entities[i].id.clone(),
                                    entities[j].id.clone(),
                                ],
                                measured_value: dist,
                                threshold: *threshold,
                                timestamp_ms,
                            });
                        }
                    }
                }
            }
            Condition::TtcLt(threshold) => {
                for i in 0..entities.len() {
                    for j in (i + 1)..entities.len() {
                        let dist = euclidean_distance(&entities[i], &entities[j]);
                        let rv = relative_velocity(&entities[i], &entities[j]);
                        let ttc = time_to_collision(dist, rv);
                        if ttc < *threshold {
                            events.push(RiskEvent {
                                rule_id: rule.rule_id.clone(),
                                severity: rule.severity.clone(),
                                regulation: rule.regulation.clone(),
                                entity_ids: vec![
                                    entities[i].id.clone(),
                                    entities[j].id.clone(),
                                ],
                                measured_value: ttc,
                                threshold: *threshold,
                                timestamp_ms,
                            });
                        }
                    }
                }
            }
            Condition::ZoneMember(polygon) => {
                for entity in entities {
                    if zone_membership(entity.position.clone(), polygon) {
                        events.push(RiskEvent {
                            rule_id: rule.rule_id.clone(),
                            severity: rule.severity.clone(),
                            regulation: rule.regulation.clone(),
                            entity_ids: vec![entity.id.clone()],
                            measured_value: 1.0,
                            threshold: 0.0,
                            timestamp_ms,
                        });
                    }
                }
            }
            Condition::AisGapGt(threshold) => {
                for entity in entities {
                    if entity.class == EntityClass::AisGap && entity.velocity.x > *threshold {
                        events.push(RiskEvent {
                            rule_id: rule.rule_id.clone(),
                            severity: rule.severity.clone(),
                            regulation: rule.regulation.clone(),
                            entity_ids: vec![entity.id.clone()],
                            measured_value: entity.velocity.x,
                            threshold: *threshold,
                            timestamp_ms,
                        });
                    }
                }
            }
        }
    }

    events
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use edgesentry_ingest::entity::{Entity, EntityClass, Vec2};

    fn entity(id: &str, x: f32, y: f32, vx: f32, vy: f32) -> Entity {
        Entity {
            id: id.into(),
            class: EntityClass::Forklift,
            position: Vec2::new(x, y),
            velocity: Vec2::new(vx, vy),
            timestamp_ms: 0,
        }
    }

    const DEMO_RULES_JSON: &str = r#"[
        {"rule_id":"PROXIMITY_ALERT","condition":"distance < 5.0","severity":"HIGH","regulation":"Site Safety Procedure §3.1"},
        {"rule_id":"EXCLUSION_ZONE_BREACH","condition":"zone_member","severity":"CRITICAL","regulation":"Site Safety Procedure §4.1","zone":[[0,0],[10,0],[10,10],[0,10]]},
        {"rule_id":"TTC_ALERT","condition":"ttc < 3.0","severity":"HIGH","regulation":"Site Safety Procedure §3.2"}
    ]"#;

    // ── load_rules ────────────────────────────────────────────────────────

    #[test]
    fn load_parses_three_rules() {
        let rules = load_rules(DEMO_RULES_JSON).unwrap();
        assert_eq!(rules.len(), 3);
    }

    #[test]
    fn load_rule_ids_correct() {
        let rules = load_rules(DEMO_RULES_JSON).unwrap();
        assert_eq!(rules[0].rule_id, "PROXIMITY_ALERT");
        assert_eq!(rules[1].rule_id, "EXCLUSION_ZONE_BREACH");
        assert_eq!(rules[2].rule_id, "TTC_ALERT");
    }

    #[test]
    fn load_severities_correct() {
        let rules = load_rules(DEMO_RULES_JSON).unwrap();
        assert_eq!(rules[0].severity, Severity::High);
        assert_eq!(rules[1].severity, Severity::Critical);
    }

    #[test]
    fn load_condition_distance_threshold() {
        let rules = load_rules(DEMO_RULES_JSON).unwrap();
        match &rules[0].condition {
            Condition::DistanceLt(t) => assert!((*t - 5.0).abs() < 1e-5),
            other => panic!("expected DistanceLt, got {other:?}"),
        }
    }

    #[test]
    fn load_condition_zone_has_polygon() {
        let rules = load_rules(DEMO_RULES_JSON).unwrap();
        match &rules[1].condition {
            Condition::ZoneMember(polygon) => assert_eq!(polygon.len(), 4),
            other => panic!("expected ZoneMember, got {other:?}"),
        }
    }

    #[test]
    fn load_condition_ttc_threshold() {
        let rules = load_rules(DEMO_RULES_JSON).unwrap();
        match &rules[2].condition {
            Condition::TtcLt(t) => assert!((*t - 3.0).abs() < 1e-5),
            other => panic!("expected TtcLt, got {other:?}"),
        }
    }

    #[test]
    fn load_invalid_json_returns_error() {
        assert!(load_rules("{not json}").is_err());
    }

    #[test]
    fn load_zone_member_without_zone_returns_error() {
        let json = r#"[{"rule_id":"X","condition":"zone_member","severity":"HIGH","regulation":"r"}]"#;
        assert!(load_rules(json).is_err());
    }

    #[test]
    fn load_unknown_condition_returns_error() {
        let json = r#"[{"rule_id":"X","condition":"unknown_op > 5","severity":"HIGH","regulation":"r"}]"#;
        assert!(load_rules(json).is_err());
    }

    // ── evaluate — distance ───────────────────────────────────────────────

    #[test]
    fn evaluate_distance_breach_fires_event() {
        let rules = load_rules(DEMO_RULES_JSON).unwrap();
        let entities = vec![
            entity("FL-01", 0.0, 0.0, 1.4, 0.0),
            entity("W-03", 3.2, 0.0, 0.0, 0.0),
        ];
        let events = evaluate(&rules, &entities, 1000);
        let evt = events.iter().find(|e| e.rule_id == "PROXIMITY_ALERT").unwrap();
        assert!((evt.measured_value - 3.2).abs() < 1e-4);
        assert_eq!(evt.threshold, 5.0);
        assert_eq!(evt.entity_ids, vec!["FL-01", "W-03"]);
        assert_eq!(evt.timestamp_ms, 1000);
    }

    #[test]
    fn evaluate_distance_no_breach_when_safe() {
        let rules = load_rules(DEMO_RULES_JSON).unwrap();
        let entities = vec![
            entity("FL-01", 0.0, 0.0, 0.0, 0.0),
            entity("W-03", 6.0, 0.0, 0.0, 0.0),
        ];
        let events = evaluate(&rules, &entities, 0);
        assert!(!events.iter().any(|e| e.rule_id == "PROXIMITY_ALERT"));
    }

    #[test]
    fn evaluate_distance_multiple_pairs() {
        // Three entities, two pairs breach clearance
        let rules = load_rules(DEMO_RULES_JSON).unwrap();
        let entities = vec![
            entity("A", 0.0, 0.0, 0.0, 0.0),
            entity("B", 1.0, 0.0, 0.0, 0.0), // A-B: 1m → breach
            entity("C", 2.0, 0.0, 0.0, 0.0), // A-C: 2m → breach; B-C: 1m → breach
        ];
        let events: Vec<_> = evaluate(&rules, &entities, 0)
            .into_iter()
            .filter(|e| e.rule_id == "PROXIMITY_ALERT")
            .collect();
        assert_eq!(events.len(), 3);
    }

    // ── evaluate — zone ───────────────────────────────────────────────────

    #[test]
    fn evaluate_zone_breach_fires_for_entity_inside() {
        let rules = load_rules(DEMO_RULES_JSON).unwrap();
        let entities = vec![entity("V-01", 5.0, 5.0, 0.0, 0.0)]; // inside [0,0]-[10,10]
        let events = evaluate(&rules, &entities, 0);
        assert!(events.iter().any(|e| e.rule_id == "EXCLUSION_ZONE_BREACH"));
    }

    #[test]
    fn evaluate_zone_no_breach_when_outside() {
        let rules = load_rules(DEMO_RULES_JSON).unwrap();
        let entities = vec![entity("V-01", 15.0, 5.0, 0.0, 0.0)]; // outside zone
        let events = evaluate(&rules, &entities, 0);
        assert!(!events.iter().any(|e| e.rule_id == "EXCLUSION_ZONE_BREACH"));
    }

    // ── evaluate — TTC ────────────────────────────────────────────────────

    #[test]
    fn evaluate_ttc_breach_fires_when_fast_approach() {
        let rules = load_rules(DEMO_RULES_JSON).unwrap();
        // 2m gap, 5 m/s approach → TTC = 0.4 s < 3.0 s
        let entities = vec![
            entity("FL-01", 0.0, 0.0, 5.0, 0.0),
            entity("W-03", 2.0, 0.0, 0.0, 0.0),
        ];
        let events = evaluate(&rules, &entities, 0);
        assert!(events.iter().any(|e| e.rule_id == "TTC_ALERT"));
    }

    #[test]
    fn evaluate_ttc_no_breach_when_slow_approach() {
        let rules = load_rules(DEMO_RULES_JSON).unwrap();
        // 20m gap, 1 m/s approach → TTC = 20 s > 3.0 s
        let entities = vec![
            entity("FL-01", 0.0, 0.0, 1.0, 0.0),
            entity("W-03", 20.0, 0.0, 0.0, 0.0),
        ];
        let events = evaluate(&rules, &entities, 0);
        assert!(!events.iter().any(|e| e.rule_id == "TTC_ALERT"));
    }

    #[test]
    fn evaluate_ttc_no_breach_when_receding() {
        let rules = load_rules(DEMO_RULES_JSON).unwrap();
        // Moving away — TTC = ∞
        let entities = vec![
            entity("FL-01", 0.0, 0.0, -3.0, 0.0),
            entity("W-03", 2.0, 0.0, 0.0, 0.0),
        ];
        let events = evaluate(&rules, &entities, 0);
        assert!(!events.iter().any(|e| e.rule_id == "TTC_ALERT"));
    }

    // ── evaluate — empty inputs ───────────────────────────────────────────

    #[test]
    fn evaluate_empty_entities_returns_no_events() {
        let rules = load_rules(DEMO_RULES_JSON).unwrap();
        assert!(evaluate(&rules, &[], 0).is_empty());
    }

    #[test]
    fn evaluate_empty_rules_returns_no_events() {
        let entities = vec![entity("A", 0.0, 0.0, 1.0, 0.0)];
        assert!(evaluate(&[], &entities, 0).is_empty());
    }

    // ── Scenario: roadmap demo ────────────────────────────────────────────
    // Forklift FL-01 at (0,0) moving at 1.4 m/s toward Worker W-03 at (3.2,0).
    // Expects PROXIMITY_ALERT and TTC_ALERT to fire.

    #[test]
    fn scenario_roadmap_demo_fires_clearance_and_ttc() {
        let rules = load_rules(DEMO_RULES_JSON).unwrap();
        let entities = vec![
            entity("FL-01", 0.0, 0.0, 1.4, 0.0),
            entity("W-03", 3.2, 0.0, 0.0, 0.0),
        ];
        let events = evaluate(&rules, &entities, 14230000);
        let rule_ids: Vec<&str> = events.iter().map(|e| e.rule_id.as_str()).collect();
        assert!(rule_ids.contains(&"PROXIMITY_ALERT"), "clearance rule should fire");
        assert!(rule_ids.contains(&"TTC_ALERT"), "TTC rule should fire");
    }

    #[test]
    fn scenario_roadmap_demo_clearance_event_values() {
        let rules = load_rules(DEMO_RULES_JSON).unwrap();
        let entities = vec![
            entity("FL-01", 0.0, 0.0, 1.4, 0.0),
            entity("W-03", 3.2, 0.0, 0.0, 0.0),
        ];
        let events = evaluate(&rules, &entities, 14230000);
        let evt = events.iter().find(|e| e.rule_id == "PROXIMITY_ALERT").unwrap();
        assert!((evt.measured_value - 3.2).abs() < 1e-4);
        assert_eq!(evt.severity, Severity::High);
        assert!(evt.regulation.contains("§3.1"));
    }

    // ── evaluate — AisGapGt ───────────────────────────────────────────────

    const AIS_RULES_JSON: &str = r#"[
        {"rule_id":"AIS_TRACK_GAP","condition":"ais_gap > 480","severity":"CRITICAL","regulation":"SOLAS V/19"}
    ]"#;

    fn ais_gap_entity(id: &str, gap_s: f32) -> Entity {
        Entity {
            id: id.into(),
            class: EntityClass::AisGap,
            position: Vec2::new(0.0, 0.0),
            velocity: Vec2::new(gap_s, 0.0),
            timestamp_ms: 0,
        }
    }

    #[test]
    fn load_ais_gap_condition_parses_threshold() {
        let rules = load_rules(AIS_RULES_JSON).unwrap();
        assert_eq!(rules.len(), 1);
        match &rules[0].condition {
            Condition::AisGapGt(t) => assert!((*t - 480.0).abs() < 1e-5),
            other => panic!("expected AisGapGt, got {other:?}"),
        }
    }

    #[test]
    fn evaluate_ais_gap_fires_when_gap_exceeds_threshold() {
        let rules = load_rules(AIS_RULES_JSON).unwrap();
        // gap_s=600 > threshold=480 → must fire
        let entities = vec![ais_gap_entity("563012345", 600.0)];
        let events = evaluate(&rules, &entities, 0);
        assert!(events.iter().any(|e| e.rule_id == "AIS_TRACK_GAP"),
            "AIS_TRACK_GAP should fire when gap > threshold");
        let evt = events.iter().find(|e| e.rule_id == "AIS_TRACK_GAP").unwrap();
        assert!((evt.measured_value - 600.0).abs() < 1e-3);
        assert!((evt.threshold - 480.0).abs() < 1e-3);
        assert_eq!(evt.severity, Severity::Critical);
    }

    #[test]
    fn evaluate_ais_gap_no_fire_when_gap_below_threshold() {
        let rules = load_rules(AIS_RULES_JSON).unwrap();
        // gap_s=300 < threshold=480 → must not fire
        let entities = vec![ais_gap_entity("563012345", 300.0)];
        let events = evaluate(&rules, &entities, 0);
        assert!(!events.iter().any(|e| e.rule_id == "AIS_TRACK_GAP"),
            "AIS_TRACK_GAP should NOT fire when gap < threshold");
    }

    #[test]
    fn evaluate_ais_gap_no_fire_for_vessel_entity() {
        let rules = load_rules(AIS_RULES_JSON).unwrap();
        // A Vessel entity (not AisGap) must not trigger the AisGapGt rule
        let entities = vec![entity("563012345", 0.0, 0.0, 600.0, 0.0)];
        let events = evaluate(&rules, &entities, 0);
        assert!(!events.iter().any(|e| e.rule_id == "AIS_TRACK_GAP"),
            "AIS_TRACK_GAP must not fire for a non-AisGap entity");
    }

    #[test]
    fn evaluate_maritime_rules_load_correctly() {
        let rules_json = include_str!(
            "../../edgesentry-profile/fixtures/maritime-zone-test/rules.json"
        );
        let rules = load_rules(rules_json).expect("maritime-zone-test rules.json must parse");
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].rule_id, "RESTRICTED_ZONE_APPROACH");
        assert_eq!(rules[1].rule_id, "AIS_TRACK_GAP");
        match &rules[0].condition {
            Condition::ZoneMember(p) => assert_eq!(p.len(), 4),
            other => panic!("expected ZoneMember, got {other:?}"),
        }
        match &rules[1].condition {
            Condition::AisGapGt(t) => assert!((*t - 480.0).abs() < 1e-3),
            other => panic!("expected AisGapGt, got {other:?}"),
        }
    }

    #[test]
    fn evaluate_maritime_zone_fires_for_vessel_inside() {
        let rules_json = include_str!(
            "../../edgesentry-profile/fixtures/maritime-zone-test/rules.json"
        );
        let rules = load_rules(rules_json).unwrap();
        // Position at (0, 400) — inside polygon [[-300,200],[300,200],[300,700],[-300,700]]
        let vessel = Entity {
            id: "563012345".into(),
            class: EntityClass::Vessel,
            position: Vec2::new(0.0, 400.0),
            velocity: Vec2::new(0.0, 0.0),
            timestamp_ms: 0,
        };
        let events = evaluate(&rules, &[vessel], 0);
        assert!(events.iter().any(|e| e.rule_id == "RESTRICTED_ZONE_APPROACH"),
            "RESTRICTED_ZONE_APPROACH should fire for vessel at (0,400)");
    }

    #[test]
    fn evaluate_maritime_zone_no_fire_for_vessel_outside() {
        let rules_json = include_str!(
            "../../edgesentry-profile/fixtures/maritime-zone-test/rules.json"
        );
        let rules = load_rules(rules_json).unwrap();
        // Position at (0, -278) — outside zone (south of y=+200m boundary)
        let vessel = Entity {
            id: "563012345".into(),
            class: EntityClass::Vessel,
            position: Vec2::new(0.0, -278.0),
            velocity: Vec2::new(0.0, 0.0),
            timestamp_ms: 0,
        };
        let events = evaluate(&rules, &[vessel], 0);
        assert!(!events.iter().any(|e| e.rule_id == "RESTRICTED_ZONE_APPROACH"),
            "RESTRICTED_ZONE_APPROACH must not fire for vessel south of zone");
    }
}
