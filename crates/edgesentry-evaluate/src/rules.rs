use serde::Deserialize;

use edgesentry_types::{Entity, EntityClass, SensorReading, SourceType, Vec2};
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
    /// Fire when a named sensor value on any entity exceeds the threshold.
    /// Parsed from: "sensor_value:NAME > THRESHOLD"  (only `>` supported)
    SensorValueGt { name: String, threshold: f32 },
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub rule_id: String,
    pub condition: Condition,
    pub severity: Severity,
    pub regulation: String,
}

/// Evidentiary quality of a RiskEvent. Sealed into the audit chain alongside every event.
/// Determined by the sensor source type and its detection confidence.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvidenceQuality {
    /// Full actuarial weight — admissible as standalone evidence.
    #[default]
    Certified,
    /// Reduced actuarial weight — requires corroboration before use in a claim.
    Degraded,
    /// Not admissible as standalone evidence.
    Rejected,
    /// Evidence quality concept does not apply (simulation / test data).
    NotApplicable,
}

impl EvidenceQuality {
    /// Derive quality from a raw confidence score (0.0–1.0).
    pub fn from_confidence(confidence_cv: f32) -> Self {
        if confidence_cv >= 0.8 {
            Self::Certified
        } else if confidence_cv >= 0.5 {
            Self::Degraded
        } else {
            Self::Rejected
        }
    }

    /// Derive quality from the sensor reading attached to an entity.
    /// Rules:
    /// - Unknown source (`None`) → `Degraded` (cannot certify without provenance)
    /// - CV / Radar with confidence → score-based
    /// - CV without confidence → `Degraded`
    /// - AIS, LiDAR, UWB, PointSensor → `Certified` (authoritative primary data)
    /// - Simulation → `NotApplicable`
    pub fn from_reading(reading: Option<&SensorReading>) -> Self {
        match reading {
            None => Self::Degraded,
            Some(r) => match (&r.source_type, r.detection_confidence) {
                (SourceType::ComputerVision, Some(c)) => Self::from_confidence(c),
                (SourceType::ComputerVision, None)    => Self::Degraded,
                (SourceType::Radar, Some(c))          => Self::from_confidence(c),
                (SourceType::Radar, None)             => Self::Degraded,
                (SourceType::Ais, _)                  => Self::Certified,
                (SourceType::Lidar, _)                => Self::Certified,
                (SourceType::Uwb, _)                  => Self::Certified,
                (SourceType::PointSensor, _)          => Self::Certified,
                (SourceType::Simulation, _)           => Self::NotApplicable,
            },
        }
    }
}

fn default_confidence() -> f32 {
    1.0
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
    /// Minimum detection confidence across all involved entities.
    /// Derived from `SensorReading.detection_confidence`; defaults to 1.0 for
    /// sources where detection confidence is not applicable (AIS, LiDAR, etc.).
    #[serde(default = "default_confidence")]
    pub confidence_cv: f32,
    /// Evidentiary quality derived from the sensor reading of involved entities.
    #[serde(default)]
    pub evidence_quality: EvidenceQuality,
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
    if let Some(rest) = s.strip_prefix("sensor_value:") {
        // Format: "sensor_value:{name} > {threshold}"
        let parts: Vec<&str> = rest.splitn(2, " > ").collect();
        if parts.len() != 2 {
            return Err(format!("malformed sensor_value condition, expected 'sensor_value:NAME > THRESHOLD', got '{s}'"));
        }
        let name = parts[0].trim().to_string();
        if name.is_empty() {
            return Err(format!("sensor_value condition missing name in '{s}'"));
        }
        let threshold: f32 = parts[1].trim().parse()
            .map_err(|_| format!("invalid threshold in sensor_value condition '{s}'"))?;
        return Ok(Condition::SensorValueGt { name, threshold });
    }
    Err(format!("unknown condition expression: '{s}'"))
}

// ── Evaluation ────────────────────────────────────────────────────────────────

/// Extract a scalar confidence_cv from an entity's sensor reading.
/// For sources without a detection confidence (AIS, LiDAR, etc.) returns 1.0
/// so that the numeric field in RiskEvent is well-defined.
fn entity_confidence(e: &Entity) -> f32 {
    e.sensor.as_ref()
        .and_then(|s| s.detection_confidence)
        .unwrap_or(1.0)
}

fn pair_confidence(a: &Entity, b: &Entity) -> f32 {
    entity_confidence(a).min(entity_confidence(b))
}

fn entity_quality(e: &Entity) -> EvidenceQuality {
    if let Some(score) = e.computed_confidence {
        EvidenceQuality::from_confidence(score)
    } else {
        EvidenceQuality::from_reading(e.sensor.as_ref())
    }
}

fn pair_quality(a: &Entity, b: &Entity) -> EvidenceQuality {
    let qa = entity_quality(a);
    let qb = entity_quality(b);
    // Take the worse of the two
    use EvidenceQuality::*;
    match (&qa, &qb) {
        (NotApplicable, _) | (_, NotApplicable) => NotApplicable,
        (Rejected, _) | (_, Rejected) => Rejected,
        (Degraded, _) | (_, Degraded) => Degraded,
        _ => Certified,
    }
}

fn make_event(
    rule: &Rule,
    entity_ids: Vec<String>,
    measured_value: f32,
    threshold: f32,
    timestamp_ms: u64,
    confidence_cv: f32,
    evidence_quality: EvidenceQuality,
) -> RiskEvent {
    RiskEvent {
        rule_id: rule.rule_id.clone(),
        severity: rule.severity.clone(),
        regulation: rule.regulation.clone(),
        entity_ids,
        measured_value,
        threshold,
        timestamp_ms,
        confidence_cv,
        evidence_quality,
    }
}

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
                            let cv = pair_confidence(&entities[i], &entities[j]);
                            let eq = pair_quality(&entities[i], &entities[j]);
                            events.push(make_event(rule,
                                vec![entities[i].id.clone(), entities[j].id.clone()],
                                dist, *threshold, timestamp_ms, cv, eq));
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
                            let cv = pair_confidence(&entities[i], &entities[j]);
                            let eq = pair_quality(&entities[i], &entities[j]);
                            events.push(make_event(rule,
                                vec![entities[i].id.clone(), entities[j].id.clone()],
                                ttc, *threshold, timestamp_ms, cv, eq));
                        }
                    }
                }
            }
            Condition::ZoneMember(polygon) => {
                for entity in entities {
                    if zone_membership(entity.position.clone(), polygon) {
                        let cv = entity_confidence(entity);
                        let eq = entity_quality(entity);
                        events.push(make_event(rule,
                            vec![entity.id.clone()],
                            1.0, 0.0, timestamp_ms, cv, eq));
                    }
                }
            }
            Condition::AisGapGt(threshold) => {
                for entity in entities {
                    if entity.class == EntityClass::AisGap && entity.velocity.x > *threshold {
                        let cv = entity_confidence(entity);
                        let eq = entity_quality(entity);
                        events.push(make_event(rule,
                            vec![entity.id.clone()],
                            entity.velocity.x, *threshold, timestamp_ms, cv, eq));
                    }
                }
            }
            Condition::SensorValueGt { name, threshold } => {
                for entity in entities {
                    if let Some(ref sv) = entity.sensor_values {
                        if let Some(&val) = sv.get(name.as_str()) {
                            if val as f32 > *threshold {
                                events.push(make_event(rule,
                                    vec![entity.id.clone()],
                                    val as f32, *threshold, timestamp_ms,
                                    1.0, EvidenceQuality::Certified));
                            }
                        }
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
    use edgesentry_types::{Entity, EntityClass, SensorReading, Vec2};

    fn entity(id: &str, x: f32, y: f32, vx: f32, vy: f32) -> Entity {
        Entity {
            id: id.into(),
            class: EntityClass::Forklift,
            position: Vec2::new(x, y),
            position_z: None,
            velocity: Vec2::new(vx, vy),
            velocity_z: None,
            timestamp_ms: 0,
            sensor: None,
            computed_confidence: None,
            sensor_values: None,
        }
    }

    fn entity_with_confidence(id: &str, x: f32, y: f32, vx: f32, vy: f32, confidence: f32) -> Entity {
        Entity { sensor: Some(SensorReading::cv(confidence)), ..entity(id, x, y, vx, vy) }
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
            position_z: None,
            velocity: Vec2::new(gap_s, 0.0),
            velocity_z: None,
            timestamp_ms: 0,
            sensor: Some(SensorReading::ais()),
            computed_confidence: None,
            sensor_values: None,
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
    fn sg_maritime_nmea_fixture_triggers_restricted_zone_approach() {
        use edgesentry_compute::latlon_to_local;
        use edgesentry_ingest::ais_nmea::{load_port_ref, parse_vdm};

        let nmea = include_str!("../../edgesentry-ingest/fixtures/sg_maritime_ais.nmea");
        let params = include_str!(
            "../../edgesentry-profile/fixtures/sg-maritime-security/params.toml"
        );
        let rules_json = include_str!(
            "../../edgesentry-profile/fixtures/sg-maritime-security/rules.json"
        );
        let port_ref = load_port_ref(params).expect("sg-maritime-security params.toml");
        let rules = load_rules(rules_json).expect("sg-maritime-security rules.json");

        let mut zone_fired = false;
        for (i, line) in nmea.lines().enumerate() {
            let line = line.trim();
            if !line.starts_with("!AIVDM") {
                continue;
            }
            let report = parse_vdm(line).expect("fixture NMEA must parse");
            let (x, y) = latlon_to_local(
                report.lat_deg,
                report.lon_deg,
                port_ref.lat_deg,
                port_ref.lon_deg,
            );
            let vessel = Entity {
                id: report.mmsi.to_string(),
                class: EntityClass::Vessel,
                position: Vec2::new(x, y),
                velocity: Vec2::new(0.0, 0.0),
                timestamp_ms: (i as u64) * 30_000,
                sensor: None,
                position_z: None,
                velocity_z: None,
                computed_confidence: None,
                sensor_values: None,
            };
            let events = evaluate(&rules, &[vessel], (i as u64) * 30_000);
            if events
                .iter()
                .any(|e| e.rule_id == "RESTRICTED_ZONE_APPROACH")
            {
                zone_fired = true;
            }
        }
        assert!(
            zone_fired,
            "vessel track in sg_maritime_ais.nmea must enter restricted zone"
        );
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
            sensor: None,
            position_z: None, velocity_z: None, computed_confidence: None, sensor_values: None,
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
            sensor: None,
            position_z: None, velocity_z: None, computed_confidence: None, sensor_values: None,
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
            position: Vec2::new(299.0, 350.0), velocity: Vec2::new(0.0, 0.0), timestamp_ms: 0, sensor: None, position_z: None, velocity_z: None, computed_confidence: None, sensor_values: None,
        };
        let inside = Entity {
            id: "V-001".into(), class: EntityClass::Vessel,
            position: Vec2::new(301.0, 350.0), velocity: Vec2::new(0.0, 0.0), timestamp_ms: 0, sensor: None, position_z: None, velocity_z: None, computed_confidence: None, sensor_values: None,
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
            position: Vec2::new(150.0, 350.0), velocity: Vec2::new(2.0, 0.0), timestamp_ms: 75_000, sensor: None, position_z: None, velocity_z: None, computed_confidence: None, sensor_values: None,
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
            position: Vec2::new(350.0, 350.0), velocity: Vec2::new(2.0, 0.0), timestamp_ms: 175_000, sensor: None, position_z: None, velocity_z: None, computed_confidence: None, sensor_values: None,
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
                position: Vec2::new(x, 350.0), velocity: Vec2::new(2.0, 0.0), timestamp_ms: 0, sensor: None, position_z: None, velocity_z: None, computed_confidence: None, sensor_values: None,
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
                position: Vec2::new(x, 350.0), velocity: Vec2::new(2.0, 0.0), timestamp_ms: ts, sensor: None, position_z: None, velocity_z: None, computed_confidence: None, sensor_values: None,
            };
            let fired = evaluate(&rules, &[vessel], ts)
                .iter().any(|e| e.rule_id == "ZONE_ENTRY");
            assert_eq!(fired, should_fire,
                "at x={x} (t={ts}ms): ZONE_ENTRY fired={fired}, expected={should_fire}");
        }
    }

    // ── Evidence quality tests ────────────────────────────────────────────────

    #[test]
    fn evidence_quality_certified_at_08() {
        assert_eq!(EvidenceQuality::from_confidence(0.8), EvidenceQuality::Certified);
        assert_eq!(EvidenceQuality::from_confidence(1.0), EvidenceQuality::Certified);
        assert_eq!(EvidenceQuality::from_confidence(0.95), EvidenceQuality::Certified);
    }

    #[test]
    fn evidence_quality_degraded_between_05_and_08() {
        assert_eq!(EvidenceQuality::from_confidence(0.5), EvidenceQuality::Degraded);
        assert_eq!(EvidenceQuality::from_confidence(0.7), EvidenceQuality::Degraded);
        assert_eq!(EvidenceQuality::from_confidence(0.79), EvidenceQuality::Degraded);
    }

    #[test]
    fn evidence_quality_rejected_below_05() {
        assert_eq!(EvidenceQuality::from_confidence(0.0), EvidenceQuality::Rejected);
        assert_eq!(EvidenceQuality::from_confidence(0.49), EvidenceQuality::Rejected);
    }

    #[test]
    fn evaluate_unknown_sensor_confidence_cv_is_one_quality_is_degraded() {
        // Entity with no SensorReading: confidence_cv defaults to 1.0 (numeric field
        // is well-defined) but evidence_quality is Degraded (unknown provenance).
        let rules = load_rules(DEMO_RULES_JSON).unwrap();
        let entities = vec![
            entity("FL-01", 0.0, 0.0, 1.4, 0.0),
            entity("W-03", 3.2, 0.0, 0.0, 0.0),
        ];
        let events = evaluate(&rules, &entities, 0);
        let evt = events.iter().find(|e| e.rule_id == "PROXIMITY_ALERT").unwrap();
        assert!((evt.confidence_cv - 1.0).abs() < 1e-6);
        assert_eq!(evt.evidence_quality, EvidenceQuality::Degraded);
    }

    #[test]
    fn evaluate_low_confidence_entity_produces_rejected_event() {
        let rules = load_rules(DEMO_RULES_JSON).unwrap();
        let entities = vec![
            entity_with_confidence("FL-01", 0.0, 0.0, 1.4, 0.0, 0.3),
            entity("W-03", 3.2, 0.0, 0.0, 0.0),
        ];
        let events = evaluate(&rules, &entities, 0);
        let evt = events.iter().find(|e| e.rule_id == "PROXIMITY_ALERT").unwrap();
        assert!((evt.confidence_cv - 0.3).abs() < 1e-6);
        assert_eq!(evt.evidence_quality, EvidenceQuality::Rejected);
    }

    #[test]
    fn evaluate_pair_confidence_uses_minimum() {
        // Entity A: 0.9, Entity B: 0.6 → pair confidence = 0.6 → Degraded
        let rules = load_rules(DEMO_RULES_JSON).unwrap();
        let entities = vec![
            entity_with_confidence("FL-01", 0.0, 0.0, 1.4, 0.0, 0.9),
            entity_with_confidence("W-03", 3.2, 0.0, 0.0, 0.0, 0.6),
        ];
        let events = evaluate(&rules, &entities, 0);
        let evt = events.iter().find(|e| e.rule_id == "PROXIMITY_ALERT").unwrap();
        assert!((evt.confidence_cv - 0.6).abs() < 1e-6);
        assert_eq!(evt.evidence_quality, EvidenceQuality::Degraded);
    }

    #[test]
    fn evaluate_zone_confidence_propagates_to_event() {
        let rules = load_rules(DEMO_RULES_JSON).unwrap();
        let entities = vec![entity_with_confidence("V-01", 5.0, 5.0, 0.0, 0.0, 0.55)];
        let events = evaluate(&rules, &entities, 0);
        let evt = events.iter().find(|e| e.rule_id == "EXCLUSION_ZONE_BREACH").unwrap();
        assert!((evt.confidence_cv - 0.55).abs() < 1e-6);
        assert_eq!(evt.evidence_quality, EvidenceQuality::Degraded);
    }

    // ── serde backward-compatibility tests ───────────────────────────────────

    #[test]
    fn risk_event_deserializes_without_confidence_fields() {
        // JSON produced before confidence_cv / evidence_quality were added
        // (e.g. Tauri demo app). Must deserialize with defaults: cv=1.0, Certified.
        let json = r#"{
            "rule_id": "PROXIMITY_ALERT",
            "severity": "HIGH",
            "regulation": "§3.1",
            "entity_ids": ["FL-01", "W-03"],
            "measured_value": 3.2,
            "threshold": 5.0,
            "timestamp_ms": 1000
        }"#;
        let ev: RiskEvent = serde_json::from_str(json).expect("should deserialize without new fields");
        assert!((ev.confidence_cv - 1.0).abs() < 1e-6, "default confidence_cv should be 1.0");
        assert_eq!(ev.evidence_quality, EvidenceQuality::Certified, "default evidence_quality should be Certified");
    }

    #[test]
    fn risk_event_deserializes_with_confidence_fields() {
        // JSON produced by current code — fields present, must round-trip correctly.
        let json = r#"{
            "rule_id": "TTC_ALERT",
            "severity": "HIGH",
            "regulation": "§3.2",
            "entity_ids": ["FL-01", "W-03"],
            "measured_value": 2.1,
            "threshold": 3.0,
            "timestamp_ms": 2000,
            "confidence_cv": 0.62,
            "evidence_quality": "DEGRADED"
        }"#;
        let ev: RiskEvent = serde_json::from_str(json).expect("should deserialize with new fields");
        assert!((ev.confidence_cv - 0.62).abs() < 1e-4);
        assert_eq!(ev.evidence_quality, EvidenceQuality::Degraded);
    }

    #[test]
    fn evidence_quality_default_is_certified() {
        assert_eq!(EvidenceQuality::default(), EvidenceQuality::Certified);
    }

    // ── EvidenceQuality::from_reading — all source types ─────────────────────

    #[test]
    fn from_reading_none_is_degraded() {
        assert_eq!(EvidenceQuality::from_reading(None), EvidenceQuality::Degraded);
    }

    #[test]
    fn from_reading_cv_with_high_confidence_is_certified() {
        let r = SensorReading::cv(0.9);
        assert_eq!(EvidenceQuality::from_reading(Some(&r)), EvidenceQuality::Certified);
    }

    #[test]
    fn from_reading_cv_with_mid_confidence_is_degraded() {
        let r = SensorReading::cv(0.65);
        assert_eq!(EvidenceQuality::from_reading(Some(&r)), EvidenceQuality::Degraded);
    }

    #[test]
    fn from_reading_cv_with_low_confidence_is_rejected() {
        let r = SensorReading::cv(0.3);
        assert_eq!(EvidenceQuality::from_reading(Some(&r)), EvidenceQuality::Rejected);
    }

    #[test]
    fn from_reading_cv_without_confidence_is_degraded() {
        let r = SensorReading { source_type: SourceType::ComputerVision, dimensions: 2, detection_confidence: None, position_stddev_m: None, position_stddev_z_m: None };
        assert_eq!(EvidenceQuality::from_reading(Some(&r)), EvidenceQuality::Degraded);
    }

    #[test]
    fn from_reading_ais_is_certified() {
        assert_eq!(EvidenceQuality::from_reading(Some(&SensorReading::ais())), EvidenceQuality::Certified);
    }

    #[test]
    fn from_reading_lidar_is_certified() {
        let r = SensorReading { source_type: SourceType::Lidar, dimensions: 2, detection_confidence: None, position_stddev_m: None, position_stddev_z_m: None };
        assert_eq!(EvidenceQuality::from_reading(Some(&r)), EvidenceQuality::Certified);
    }

    #[test]
    fn from_reading_uwb_is_certified() {
        let r = SensorReading { source_type: SourceType::Uwb, dimensions: 2, detection_confidence: None, position_stddev_m: None, position_stddev_z_m: None };
        assert_eq!(EvidenceQuality::from_reading(Some(&r)), EvidenceQuality::Certified);
    }

    #[test]
    fn from_reading_point_sensor_is_certified() {
        let r = SensorReading { source_type: SourceType::PointSensor, dimensions: 1, detection_confidence: None, position_stddev_m: None, position_stddev_z_m: None };
        assert_eq!(EvidenceQuality::from_reading(Some(&r)), EvidenceQuality::Certified);
    }

    #[test]
    fn from_reading_radar_with_score_follows_threshold() {
        let high = SensorReading { source_type: SourceType::Radar, dimensions: 2, detection_confidence: Some(0.85), position_stddev_m: None, position_stddev_z_m: None };
        let low  = SensorReading { source_type: SourceType::Radar, dimensions: 2, detection_confidence: Some(0.3),  position_stddev_m: None, position_stddev_z_m: None };
        assert_eq!(EvidenceQuality::from_reading(Some(&high)), EvidenceQuality::Certified);
        assert_eq!(EvidenceQuality::from_reading(Some(&low)),  EvidenceQuality::Rejected);
    }

    #[test]
    fn from_reading_radar_without_confidence_is_degraded() {
        let r = SensorReading { source_type: SourceType::Radar, dimensions: 2, detection_confidence: None, position_stddev_m: None, position_stddev_z_m: None };
        assert_eq!(EvidenceQuality::from_reading(Some(&r)), EvidenceQuality::Degraded);
    }

    #[test]
    fn from_reading_simulation_is_not_applicable() {
        assert_eq!(EvidenceQuality::from_reading(Some(&SensorReading::simulation())), EvidenceQuality::NotApplicable);
    }

    // ── pair_quality worst-of-two logic ───────────────────────────────────────

    #[test]
    fn pair_quality_both_certified_is_certified() {
        let a = Entity { sensor: Some(SensorReading::ais()), ..entity("A", 0.0, 0.0, 0.0, 0.0) };
        let b = Entity { sensor: Some(SensorReading::ais()), ..entity("B", 1.0, 0.0, 0.0, 0.0) };
        assert_eq!(pair_quality(&a, &b), EvidenceQuality::Certified);
    }

    #[test]
    fn pair_quality_one_rejected_is_rejected() {
        let a = Entity { sensor: Some(SensorReading::cv(0.9)), ..entity("A", 0.0, 0.0, 0.0, 0.0) };
        let b = Entity { sensor: Some(SensorReading::cv(0.2)), ..entity("B", 1.0, 0.0, 0.0, 0.0) };
        assert_eq!(pair_quality(&a, &b), EvidenceQuality::Rejected);
    }

    #[test]
    fn pair_quality_simulation_is_not_applicable() {
        let a = Entity { sensor: Some(SensorReading::simulation()), ..entity("A", 0.0, 0.0, 0.0, 0.0) };
        let b = Entity { sensor: Some(SensorReading::simulation()), ..entity("B", 1.0, 0.0, 0.0, 0.0) };
        assert_eq!(pair_quality(&a, &b), EvidenceQuality::NotApplicable);
    }

    // ── AIS gap event carries Certified quality ───────────────────────────────
    // The gap observation is derived from the AIS system (authoritative) so it
    // carries Certified evidence quality despite being a synthetic entity.

    #[test]
    fn evaluate_ais_gap_event_quality_is_certified() {
        let rules = load_rules(AIS_RULES_JSON).unwrap();
        let entities = vec![ais_gap_entity("563012345", 600.0)];
        let events = evaluate(&rules, &entities, 0);
        let evt = events.iter().find(|e| e.rule_id == "AIS_TRACK_GAP").unwrap();
        assert_eq!(evt.evidence_quality, EvidenceQuality::Certified);
    }

    // ── AIS vessel event carries Certified quality ────────────────────────────

    #[test]
    fn evaluate_ais_vessel_in_zone_is_certified() {
        let rules = load_rules(SG_MARITIME_RULES).unwrap();
        let vessel = Entity {
            sensor: Some(SensorReading::ais()),
            ..Entity {
                id: "V-001".into(), class: EntityClass::Vessel,
                position: Vec2::new(350.0, 350.0), velocity: Vec2::new(2.0, 0.0),
                timestamp_ms: 0, sensor: None, position_z: None, velocity_z: None, computed_confidence: None, sensor_values: None,
            }
        };
        let events = evaluate(&rules, &[vessel], 0);
        let evt = events.iter().find(|e| e.rule_id == "RESTRICTED_ZONE_APPROACH").unwrap();
        assert_eq!(evt.evidence_quality, EvidenceQuality::Certified);
        assert!((evt.confidence_cv - 1.0).abs() < 1e-6);
    }

    // ── SensorValueGt ─────────────────────────────────────────────────────────

    const BCA_RULES_JSON: &str = r#"[
        {"rule_id":"EUI_PLATINUM_EXCEEDED","condition":"sensor_value:eui_kwh_m2 > 115.0","severity":"HIGH","regulation":"BCA Green Mark 2021 Section 4.1"}
    ]"#;

    fn bca_entity(id: &str, eui: f64) -> Entity {
        let mut vals = std::collections::HashMap::new();
        vals.insert("eui_kwh_m2".to_string(), eui);
        Entity { sensor_values: Some(vals), ..entity(id, 0.0, 0.0, 0.0, 0.0) }
    }

    #[test]
    fn parse_condition_sensor_value_gt() {
        let rules = load_rules(BCA_RULES_JSON).unwrap();
        assert_eq!(rules.len(), 1);
        match &rules[0].condition {
            Condition::SensorValueGt { name, threshold } => {
                assert_eq!(name, "eui_kwh_m2");
                assert!((*threshold - 115.0).abs() < 1e-5);
            }
            other => panic!("expected SensorValueGt, got {other:?}"),
        }
    }

    #[test]
    fn sensor_value_gt_fires_when_exceeded() {
        let rules = load_rules(BCA_RULES_JSON).unwrap();
        let entities = vec![bca_entity("OUTLET-SENSORS", 122.5)];
        let events = evaluate(&rules, &entities, 1000);
        let evt = events.iter().find(|e| e.rule_id == "EUI_PLATINUM_EXCEEDED")
            .expect("EUI_PLATINUM_EXCEEDED must fire when eui=122.5 > 115.0");
        assert!((evt.measured_value - 122.5).abs() < 0.01);
        assert!((evt.threshold - 115.0).abs() < 1e-5);
        assert_eq!(evt.evidence_quality, EvidenceQuality::Certified);
        assert!((evt.confidence_cv - 1.0).abs() < 1e-6);
        assert_eq!(evt.entity_ids, vec!["OUTLET-SENSORS"]);
    }

    #[test]
    fn sensor_value_gt_no_fire_when_below() {
        let rules = load_rules(BCA_RULES_JSON).unwrap();
        let entities = vec![bca_entity("OUTLET-SENSORS", 108.0)];
        let events = evaluate(&rules, &entities, 0);
        assert!(!events.iter().any(|e| e.rule_id == "EUI_PLATINUM_EXCEEDED"),
            "EUI_PLATINUM_EXCEEDED must NOT fire when eui=108.0 < 115.0");
    }

    #[test]
    fn sensor_value_gt_no_fire_when_key_absent() {
        let rules = load_rules(BCA_RULES_JSON).unwrap();
        // entity has no sensor_values at all
        let entities = vec![entity("OUTLET-SENSORS", 0.0, 0.0, 0.0, 0.0)];
        let events = evaluate(&rules, &entities, 0);
        assert!(!events.iter().any(|e| e.rule_id == "EUI_PLATINUM_EXCEEDED"),
            "EUI_PLATINUM_EXCEEDED must NOT fire when sensor_values is None");
    }
}
