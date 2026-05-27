use edgesentry_audit::{
    build_clearance_payload_bytes, inspect_key, parse_clearance_manifest_json, sign_record,
    verify_chain_records, verify_record, AuditRecord,
};

const PRIV_HEX: &str = "0101010101010101010101010101010101010101010101010101010101010101";

#[test]
fn clearance_manifest_strips_decision_hash_for_payload() {
    let raw = include_str!("../../fixtures/clearance/vessel-hold_evaluation_manifest.json");
    let body = parse_clearance_manifest_json(raw).expect("parse");
    assert_eq!(body.vessel_key, "vessel-hold");
    assert_eq!(body.outcome, "hold");
}

#[test]
fn clearance_sign_and_verify_chain() {
    let raw = include_str!("../../fixtures/clearance/vessel-hold_evaluation_manifest.json");
    let body = parse_clearance_manifest_json(raw).expect("parse");
    let payload = build_clearance_payload_bytes(&body).expect("payload");
    let record = sign_record(
        "port-clearance-poc".into(),
        1,
        1_700_000_000_000,
        payload,
        AuditRecord::zero_hash(),
        "clearance:vessel-hold/port-call-demo-sgsin".into(),
        PRIV_HEX,
    )
    .expect("sign");
    let records = vec![record];
    verify_chain_records(&records).expect("chain");
    let pub_hex = inspect_key(PRIV_HEX).expect("inspect").public_key_hex;
    assert!(verify_record(&records[0], &pub_hex).expect("verify sig"));
}
