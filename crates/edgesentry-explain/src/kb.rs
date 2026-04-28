use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Regulatory KB: maps rule_id → text snippet loaded from `<profile_dir>/kb/<rule_id>.txt`.
pub struct KnowledgeBase {
    snippets: HashMap<String, String>,
}

impl KnowledgeBase {
    /// Load all `.txt` files from `<profile_dir>/kb/`.
    /// Each file must be named `<RULE_ID>.txt`; the stem becomes the key.
    pub fn load(profile_dir: &str) -> Result<Self, String> {
        let kb_dir = Path::new(profile_dir).join("kb");
        let mut snippets = HashMap::new();

        let entries = fs::read_dir(&kb_dir)
            .map_err(|e| format!("Cannot read KB dir {}: {e}", kb_dir.display()))?;

        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("txt") {
                continue;
            }
            let rule_id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| format!("Bad filename: {}", path.display()))?
                .to_string();
            let text = fs::read_to_string(&path)
                .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
            snippets.insert(rule_id, text.trim().to_string());
        }

        Ok(Self { snippets })
    }

    /// Build from an in-memory map (used in tests without touching the filesystem).
    pub fn from_map(map: HashMap<String, String>) -> Self {
        Self { snippets: map }
    }

    /// Look up the regulation snippet for a rule. Returns `None` if the rule has no KB entry.
    pub fn get(&self, rule_id: &str) -> Option<&str> {
        self.snippets.get(rule_id).map(String::as_str)
    }

    pub fn rule_ids(&self) -> impl Iterator<Item = &str> {
        self.snippets.keys().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_kb() -> KnowledgeBase {
        let mut m = HashMap::new();
        m.insert(
            "PROXIMITY_ALERT".to_string(),
            "Minimum 5 m clearance from pedestrians — Site Safety §3.1".to_string(),
        );
        m.insert(
            "TTC_ALERT".to_string(),
            "TTC below 3 s requires immediate stop — Site Safety §3.2".to_string(),
        );
        KnowledgeBase::from_map(m)
    }

    #[test]
    fn known_rule_returns_snippet() {
        let kb = make_kb();
        assert!(kb.get("PROXIMITY_ALERT").unwrap().contains("5 m"));
    }

    #[test]
    fn unknown_rule_returns_none() {
        let kb = make_kb();
        assert!(kb.get("DOES_NOT_EXIST").is_none());
    }

    #[test]
    fn rule_ids_lists_all_keys() {
        let kb = make_kb();
        let mut ids: Vec<&str> = kb.rule_ids().collect();
        ids.sort();
        assert_eq!(ids, vec!["PROXIMITY_ALERT", "TTC_ALERT"]);
    }
}
