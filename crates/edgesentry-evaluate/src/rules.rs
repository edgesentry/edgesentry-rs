use serde::Deserialize;

use edgesentry_types::{Entity, EntityClass, Vec2};
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
    use edgesentry_types::{Entity, EntityClass, Vec2};

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

    // ── Physics validation scenarios ──────────────────────────────────────────
    //
    // Each scenario is defined analytically with hand-calculated ground truth.
    // These tests constitute the physics validation evidence for submission.
    // See docs/pipeline/physics-validation.md.

    const PORT_SAFETY_RULES: &str = r#"[
        {"rule_id":"PROXIMITY_ALERT","condition":"distance < 5.0","severity":"HIGH","regulation":"Site Safety §3.1"},
        {"rule_id":"TTC_ALERT",      "condition":"ttc < 3.0",      "severity":"HIGH","regulation":"Site Safety §3.2"}
    ]"#;

    const ZONE_RULES: &str = r#"[
        {"rule_id":"ZONE_ENTRY","condition":"zone_member","severity":"HIGH","regulation":"Site Safety §4.1",
         "zone":[[300,200],[600,200],[600,500],[300,500]]}
    ]"#;

    // Scenario 1 — Proximity approach
    //
    // Vehicle closes from 12.0 m to 1.0 m over 15 frames at constant velocity.
    // step = (12.0 - 1.0) / 14 = 11/14 m/frame  (1 frame = 1 s)
    //
    // Hand-calculated first alert frame:
    //   12.0 - N*(11/14) < 5.0  →  N > 7*14/11 = 8.909  →  frame 9
    //   distance at frame 9 = 12.0 - 9*(11/14) = 69/14 ≈ 4.929 m
    //
    // Hand-calculated first TTC alert frame:
    //   TTC = distance/speed = (12.0 - N*step)/step = 12*14/11 - N
    //   TTC < 3.0  →  N > 12*14/11 - 3 = 135/11 = 12.27  →  frame 13
    //   TTC at frame 13 = 25/14 / (11/14) = 25/11 ≈ 2.273 s
    #[test]
    fn scenario_1_proximity_approach_first_alert_frame() {
        let rules = load_rules(PORT_SAFETY_RULES).unwrap();
        let step = 11.0_f32 / 14.0;
        let worker = entity("W-01", 0.0, 0.0, 0.0, 0.0);

        let first_proximity_frame = (0..15).find(|&n| {
            let x = 12.0 - n as f32 * step;
            let vehicle = entity("FL-01", x, 0.0, -step, 0.0);
            evaluate(&rules, &[vehicle, worker.clone()], n as u64 * 1000)
                .iter().any(|e| e.rule_id == "PROXIMITY_ALERT")
        });

        assert_eq!(first_proximity_frame, Some(9),
            "PROXIMITY_ALERT must first fire at frame 9 (distance ≈ 4.929 m)");

        // Verify measured_value at frame 9 matches hand-calculated distance
        let x9 = 12.0_f32 - 9.0 * step;
        let vehicle9 = entity("FL-01", x9, 0.0, -step, 0.0);
        let events = evaluate(&rules, &[vehicle9, worker.clone()], 9000);
        let evt = events.iter().find(|e| e.rule_id == "PROXIMITY_ALERT").unwrap();
        let expected_dist = 69.0_f32 / 14.0;
        assert!((evt.measured_value - expected_dist).abs() < 1e-3,
            "measured distance at frame 9 must be 69/14 ≈ {:.4} m, got {:.4}",
            expected_dist, evt.measured_value);
        assert_eq!(evt.threshold, 5.0);
    }

    #[test]
    fn scenario_1_ttc_alert_frame() {
        let rules = load_rules(PORT_SAFETY_RULES).unwrap();
        let step = 11.0_f32 / 14.0;
        let worker = entity("W-01", 0.0, 0.0, 0.0, 0.0);

        let first_ttc_frame = (0..15).find(|&n| {
            let x = 12.0 - n as f32 * step;
            let vehicle = entity("FL-01", x, 0.0, -step, 0.0);
            evaluate(&rules, &[vehicle, worker.clone()], n as u64 * 1000)
                .iter().any(|e| e.rule_id == "TTC_ALERT")
        });

        assert_eq!(first_ttc_frame, Some(13),
            "TTC_ALERT must first fire at frame 13 (TTC = 25/11 ≈ 2.273 s)");

        // Verify TTC measured_value at frame 13: distance=25/14, speed=11/14 → TTC=25/11
        let x13 = 12.0_f32 - 13.0 * step;
        let vehicle13 = entity("FL-01", x13, 0.0, -step, 0.0);
        let events = evaluate(&rules, &[vehicle13, worker.clone()], 13000);
        let evt = events.iter().find(|e| e.rule_id == "TTC_ALERT").unwrap();
        let expected_ttc = 25.0_f32 / 11.0;
        assert!((evt.measured_value - expected_ttc).abs() < 1e-3,
            "TTC at frame 13 must be 25/11 ≈ {:.4} s, got {:.4}",
            expected_ttc, evt.measured_value);
        assert_eq!(evt.threshold, 3.0);
    }

    // Scenario 2 — TTC trigger (clean numbers)
    //
    // Vehicle at (8.0, 0) closing at 4.0 m/s toward worker at origin.
    //   TTC = distance / closing_speed = 8.0 / 4.0 = 2.0 s < threshold 3.0 s  → TTC_ALERT fires
    //   distance = 8.0 m > threshold 5.0 m                                     → PROXIMITY_ALERT silent
    #[test]
    fn scenario_2_ttc_clean_numbers() {
        let rules = load_rules(PORT_SAFETY_RULES).unwrap();
        let vehicle = entity("FL-01", 8.0, 0.0, -4.0, 0.0);
        let worker  = entity("W-01",  0.0, 0.0,  0.0, 0.0);
        let events  = evaluate(&rules, &[vehicle, worker], 0);

        assert!(events.iter().any(|e| e.rule_id == "TTC_ALERT"),
            "TTC_ALERT must fire: TTC = 8.0/4.0 = 2.0 s < 3.0 s");
        assert!(!events.iter().any(|e| e.rule_id == "PROXIMITY_ALERT"),
            "PROXIMITY_ALERT must be silent: distance = 8.0 m > 5.0 m");

        let evt = events.iter().find(|e| e.rule_id == "TTC_ALERT").unwrap();
        assert!((evt.measured_value - 2.0).abs() < 1e-3,
            "measured TTC must be 2.0 s, got {:.4}", evt.measured_value);
        assert_eq!(evt.threshold, 3.0);
    }

    // Scenario 3 — Safe pass, zero false positives
    //
    // Vehicle passes at 6.0 m lateral clearance, moving parallel to worker (no closing velocity).
    //   distance = 6.0 m > threshold 5.0 m  → PROXIMITY_ALERT silent
    //   closing speed = 0                   → TTC = ∞              → TTC_ALERT silent
    // Over 10 frames the vehicle moves further away: zero events total.
    #[test]
    fn scenario_3_safe_pass_no_false_positives() {
        let rules = load_rules(PORT_SAFETY_RULES).unwrap();
        let worker = entity("W-01", 0.0, 0.0, 0.0, 0.0);
        let mut total_events = 0usize;

        for frame in 0..10_u64 {
            // Vehicle moves along x-axis at y=6.0 — always 6 m from worker at origin
            let x = frame as f32 * 1.0;
            let vehicle = entity("FL-01", x, 6.0, 1.0, 0.0);
            total_events += evaluate(&rules, &[vehicle, worker.clone()], frame * 1000).len();
        }

        assert_eq!(total_events, 0,
            "safe pass at 6 m lateral clearance must produce zero events across 10 frames");
    }

    // Scenario 4 — Zone boundary precision
    //
    // Zone: [[300,200],[600,200],[600,500],[300,500]] (300 m × 300 m rectangle)
    // Vessel at (299, 350): x < 300 → outside → no alert
    // Vessel at (301, 350): x > 300, 200 < y < 500 → inside → ZONE_ENTRY fires
    #[test]
    fn scenario_4_zone_boundary_precision() {
        let rules = load_rules(ZONE_RULES).unwrap();

        let outside = Entity {
            id: "V-001".into(), class: EntityClass::Vessel,
            position: Vec2::new(299.0, 350.0), velocity: Vec2::new(0.0, 0.0), timestamp_ms: 0,
        };
        let inside = Entity {
            id: "V-001".into(), class: EntityClass::Vessel,
            position: Vec2::new(301.0, 350.0), velocity: Vec2::new(0.0, 0.0), timestamp_ms: 0,
        };

        assert!(evaluate(&rules, &[outside], 0).is_empty(),
            "vessel at x=299 (outside zone x∈[300,600]) must produce no alert");
        assert!(evaluate(&rules, &[inside], 1000)
            .iter().any(|e| e.rule_id == "ZONE_ENTRY"),
            "vessel at x=301 (inside zone) must fire ZONE_ENTRY");
    }

    // Scenario 5 — Zone exit: events on inside frames only
    //
    // Vessel trajectory (6 frames, Δt=30 s, speed=2 m/s → 60 m/frame):
    //   f0 t=0:      x=250 → outside  (x < 300)
    //   f1 t=30000:  x=310 → inside
    //   f2 t=60000:  x=400 → inside
    //   f3 t=90000:  x=500 → inside
    //   f4 t=120000: x=590 → inside
    //   f5 t=150000: x=650 → outside  (x > 600)
    //
    // Expected: ZONE_ENTRY fires at frames 1-4 only → exactly 4 events.
    // ── Unity demo pipeline tests ─────────────────────────────────────────────
    //
    // These tests validate the exact scenario the Unity scene exercises:
    // - Entity ID "V-001", class Vessel (as ClarusUdpExporter sends)
    // - sg-maritime-security zone: [[300,200],[600,200],[600,500],[300,500]]
    // - Vessel path: x = 0 → 700, y = 350 at 2 m/s (VesselPath.cs defaults)
    // - RESTRICTED_ZONE_APPROACH fires when x ∈ [300,600], y ∈ [200,500]

    const SG_MARITIME_RULES: &str = r#"[{
        "rule_id": "RESTRICTED_ZONE_APPROACH",
        "condition": "zone_member",
        "severity": "HIGH",
        "regulation": "Singapore Infrastructure Protection Act (Cap. 136A) §18",
        "zone": [[300,200],[600,200],[600,500],[300,500]]
    }]"#;

    #[test]
    fn unity_demo_vessel_outside_zone_no_alert() {
        // V-001 at x=150, y=350 — approaching but not yet inside zone
        let rules = load_rules(SG_MARITIME_RULES).unwrap();
        let vessel = Entity {
            id: "V-001".into(), class: EntityClass::Vessel,
            position: Vec2::new(150.0, 350.0), velocity: Vec2::new(2.0, 0.0), timestamp_ms: 75_000,
        };
        let events = evaluate(&rules, &[vessel], 75_000);
        assert!(events.is_empty(), "no alert before zone entry at x=150");
    }

    #[test]
    fn unity_demo_vessel_enters_zone_alert_fires() {
        // V-001 at x=350, y=350 — inside zone (t ≈ 175 s at 2 m/s from x=0)
        let rules = load_rules(SG_MARITIME_RULES).unwrap();
        let vessel = Entity {
            id: "V-001".into(), class: EntityClass::Vessel,
            position: Vec2::new(350.0, 350.0), velocity: Vec2::new(2.0, 0.0), timestamp_ms: 175_000,
        };
        let events = evaluate(&rules, &[vessel], 175_000);
        let ev = events.iter().find(|e| e.rule_id == "RESTRICTED_ZONE_APPROACH");
        assert!(ev.is_some(), "RESTRICTED_ZONE_APPROACH must fire at x=350");
        assert_eq!(ev.unwrap().entity_ids, vec!["V-001"]);
    }

    #[test]
    fn unity_demo_zone_entry_at_x300_boundary() {
        // Exactly the zone boundary — x=299 silent, x=301 fires
        let rules = load_rules(SG_MARITIME_RULES).unwrap();
        for (x, should_fire) in [(299.0f32, false), (301.0f32, true)] {
            let vessel = Entity {
                id: "V-001".into(), class: EntityClass::Vessel,
                position: Vec2::new(x, 350.0), velocity: Vec2::new(2.0, 0.0), timestamp_ms: 0,
            };
            let fired = evaluate(&rules, &[vessel], 0)
                .iter().any(|e| e.rule_id == "RESTRICTED_ZONE_APPROACH");
            assert_eq!(fired, should_fire, "x={x}: fired={fired}, expected={should_fire}");
        }
    }

    #[test]
    fn scenario_5_zone_exit_events_only_on_inside_frames() {
        let rules = load_rules(ZONE_RULES).unwrap();
        let x_positions: &[(u64, f32, bool)] = &[
            (0,      250.0, false),
            (30_000, 310.0, true),
            (60_000, 400.0, true),
            (90_000, 500.0, true),
            (120_000,590.0, true),
            (150_000,650.0, false),
        ];

        for &(ts, x, should_fire) in x_positions {
            let vessel = Entity {
                id: "V-001".into(), class: EntityClass::Vessel,
                position: Vec2::new(x, 350.0), velocity: Vec2::new(2.0, 0.0), timestamp_ms: ts,
            };
            let fired = evaluate(&rules, &[vessel], ts)
                .iter().any(|e| e.rule_id == "ZONE_ENTRY");
            assert_eq!(fired, should_fire,
                "at x={x} (t={ts}ms): ZONE_ENTRY fired={fired}, expected={should_fire}");
        }
    }
}
