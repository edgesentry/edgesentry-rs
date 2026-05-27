//! Port Cyber Clearance evaluation manifest sealing (Cap Vista W4).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::CliError;

/// Payload sealed into the audit chain for an indago clearance evaluation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClearanceAuditPayload {
    pub record_type: String,
    pub manifest: ClearanceManifestBody,
}

/// Manifest fields hashed by indago `decision_hash` (excludes `decision_hash` itself).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClearanceManifestBody {
    pub vessel_key: String,
    pub port_call_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_pack_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_pack_version: Option<String>,
    pub rule_pack_sha256: String,
    pub cve_snapshot_sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sbom_sha256: Option<String>,
    pub outcome: String,
    pub rules_fired: Vec<String>,
    pub graph_node_count: u64,
    pub graph_edge_count: u64,
}

/// Parse evaluation manifest JSON (with optional `decision_hash` field stripped for signing).
pub fn parse_clearance_manifest_json(content: &str) -> Result<ClearanceManifestBody, CliError> {
    let value: Value = serde_json::from_str(content)?;
    manifest_body_from_value(value)
}

pub fn manifest_body_from_value(mut value: Value) -> Result<ClearanceManifestBody, CliError> {
    if let Some(obj) = value.as_object_mut() {
        obj.remove("decision_hash");
    }
    serde_json::from_value(value).map_err(Into::into)
}

/// Canonical JSON bytes for signing (matches indago `decision_hash` input + record_type wrapper).
pub fn build_clearance_payload_bytes(manifest: &ClearanceManifestBody) -> Result<Vec<u8>, CliError> {
    let payload = ClearanceAuditPayload {
        record_type: "port_cyber_clearance".to_string(),
        manifest: manifest.clone(),
    };
    let value = serde_json::to_value(&payload)?;
    let canonical = canonicalize_json(value);
    serde_json::to_vec(&canonical).map_err(Into::into)
}

pub fn payload_hash_hex(payload_bytes: &[u8]) -> String {
    hex::encode(blake3::hash(payload_bytes).as_bytes())
}

fn canonicalize_json(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let sorted: BTreeMap<String, Value> = map
                .into_iter()
                .map(|(k, v)| (k, canonicalize_json(v)))
                .collect();
            let mut map = serde_json::Map::new();
            for (k, v) in sorted {
                map.insert(k, v);
            }
            Value::Object(map)
        }
        Value::Array(items) => Value::Array(items.into_iter().map(canonicalize_json).collect()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_payload_is_stable() {
        let manifest = ClearanceManifestBody {
            vessel_key: "vessel-hold".into(),
            port_call_id: "pc-1".into(),
            rule_pack_id: Some("sg-cyber-clearance-v0".into()),
            rule_pack_version: Some("0.1.0".into()),
            rule_pack_sha256: "aa".repeat(64),
            cve_snapshot_sha256: "bb".repeat(64),
            sbom_sha256: Some("cc".repeat(64)),
            outcome: "hold".into(),
            rules_fired: vec!["SG-CC-001".into()],
            graph_node_count: 6,
            graph_edge_count: 5,
        };
        let a = build_clearance_payload_bytes(&manifest).unwrap();
        let b = build_clearance_payload_bytes(&manifest).unwrap();
        assert_eq!(a, b);
    }
}
