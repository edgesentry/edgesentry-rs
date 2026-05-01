use std::collections::HashSet;
use std::path::Path;
use std::fs;

// ── Public API ────────────────────────────────────────────────────────────────

/// The result of validating a profile directory.
#[derive(Debug, Default)]
pub struct ValidationReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Validate a profile directory and return a `ValidationReport`.
///
/// Checks:
/// - rules.json exists and is valid JSON
/// - Top-level is a non-empty array
/// - Each rule has rule_id (SCREAMING_SNAKE_CASE, unique, no spaces)
/// - Valid condition syntax
/// - Valid severity values
/// - Non-empty regulation
/// - zone polygons have ≥3 vertices
/// - KB file coverage (missing/orphaned KB files)
pub fn validate_profile(profile_dir: &Path) -> ValidationReport {
    let mut report = ValidationReport::default();
    let rule_ids = validate_rules_json(profile_dir, &mut report);
    validate_kb(profile_dir, &rule_ids, &mut report);
    report
}

/// Load and parse rules from a profile directory's `rules.json`.
///
/// Returns the parsed `Rule` list, or an error string.
pub fn load_profile(profile_dir: &Path) -> Result<Vec<edgesentry_evaluate::Rule>, String> {
    let rules_path = profile_dir.join("rules.json");
    let content = fs::read_to_string(&rules_path)
        .map_err(|e| format!("cannot read {}: {e}", rules_path.display()))?;
    edgesentry_evaluate::load_rules(&content)
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Returns rule_ids found in rules.json (empty on parse failure).
fn validate_rules_json(profile_dir: &Path, report: &mut ValidationReport) -> Vec<String> {
    let rules_path = profile_dir.join("rules.json");

    // 1. File exists
    if !rules_path.exists() {
        report.errors.push(format!("rules.json not found at {}", rules_path.display()));
        return vec![];
    }

    // 2. Valid JSON
    let content = match fs::read_to_string(&rules_path) {
        Ok(c) => c,
        Err(e) => {
            report.errors.push(format!("cannot read rules.json: {e}"));
            return vec![];
        }
    };
    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            report.errors.push(format!("rules.json is not valid JSON: {e}"));
            return vec![];
        }
    };

    // 3. Top-level is an array
    let rules = match json.as_array() {
        Some(r) => r,
        None => {
            report.errors.push("rules.json must be a JSON array at the top level".to_string());
            return vec![];
        }
    };

    if rules.is_empty() {
        report.errors.push("rules.json contains no rules".to_string());
        return vec![];
    }

    // 4. Validate each rule
    let mut rule_ids: Vec<String> = Vec::new();
    let mut seen_ids: HashSet<String> = HashSet::new();

    for (i, rule) in rules.iter().enumerate() {
        let idx = i + 1;
        validate_rule(rule, idx, &mut rule_ids, &mut seen_ids, report);
    }

    rule_ids
}

fn validate_rule(
    rule: &serde_json::Value,
    idx: usize,
    rule_ids: &mut Vec<String>,
    seen_ids: &mut HashSet<String>,
    report: &mut ValidationReport,
) {
    let obj = match rule.as_object() {
        Some(o) => o,
        None => {
            report.errors.push(format!("Rule {idx}: must be a JSON object"));
            return;
        }
    };

    // rule_id
    let rule_id = match obj.get("rule_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => {
            report.errors.push(format!("Rule {idx}: missing required field \"rule_id\" (string)"));
            return;
        }
    };

    if rule_id.is_empty() {
        report.errors.push(format!("Rule {idx}: \"rule_id\" must not be empty"));
        return;
    }
    if rule_id.contains(' ') {
        report.errors.push(format!(
            "Rule {idx} ({rule_id}): \"rule_id\" must not contain spaces"
        ));
    }
    if rule_id != rule_id.to_uppercase() {
        report.warnings.push(format!(
            "Rule {idx} ({rule_id}): \"rule_id\" should be SCREAMING_SNAKE_CASE"
        ));
    }
    if seen_ids.contains(&rule_id) {
        report.errors.push(format!("Rule {idx}: duplicate rule_id \"{rule_id}\""));
    } else {
        seen_ids.insert(rule_id.clone());
        rule_ids.push(rule_id.clone());
    }

    // condition
    match obj.get("condition").and_then(|v| v.as_str()) {
        None => {
            report.errors.push(format!(
                "Rule {idx} ({rule_id}): missing required field \"condition\" (string)"
            ));
        }
        Some(cond) => validate_condition(cond, obj, &rule_id, idx, report),
    }

    // severity
    match obj.get("severity").and_then(|v| v.as_str()) {
        None => {
            report.errors.push(format!(
                "Rule {idx} ({rule_id}): missing required field \"severity\""
            ));
        }
        Some(sev) => match sev {
            "LOW" | "MEDIUM" | "HIGH" | "CRITICAL" => {}
            _ => {
                report.errors.push(format!(
                    "Rule {idx} ({rule_id}): invalid severity \"{sev}\""
                ));
            }
        },
    }

    // regulation
    match obj.get("regulation").and_then(|v| v.as_str()) {
        None => {
            report.errors.push(format!(
                "Rule {idx} ({rule_id}): missing required field \"regulation\" (string)"
            ));
        }
        Some(reg) if reg.trim().is_empty() => {
            report.errors.push(format!(
                "Rule {idx} ({rule_id}): \"regulation\" must not be empty"
            ));
        }
        Some(_) => {}
    }
}

fn validate_condition(
    cond: &str,
    obj: &serde_json::Map<String, serde_json::Value>,
    rule_id: &str,
    idx: usize,
    report: &mut ValidationReport,
) {
    let cond = cond.trim();

    if let Some(rest) = cond.strip_prefix("distance < ") {
        match rest.trim().parse::<f64>() {
            Ok(n) if n > 0.0 => {}
            Ok(_) => report.errors.push(format!(
                "Rule {idx} ({rule_id}): distance threshold must be > 0"
            )),
            Err(_) => report.errors.push(format!(
                "Rule {idx} ({rule_id}): invalid number in condition \"{cond}\""
            )),
        }
    } else if let Some(rest) = cond.strip_prefix("ttc < ") {
        match rest.trim().parse::<f64>() {
            Ok(n) if n > 0.0 => {}
            Ok(_) => report.errors.push(format!(
                "Rule {idx} ({rule_id}): ttc threshold must be > 0"
            )),
            Err(_) => report.errors.push(format!(
                "Rule {idx} ({rule_id}): invalid number in condition \"{cond}\""
            )),
        }
    } else if cond == "zone_member" {
        match obj.get("zone") {
            None => {
                report.errors.push(format!(
                    "Rule {idx} ({rule_id}): condition \"zone_member\" requires a \"zone\" field"
                ));
            }
            Some(zone) => validate_zone(zone, rule_id, idx, report),
        }
    } else {
        report.errors.push(format!(
            "Rule {idx} ({rule_id}): unknown condition \"{cond}\""
        ));
    }
}

fn validate_zone(
    zone: &serde_json::Value,
    rule_id: &str,
    idx: usize,
    report: &mut ValidationReport,
) {
    let arr = match zone.as_array() {
        Some(a) => a,
        None => {
            report.errors.push(format!(
                "Rule {idx} ({rule_id}): \"zone\" must be an array of [x, y] pairs"
            ));
            return;
        }
    };

    if arr.len() < 3 {
        report.errors.push(format!(
            "Rule {idx} ({rule_id}): \"zone\" polygon must have at least 3 vertices, got {}",
            arr.len()
        ));
        return;
    }

    for (vi, vertex) in arr.iter().enumerate() {
        let pair = match vertex.as_array() {
            Some(p) if p.len() == 2 => p,
            _ => {
                report.errors.push(format!(
                    "Rule {idx} ({rule_id}): zone vertex {vi} must be [x, y]"
                ));
                return;
            }
        };
        for coord in pair {
            if !coord.is_number() {
                report.errors.push(format!(
                    "Rule {idx} ({rule_id}): zone vertex {vi} coordinates must be numbers"
                ));
                return;
            }
        }
    }
}

fn validate_kb(profile_dir: &Path, rule_ids: &[String], report: &mut ValidationReport) {
    let kb_dir = profile_dir.join("kb");

    if !kb_dir.exists() {
        report.warnings.push(
            "kb/ directory not found — LLM explanations will fall back to \"No KB entry\""
                .to_string(),
        );
        return;
    }

    // Collect existing KB files
    let kb_files: HashSet<String> = match fs::read_dir(&kb_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
            .filter_map(|e| e.path().file_stem()?.to_str().map(str::to_string))
            .collect(),
        Err(e) => {
            report.errors.push(format!("cannot read kb/: {e}"));
            return;
        }
    };

    // Each rule should have a KB file
    for rule_id in rule_ids {
        if kb_files.contains(rule_id) {
            let path = kb_dir.join(format!("{rule_id}.md"));
            let content = fs::read_to_string(&path).unwrap_or_default();
            if content.trim().is_empty() {
                report.warnings.push(format!("kb/{rule_id}.md exists but is empty"));
            }
        } else {
            report.warnings.push(format!(
                "kb/{rule_id}.md not found — LLM explanation for {rule_id} will be ungrounded"
            ));
        }
    }

    // Warn about orphaned KB files (no matching rule)
    let rule_set: HashSet<&str> = rule_ids.iter().map(String::as_str).collect();
    for kb_id in &kb_files {
        if !rule_set.contains(kb_id.as_str()) {
            report.warnings.push(format!(
                "kb/{kb_id}.md has no matching rule in rules.json (orphaned)"
            ));
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_profile(dir: &Path, rules: &str, kb: &[(&str, &str)]) {
        fs::write(dir.join("rules.json"), rules).unwrap();
        let kb_dir = dir.join("kb");
        fs::create_dir_all(&kb_dir).unwrap();
        for (name, content) in kb {
            fs::write(kb_dir.join(format!("{name}.md")), content).unwrap();
        }
    }

    fn run(dir: &Path) -> ValidationReport {
        validate_profile(dir)
    }

    #[test]
    fn valid_profile_passes() {
        let tmp = TempDir::new().unwrap();
        write_profile(tmp.path(), r#"[
            {"rule_id":"MIN_CLEARANCE","condition":"distance < 5.0",
             "severity":"HIGH","regulation":"Safety Code §3.1"}
        ]"#, &[("MIN_CLEARANCE", "Keep 5 m clearance.")]);
        let r = run(tmp.path());
        assert_eq!(r.errors.len(), 0);
        assert_eq!(r.warnings.len(), 0);
    }

    #[test]
    fn missing_rules_json_is_error() {
        let tmp = TempDir::new().unwrap();
        let r = run(tmp.path());
        assert!(!r.errors.is_empty());
    }

    #[test]
    fn invalid_json_is_error() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("rules.json"), "not json").unwrap();
        let r = run(tmp.path());
        assert!(!r.errors.is_empty());
    }

    #[test]
    fn empty_array_is_error() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("rules.json"), "[]").unwrap();
        let r = run(tmp.path());
        assert!(!r.errors.is_empty());
    }

    #[test]
    fn duplicate_rule_ids_are_error() {
        let tmp = TempDir::new().unwrap();
        write_profile(tmp.path(), r#"[
            {"rule_id":"R1","condition":"distance < 5.0","severity":"HIGH","regulation":"X §1"},
            {"rule_id":"R1","condition":"ttc < 3.0","severity":"LOW","regulation":"X §2"}
        ]"#, &[]);
        let r = run(tmp.path());
        assert!(!r.errors.is_empty());
    }

    #[test]
    fn invalid_severity_is_error() {
        let tmp = TempDir::new().unwrap();
        write_profile(tmp.path(), r#"[
            {"rule_id":"R1","condition":"distance < 5.0","severity":"URGENT","regulation":"X §1"}
        ]"#, &[]);
        let r = run(tmp.path());
        assert!(!r.errors.is_empty());
    }

    #[test]
    fn unknown_condition_is_error() {
        let tmp = TempDir::new().unwrap();
        write_profile(tmp.path(), r#"[
            {"rule_id":"R1","condition":"speed > 10","severity":"HIGH","regulation":"X §1"}
        ]"#, &[]);
        let r = run(tmp.path());
        assert!(!r.errors.is_empty());
    }

    #[test]
    fn zone_member_without_zone_is_error() {
        let tmp = TempDir::new().unwrap();
        write_profile(tmp.path(), r#"[
            {"rule_id":"ZONE","condition":"zone_member","severity":"CRITICAL","regulation":"X §2"}
        ]"#, &[]);
        let r = run(tmp.path());
        assert!(!r.errors.is_empty());
    }

    #[test]
    fn zone_member_with_valid_polygon_passes() {
        let tmp = TempDir::new().unwrap();
        write_profile(tmp.path(), r#"[
            {"rule_id":"ZONE","condition":"zone_member","severity":"CRITICAL",
             "regulation":"X §2","zone":[[0,0],[10,0],[10,10],[0,10]]}
        ]"#, &[("ZONE", "Exclusion zone.")]);
        let r = run(tmp.path());
        assert_eq!(r.errors.len(), 0);
    }

    #[test]
    fn zone_with_fewer_than_3_vertices_is_error() {
        let tmp = TempDir::new().unwrap();
        write_profile(tmp.path(), r#"[
            {"rule_id":"ZONE","condition":"zone_member","severity":"CRITICAL",
             "regulation":"X §2","zone":[[0,0],[10,0]]}
        ]"#, &[]);
        let r = run(tmp.path());
        assert!(!r.errors.is_empty());
    }

    #[test]
    fn missing_kb_file_is_warning_not_error() {
        let tmp = TempDir::new().unwrap();
        write_profile(tmp.path(), r#"[
            {"rule_id":"R1","condition":"distance < 5.0","severity":"HIGH","regulation":"X §1"}
        ]"#, &[]); // no KB files
        let r = run(tmp.path());
        assert_eq!(r.errors.len(), 0);
        assert!(!r.warnings.is_empty());
    }

    #[test]
    fn orphaned_kb_file_is_warning() {
        let tmp = TempDir::new().unwrap();
        write_profile(tmp.path(), r#"[
            {"rule_id":"R1","condition":"distance < 5.0","severity":"HIGH","regulation":"X §1"}
        ]"#, &[
            ("R1", "Relevant regulation text."),
            ("R2_ORPHAN", "This has no matching rule."),
        ]);
        let r = run(tmp.path());
        assert_eq!(r.errors.len(), 0);
        assert!(!r.warnings.is_empty());
    }

    #[test]
    fn missing_regulation_field_is_error() {
        let tmp = TempDir::new().unwrap();
        write_profile(tmp.path(), r#"[
            {"rule_id":"R1","condition":"distance < 5.0","severity":"HIGH"}
        ]"#, &[]);
        let r = run(tmp.path());
        assert!(!r.errors.is_empty());
    }

    #[test]
    fn rule_id_with_spaces_is_error() {
        let tmp = TempDir::new().unwrap();
        write_profile(tmp.path(), r#"[
            {"rule_id":"bad id","condition":"distance < 5.0","severity":"HIGH","regulation":"X §1"}
        ]"#, &[]);
        let r = run(tmp.path());
        assert!(!r.errors.is_empty());
    }

    #[test]
    fn load_profile_returns_rules() {
        let tmp = TempDir::new().unwrap();
        write_profile(tmp.path(), r#"[
            {"rule_id":"R1","condition":"distance < 5.0","severity":"HIGH","regulation":"X §1"},
            {"rule_id":"R2","condition":"ttc < 3.0","severity":"LOW","regulation":"X §2"}
        ]"#, &[]);
        let rules = load_profile(tmp.path()).unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].rule_id, "R1");
        assert_eq!(rules[1].rule_id, "R2");
    }

    #[test]
    fn load_profile_missing_file_is_error() {
        let tmp = TempDir::new().unwrap();
        assert!(load_profile(tmp.path()).is_err());
    }
}
