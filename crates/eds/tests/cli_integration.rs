//! CLI integration tests for the `eds` binary.
//!
//! Each test invokes the compiled binary via `std::process::Command`.
//! Cargo sets `CARGO_BIN_EXE_eds` to the binary path when running `cargo test`.

use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

static FILE_COUNTER: AtomicUsize = AtomicUsize::new(0);

// ── helpers ──────────────────────────────────────────────────────────────────

fn eds() -> Command {
    let bin = env!("CARGO_BIN_EXE_eds");
    Command::new(bin)
}

/// A temp path that is deleted on drop.
struct TmpFile(PathBuf);

impl TmpFile {
    fn new(name: &str) -> Self {
        let mut p = std::env::temp_dir();
        p.push(format!("eds_cli_test_{}_{name}", std::process::id()));
        Self(p)
    }
    fn path(&self) -> &PathBuf {
        &self.0
    }
}

impl Drop for TmpFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

// A fixed private key (all 0x01 bytes — same as the demo default).
const PRIV_HEX: &str = "0101010101010101010101010101010101010101010101010101010101010101";

// ── keygen ───────────────────────────────────────────────────────────────────

#[test]
fn keygen_exits_zero_and_outputs_valid_json() {
    let out = eds().args(["audit", "keygen"]).output().expect("eds keygen");
    assert!(out.status.success(), "exit code: {:?}\n{}", out.status, stderr(&out));

    let json: serde_json::Value = serde_json::from_str(&stdout(&out)).expect("stdout is JSON");
    let priv_hex = json["private_key_hex"].as_str().expect("private_key_hex");
    let pub_hex = json["public_key_hex"].as_str().expect("public_key_hex");
    assert_eq!(priv_hex.len(), 64, "private key must be 32 bytes / 64 hex chars");
    assert_eq!(pub_hex.len(), 64, "public key must be 32 bytes / 64 hex chars");
}

#[test]
fn keygen_out_flag_writes_file() {
    let tmp = TmpFile::new("keygen.json");
    let out = eds().args(["audit", "keygen", "--out"]).arg(tmp.path()).output().expect("eds keygen --out");
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(tmp.path().exists(), "output file not created");

    let content = fs::read_to_string(tmp.path()).expect("read file");
    let json: serde_json::Value = serde_json::from_str(&content).expect("file is JSON");
    assert_eq!(json["private_key_hex"].as_str().unwrap().len(), 64);
}

#[test]
fn keygen_produces_unique_pairs() {
    let run = |_| {
        let o = eds().args(["audit", "keygen"]).output().expect("eds keygen");
        let j: serde_json::Value = serde_json::from_str(&stdout(&o)).unwrap();
        j["public_key_hex"].as_str().unwrap().to_string()
    };
    let k1 = run(0);
    let k2 = run(1);
    assert_ne!(k1, k2, "two keygen runs must not produce the same key");
}

// ── inspect-key ──────────────────────────────────────────────────────────────

#[test]
fn inspect_key_derives_expected_public_key() {
    // Derive what the public key should be from the same private key.
    let keygen_out = eds()
        .args(["audit", "inspect-key", "--private-key-hex", PRIV_HEX])
        .output()
        .expect("eds inspect-key");
    assert!(keygen_out.status.success(), "{}", stderr(&keygen_out));

    let json: serde_json::Value = serde_json::from_str(&stdout(&keygen_out)).unwrap();
    assert_eq!(json["private_key_hex"].as_str().unwrap(), PRIV_HEX);
    assert_eq!(json["public_key_hex"].as_str().unwrap().len(), 64);
}

#[test]
fn inspect_key_rejects_invalid_hex() {
    let out = eds()
        .args(["audit", "inspect-key", "--private-key-hex", "not-valid-hex"])
        .output()
        .expect("eds inspect-key");
    assert!(!out.status.success(), "should exit non-zero for invalid hex");
}

#[test]
fn inspect_key_roundtrips_with_keygen() {
    // Generate a fresh keypair, then inspect-key must return the same public key.
    let kg = eds().args(["audit", "keygen"]).output().expect("keygen");
    let kj: serde_json::Value = serde_json::from_str(&stdout(&kg)).unwrap();
    let priv_hex = kj["private_key_hex"].as_str().unwrap();
    let expected_pub = kj["public_key_hex"].as_str().unwrap();

    let ik = eds()
        .args(["audit", "inspect-key", "--private-key-hex", priv_hex])
        .output()
        .expect("inspect-key");
    assert!(ik.status.success(), "{}", stderr(&ik));
    let ij: serde_json::Value = serde_json::from_str(&stdout(&ik)).unwrap();
    assert_eq!(ij["public_key_hex"].as_str().unwrap(), expected_pub);
}

// ── sign-record ──────────────────────────────────────────────────────────────

fn sign_record_to_file(priv_hex: &str, out: &TmpFile) -> Output {
    eds()
        .args([
            "audit", "sign-record",
            "--device-id", "lift-01",
            "--sequence", "1",
            "--timestamp-ms", "1700000000000",
            "--payload", "door=open",
            "--object-ref", "s3://bucket/lift-01/1.bin",
            "--private-key-hex", priv_hex,
            "--out",
        ])
        .arg(out.path())
        .output()
        .expect("eds sign-record")
}

#[test]
fn sign_record_exits_zero_and_writes_valid_json() {
    let tmp = TmpFile::new("record.json");
    let out = sign_record_to_file(PRIV_HEX, &tmp);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(tmp.path().exists());

    let content = fs::read_to_string(tmp.path()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json["device_id"].as_str().unwrap(), "lift-01");
    assert_eq!(json["sequence"].as_u64().unwrap(), 1);
}

#[test]
fn sign_record_rejects_invalid_private_key() {
    let tmp = TmpFile::new("record_bad.json");
    let out = eds()
        .args([
            "audit", "sign-record",
            "--device-id", "lift-01",
            "--sequence", "1",
            "--timestamp-ms", "1700000000000",
            "--payload", "door=open",
            "--object-ref", "s3://bucket/lift-01/1.bin",
            "--private-key-hex", "not-hex",
            "--out",
        ])
        .arg(tmp.path())
        .output()
        .expect("eds sign-record bad key");
    assert!(!out.status.success(), "should exit non-zero");
}

// ── verify-record ─────────────────────────────────────────────────────────────

#[test]
fn verify_record_prints_valid_for_correct_key() {
    // First: get the matching public key.
    let ik = eds()
        .args(["audit", "inspect-key", "--private-key-hex", PRIV_HEX])
        .output()
        .unwrap();
    let ij: serde_json::Value = serde_json::from_str(&stdout(&ik)).unwrap();
    let pub_hex = ij["public_key_hex"].as_str().unwrap().to_string();

    // Sign a record.
    let rec = TmpFile::new("vr_record.json");
    let s = sign_record_to_file(PRIV_HEX, &rec);
    assert!(s.status.success());

    // Verify with the correct key.
    let out = eds()
        .args(["audit", "verify-record", "--record-file"])
        .arg(rec.path())
        .args(["--public-key-hex", &pub_hex])
        .output()
        .expect("eds verify-record");

    assert!(out.status.success(), "exit: {:?}\n{}", out.status, stderr(&out));
    assert!(stdout(&out).trim() == "VALID", "stdout: {}", stdout(&out));
}

#[test]
fn verify_record_exits_2_for_wrong_key() {
    let rec = TmpFile::new("vr_wrong_record.json");
    let s = sign_record_to_file(PRIV_HEX, &rec);
    assert!(s.status.success());

    // Use a different (all-0x02) key.
    let wrong_key = "0202020202020202020202020202020202020202020202020202020202020202";
    let wrong_pub = {
        let o = eds()
            .args(["audit", "inspect-key", "--private-key-hex", wrong_key])
            .output()
            .unwrap();
        let j: serde_json::Value = serde_json::from_str(&stdout(&o)).unwrap();
        j["public_key_hex"].as_str().unwrap().to_string()
    };

    let out = eds()
        .args(["audit", "verify-record", "--record-file"])
        .arg(rec.path())
        .args(["--public-key-hex", &wrong_pub])
        .output()
        .expect("eds verify-record wrong key");

    assert_eq!(out.status.code(), Some(2), "should exit 2 for INVALID");
    assert!(stdout(&out).trim() == "INVALID", "stdout: {}", stdout(&out));
}

// ── verify-chain ──────────────────────────────────────────────────────────────

#[test]
fn verify_chain_exits_zero_for_valid_chain() {
    let chain_file = TmpFile::new("chain.json");
    // Use demo-lift-inspection to produce a valid chain file.
    let out = eds()
        .args([
            "audit", "demo-lift-inspection",
            "--private-key-hex", PRIV_HEX,
            "--out-file",
        ])
        .arg(chain_file.path())
        .output()
        .expect("eds demo-lift-inspection");
    assert!(out.status.success(), "{}", stderr(&out));

    let verify = eds()
        .args(["audit", "verify-chain", "--records-file"])
        .arg(chain_file.path())
        .output()
        .expect("eds verify-chain");

    assert!(verify.status.success(), "{}", stderr(&verify));
    assert!(stdout(&verify).trim() == "CHAIN_VALID", "stdout: {}", stdout(&verify));
}

#[test]
fn verify_chain_exits_nonzero_for_tampered_chain() {
    let chain_file = TmpFile::new("tampered_chain.json");
    let out = eds()
        .args([
            "audit", "demo-lift-inspection",
            "--private-key-hex", PRIV_HEX,
            "--out-file",
        ])
        .arg(chain_file.path())
        .output()
        .unwrap();
    assert!(out.status.success());

    // Tamper: corrupt the first byte of prev_record_hash in the second record.
    // The field is serialized as a JSON array of integers ([u8; 32]).
    let content = fs::read_to_string(chain_file.path()).unwrap();
    let mut records: Vec<serde_json::Value> = serde_json::from_str(&content).unwrap();
    records[1]["prev_record_hash"][0] = serde_json::Value::Number(0xff.into());
    fs::write(chain_file.path(), serde_json::to_string_pretty(&records).unwrap()).unwrap();

    let verify = eds()
        .args(["audit", "verify-chain", "--records-file"])
        .arg(chain_file.path())
        .output()
        .expect("eds verify-chain tampered");

    assert!(!verify.status.success(), "tampered chain should exit non-zero");
}

#[test]
fn verify_chain_exits_nonzero_for_missing_file() {
    let out = eds()
        .args(["audit", "verify-chain", "--records-file", "/nonexistent/path/chain.json"])
        .output()
        .expect("eds verify-chain missing");
    assert!(!out.status.success());
}

// ── demo-lift-inspection ─────────────────────────────────────────────────────

#[test]
fn demo_lift_inspection_creates_output_and_validates_chain() {
    let out_file = TmpFile::new("demo_out.json");
    let out = eds()
        .args([
            "audit", "demo-lift-inspection",
            "--private-key-hex", PRIV_HEX,
            "--out-file",
        ])
        .arg(out_file.path())
        .output()
        .expect("eds demo-lift-inspection");

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(out_file.path().exists(), "output file not created");

    let s = stdout(&out);
    assert!(s.contains("DEMO_CREATED:"), "missing DEMO_CREATED in stdout:\n{s}");
    assert!(s.contains("CHAIN_VALID"), "missing CHAIN_VALID in stdout:\n{s}");
}

#[test]
fn demo_lift_inspection_writes_payloads_file_when_requested() {
    let out_file = TmpFile::new("demo_records.json");
    let payloads_file = TmpFile::new("demo_payloads.json");
    let out = eds()
        .args([
            "audit", "demo-lift-inspection",
            "--private-key-hex", PRIV_HEX,
            "--out-file",
        ])
        .arg(out_file.path())
        .args(["--payloads-file"])
        .arg(payloads_file.path())
        .output()
        .expect("eds demo-lift-inspection payloads");

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(payloads_file.path().exists(), "payloads file not created");

    let payloads: Vec<String> =
        serde_json::from_str(&fs::read_to_string(payloads_file.path()).unwrap()).unwrap();
    assert_eq!(payloads.len(), 3, "expected 3 payloads");
}

// ── Phase 2: ingest / evaluate / assess / explain pipeline ───────────────────

/// Resolve a path relative to the workspace root (two levels above this
/// crate's Cargo.toml).
fn workspace_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
        .canonicalize()
        .expect("workspace path must exist")
}

fn demo_fixture_csv() -> PathBuf {
    workspace_path("crates/edgesentry-ingest/fixtures/forklift_approach.csv")
}

fn demo_profile_dir() -> PathBuf {
    workspace_path("crates/edgesentry-profile/fixtures/demo")
}

fn vessel_fixture_csv() -> PathBuf {
    workspace_path("crates/edgesentry-ingest/fixtures/vessel_zone_approach.csv")
}

fn ais_maritime_fixture_csv() -> PathBuf {
    workspace_path("crates/edgesentry-ingest/fixtures/ais_maritime_approach.csv")
}

fn sg_maritime_profile_dir() -> PathBuf {
    workspace_path("crates/edgesentry-profile/fixtures/sg-maritime-security")
}

fn zone_test_profile_dir() -> PathBuf {
    workspace_path("crates/edgesentry-profile/fixtures/zone-test")
}

#[test]
fn ingest_replay_with_demo_fixture_exits_zero() {
    let out_file = TmpFile::new("frames.jsonl");
    let out = eds()
        .args(["ingest", "replay",
               "--source"]).arg(demo_fixture_csv())
        .args(["--profile"]).arg(demo_profile_dir())
        .args(["--out"]).arg(out_file.path())
        .output()
        .expect("eds ingest replay");

    assert!(out.status.success(), "exit: {:?}\n{}", out.status, stderr(&out));
    assert!(out_file.path().exists(), "output file not created");

    let content = fs::read_to_string(out_file.path()).unwrap();
    let mut lines = content.lines();
    let header: serde_json::Value = serde_json::from_str(lines.next().unwrap()).unwrap();
    assert_eq!(header["eds_schema"], "eds.entity-frame", "wrong schema header");
    assert!(lines.count() > 0, "no entity frames written");
}

#[test]
fn evaluate_run_on_demo_fixture_produces_risk_events() {
    let frames = TmpFile::new("frames_eval.jsonl");
    let events = TmpFile::new("events_eval.jsonl");

    // ingest
    let r = eds()
        .args(["ingest", "replay", "--source"]).arg(demo_fixture_csv())
        .args(["--profile"]).arg(demo_profile_dir())
        .args(["--out"]).arg(frames.path())
        .output().unwrap();
    assert!(r.status.success(), "ingest failed: {}", stderr(&r));

    // evaluate
    let out = eds()
        .args(["evaluate", "run", "--input"]).arg(frames.path())
        .args(["--profile"]).arg(demo_profile_dir())
        .args(["--out"]).arg(events.path())
        .output()
        .expect("eds evaluate run");

    assert!(out.status.success(), "exit: {:?}\n{}", out.status, stderr(&out));

    let content = fs::read_to_string(events.path()).unwrap();
    let mut lines = content.lines();
    let header: serde_json::Value = serde_json::from_str(lines.next().unwrap()).unwrap();
    assert_eq!(header["eds_schema"], "eds.risk-event");

    let event_lines: Vec<&str> = lines.collect();
    assert!(!event_lines.is_empty(), "demo fixture must produce at least one RiskEvent");

    // verify the expected rule fires
    let combined = event_lines.join("\n");
    assert!(combined.contains("PROXIMITY_ALERT"), "PROXIMITY_ALERT must fire on the demo fixture");
}

#[test]
fn assess_run_on_demo_events_produces_assessment() {
    let frames  = TmpFile::new("frames_assess.jsonl");
    let events  = TmpFile::new("events_assess.jsonl");
    let assessment = TmpFile::new("assessment.jsonl");

    // ingest → evaluate
    let r = eds()
        .args(["ingest", "replay", "--source"]).arg(demo_fixture_csv())
        .args(["--profile"]).arg(demo_profile_dir())
        .args(["--out"]).arg(frames.path()).output().unwrap();
    assert!(r.status.success(), "ingest: {}", stderr(&r));

    let r = eds()
        .args(["evaluate", "run", "--input"]).arg(frames.path())
        .args(["--profile"]).arg(demo_profile_dir())
        .args(["--out"]).arg(events.path()).output().unwrap();
    assert!(r.status.success(), "evaluate: {}", stderr(&r));

    // assess
    let out = eds()
        .args(["assess", "run", "--input"]).arg(events.path())
        .args(["--out"]).arg(assessment.path())
        .output()
        .expect("eds assess run");

    assert!(out.status.success(), "exit: {:?}\n{}", out.status, stderr(&out));

    let content = fs::read_to_string(assessment.path()).unwrap();
    let mut lines = content.lines();
    let header: serde_json::Value = serde_json::from_str(lines.next().unwrap()).unwrap();
    assert_eq!(header["eds_schema"], "eds.assessment");

    let body: serde_json::Value =
        serde_json::from_str(lines.next().expect("assessment body line missing")).unwrap();
    assert!(body["event_count"].as_u64().unwrap_or(0) > 0, "event_count must be > 0");
    assert!(body.get("trend").is_some(), "assessment must contain a trend field");
}

#[test]
fn assess_run_window_sec_filters_events() {
    // Create a synthetic events JSONL with two events far apart in time.
    // A 1-second window should include only the newest event.
    let events_file = TmpFile::new("events_window.jsonl");
    let assessment  = TmpFile::new("assessment_window.jsonl");

    let header = r#"{"eds_schema":"eds.risk-event","version":"0.1"}"#;
    let old_event = r#"{"rule_id":"PROXIMITY_ALERT","severity":"HIGH","regulation":"§1","entity_ids":["A","B"],"measured_value":3.0,"threshold":5.0,"timestamp_ms":1000,"confidence_cv":1.0,"evidence_quality":"CERTIFIED"}"#;
    let new_event = r#"{"rule_id":"PROXIMITY_ALERT","severity":"HIGH","regulation":"§1","entity_ids":["A","B"],"measured_value":3.0,"threshold":5.0,"timestamp_ms":60000,"confidence_cv":1.0,"evidence_quality":"CERTIFIED"}"#;
    fs::write(events_file.path(), format!("{header}\n{old_event}\n{new_event}\n")).unwrap();

    let out = eds()
        .args(["assess", "run",
               "--input"]).arg(events_file.path())
        .args(["--window-sec", "1",
               "--out"]).arg(assessment.path())
        .output()
        .expect("eds assess run --window-sec");

    assert!(out.status.success(), "{}", stderr(&out));
    let content = fs::read_to_string(assessment.path()).unwrap();
    let body: serde_json::Value = serde_json::from_str(content.lines().nth(1).unwrap()).unwrap();
    assert_eq!(body["event_count"].as_u64(), Some(1), "window-sec=1 must retain only the newest event");
}

#[test]
fn explain_run_help_lists_expected_flags() {
    let out = eds().args(["explain", "run", "--help"]).output().expect("eds explain run --help");
    assert!(out.status.success(), "{}", stderr(&out));
    let help = stdout(&out);
    assert!(help.contains("--input"),   "must list --input");
    assert!(help.contains("--out"),     "must list --out");
    assert!(help.contains("--n"),       "must list --n");
    assert!(help.contains("--pick"),    "must list --pick");
    assert!(help.contains("--llm-url"), "must list --llm-url");
    assert!(help.contains("--profile"), "must list --profile");
}

#[test]
fn ingest_stream_help_lists_expected_flags() {
    let out = eds().args(["ingest", "stream", "--help"]).output().expect("eds ingest stream --help");
    assert!(out.status.success(), "{}", stderr(&out));
    let help = stdout(&out);
    assert!(help.contains("--profile"), "must list --profile");
    assert!(help.contains("--out"),     "must list --out");
}

// ── server commands ───────────────────────────────────────────────────────────

/// Bind an ephemeral port, drop the listener, and return the address string.
/// The port is free for the next bind — there is a brief TOCTOU window but
/// it is acceptable for test use.
#[cfg(feature = "transport-http")]
fn free_addr() -> String {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    l.local_addr().unwrap().to_string()
}

/// Poll TCP connect until the address is accepting connections or the timeout
/// elapses.  Returns `true` if the server became ready in time.
#[cfg(feature = "transport-http")]
fn wait_for_tcp(addr: &str, timeout_secs: u64) -> bool {
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    while std::time::Instant::now() < deadline {
        if std::net::TcpStream::connect(addr).is_ok() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    false
}

#[cfg(feature = "transport-http")]
#[test]
fn serve_help_lists_expected_flags() {
    let out = eds().args(["audit", "serve", "--help"]).output().expect("eds serve --help");
    assert!(out.status.success(), "exit: {:?}\n{}", out.status, stderr(&out));
    let help = stdout(&out);
    assert!(help.contains("--addr"), "serve --help must list --addr");
    assert!(help.contains("--allowed-sources"), "serve --help must list --allowed-sources");
    assert!(help.contains("--device"), "serve --help must list --device");
}

#[cfg(feature = "transport-tls")]
#[test]
fn serve_tls_help_lists_expected_flags() {
    let out = eds().args(["audit", "serve-tls", "--help"]).output().expect("eds serve-tls --help");
    assert!(out.status.success(), "exit: {:?}\n{}", out.status, stderr(&out));
    let help = stdout(&out);
    assert!(help.contains("--tls-cert"), "serve-tls --help must list --tls-cert");
    assert!(help.contains("--tls-key"), "serve-tls --help must list --tls-key");
    assert!(help.contains("--addr"), "serve-tls --help must list --addr");
}

#[cfg(feature = "transport-mqtt")]
#[test]
fn serve_mqtt_help_lists_expected_flags() {
    let out = eds().args(["audit", "serve-mqtt", "--help"]).output().expect("eds serve-mqtt --help");
    assert!(out.status.success(), "exit: {:?}\n{}", out.status, stderr(&out));
    let help = stdout(&out);
    assert!(help.contains("--broker"), "serve-mqtt --help must list --broker");
    assert!(help.contains("--port"), "serve-mqtt --help must list --port");
    assert!(help.contains("--topic"), "serve-mqtt --help must list --topic");
}

#[cfg(feature = "transport-http")]
#[test]
fn serve_accepts_valid_record_returns_202() {
    use edgesentry_audit::{build_signed_record, AuditRecord};
    use ed25519_dalek::SigningKey;

    let signing_key = SigningKey::from_bytes(&[1u8; 32]);
    let pub_hex = hex::encode(signing_key.verifying_key().to_bytes());
    let addr = free_addr();

    let mut child = eds()
        .args([
            "audit", "serve",
            "--addr", &addr,
            "--allowed-sources", "127.0.0.1",
            "--device", &format!("dev-cli={pub_hex}"),
        ])
        .spawn()
        .expect("eds serve must spawn");

    if !wait_for_tcp(&addr, 30) {
        child.kill().ok();
        panic!("eds serve did not bind within 30 s");
    }

    let payload = b"cli-test-payload";
    let record = build_signed_record(
        "dev-cli",
        1,
        1_700_000_000_000,
        payload,
        AuditRecord::zero_hash(),
        "s3://bucket/dev-cli/1.bin",
        &signing_key,
    );
    let body = serde_json::json!({
        "record": record,
        "raw_payload_hex": hex::encode(payload),
    });

    let resp = reqwest::blocking::Client::new()
        .post(format!("http://{addr}/api/v1/ingest"))
        .json(&body)
        .send()
        .expect("HTTP request to eds serve must succeed");

    child.kill().ok();
    child.wait().ok();

    assert_eq!(resp.status().as_u16(), 202, "valid record must return 202");
}

#[cfg(feature = "transport-tls")]
#[test]
fn serve_tls_accepts_valid_record_returns_202() {
    use edgesentry_audit::{build_signed_record, AuditRecord};
    use ed25519_dalek::SigningKey;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let pid = std::process::id();
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let cert_path = std::env::temp_dir()
        .join(format!("eds_cli_tls_test_{pid}_{id}_cert.pem"));
    let key_path = std::env::temp_dir()
        .join(format!("eds_cli_tls_test_{pid}_{id}_key.pem"));

    let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])
        .expect("rcgen self-signed cert");
    std::fs::write(&cert_path, cert.cert.pem()).expect("write cert");
    std::fs::write(&key_path, cert.signing_key.serialize_pem()).expect("write key");

    let signing_key = SigningKey::from_bytes(&[2u8; 32]);
    let pub_hex = hex::encode(signing_key.verifying_key().to_bytes());
    let addr = free_addr();

    let mut child = eds()
        .args([
            "audit", "serve-tls",
            "--addr", &addr,
            "--allowed-sources", "127.0.0.1",
            "--device", &format!("dev-cli-tls={pub_hex}"),
            "--tls-cert",
        ])
        .arg(&cert_path)
        .arg("--tls-key")
        .arg(&key_path)
        .spawn()
        .expect("eds serve-tls must spawn");

    if !wait_for_tcp(&addr, 30) {
        child.kill().ok();
        panic!("eds serve-tls did not bind within 30 s");
    }

    let payload = b"cli-tls-test-payload";
    let record = build_signed_record(
        "dev-cli-tls",
        1,
        1_700_000_000_001,
        payload,
        AuditRecord::zero_hash(),
        "s3://bucket/dev-cli-tls/1.bin",
        &signing_key,
    );
    let body = serde_json::json!({
        "record": record,
        "raw_payload_hex": hex::encode(payload),
    });

    let cert_bytes = std::fs::read(&cert_path).expect("read test TLS certificate");
    let cert = reqwest::tls::Certificate::from_pem(&cert_bytes).expect("parse test TLS certificate");

    let resp = reqwest::blocking::Client::builder()
        .add_root_certificate(cert)
        .build()
        .expect("TLS client")
        .post(format!("https://localhost:{}/api/v1/ingest", addr.split(':').nth(1).unwrap()))
        .json(&body)
        .send()
        .expect("HTTPS request to eds serve-tls must succeed");

    child.kill().ok();
    child.wait().ok();
    let _ = std::fs::remove_file(&cert_path);
    let _ = std::fs::remove_file(&key_path);

    assert_eq!(resp.status().as_u16(), 202, "valid record over TLS must return 202");
}

// ── argument parsing errors ───────────────────────────────────────────────────

#[test]
fn unknown_subcommand_exits_nonzero() {
    let out = eds().arg("definitely-not-a-subcommand").output().expect("unknown subcommand");
    assert!(!out.status.success());
}

#[test]
fn help_flag_exits_zero() {
    let out = eds().arg("--help").output().expect("eds --help");
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("eds"), "help output should mention binary name");
}

#[test]
fn missing_required_arg_exits_nonzero() {
    // sign-record requires --device-id; omitting it should fail at arg parsing.
    let out = eds()
        .args(["audit", "sign-record", "--sequence", "1", "--timestamp-ms", "1", "--payload", "x",
               "--object-ref", "s3://b/1", "--private-key-hex", PRIV_HEX])
        .output()
        .expect("sign-record missing --device-id");
    assert!(!out.status.success());
}

// ── sign-document / verify-document ──────────────────────────────────────────

fn voyage_v001_csv() -> PathBuf {
    workspace_path("crates/edgesentry-document/fixtures/voyage_V001_compliant.csv")
}

fn voyage_v002_csv() -> PathBuf {
    workspace_path("crates/edgesentry-document/fixtures/voyage_V002_bwm_expired.csv")
}

/// Run `eds parse maritime` + `eds document fill` and return the resulting
/// `filled.jsonl` temp file. The intermediate `entity.jsonl` is cleaned up
/// when the returned `TmpFile` pair is dropped.
///
/// Uses the thread ID in filenames to avoid collisions between parallel tests.
fn build_filled_document(csv: &std::path::Path) -> (TmpFile, TmpFile) {
    let n = FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tid = format!("{:?}", std::thread::current().id())
        .replace("ThreadId(", "").replace(")", "");
    let entity = TmpFile::new(&format!("entity_{tid}_{n}.jsonl"));
    let filled = TmpFile::new(&format!("filled_{tid}_{n}.jsonl"));

    let parse_out = eds()
        .args(["parse", "maritime", "--source"])
        .arg(csv)
        .arg("--out").arg(entity.path())
        .output()
        .expect("eds parse maritime");
    assert!(parse_out.status.success(), "parse failed: {}", stderr(&parse_out));

    let fill_out = eds()
        .args(["document", "fill", "--input"])
        .arg(entity.path())
        .args(["--template", "fal-form-1", "--out"])
        .arg(filled.path())
        .output()
        .expect("eds document fill");
    assert!(fill_out.status.success(), "fill failed: {}", stderr(&fill_out));

    (entity, filled)
}

#[test]
fn sign_document_exits_zero_and_writes_audit_record() {
    let (_entity, filled) = build_filled_document(&voyage_v001_csv());
    let n = FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let record = TmpFile::new(&format!("record_{n}.json"));

    let out = eds()
        .args(["audit", "sign-document", "--payload"])
        .arg(filled.path())
        .args(["--key", PRIV_HEX, "--out"])
        .arg(record.path())
        .output()
        .expect("eds audit sign-document");

    assert!(out.status.success(), "sign-document failed: {}", stderr(&out));
    assert!(stdout(&out).contains("SIGNED"), "stdout must say SIGNED");
    assert!(stdout(&out).contains("1 document record"), "must report 1 record");

    // Output must be valid JSON array of AuditRecord(s).
    let content = fs::read_to_string(record.path()).expect("read record.json");
    let records: Vec<serde_json::Value> = serde_json::from_str(&content)
        .expect("record.json must be a JSON array");
    assert_eq!(records.len(), 1);
    assert!(records[0]["payload_hash"].is_array(), "payload_hash must be present");
    assert_eq!(records[0]["sequence"].as_u64(), Some(1));
}

#[test]
fn verify_document_prints_verified_for_matching_payload() {
    let (_entity, filled) = build_filled_document(&voyage_v001_csv());
    let n = FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let record = TmpFile::new(&format!("record_{n}.json"));

    eds().args(["audit", "sign-document", "--payload"])
        .arg(filled.path())
        .args(["--key", PRIV_HEX, "--out"]).arg(record.path())
        .output().expect("sign-document");

    let out = eds()
        .args(["audit", "verify-document", "--payload"])
        .arg(filled.path())
        .args(["--chain"]).arg(record.path())
        .output()
        .expect("eds audit verify-document");

    assert!(out.status.success(), "verify-document failed: {}", stderr(&out));
    let s = stdout(&out);
    assert!(s.contains("VERIFIED"), "must print VERIFIED");
    assert!(s.contains("V001"), "must include voyage_id");
    assert!(s.contains("fal-form-1"), "must include template");
    assert!(s.contains("VESSEL_NAME"), "must list field confidence");
}

#[test]
fn verify_document_exits_nonzero_when_payload_not_in_chain() {
    // Sign V001, then try to verify V002 against that chain — must fail.
    let (_e1, filled_v001) = build_filled_document(&voyage_v001_csv());
    let (_e2, filled_v002) = build_filled_document(&voyage_v002_csv());
    let n = FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let record = TmpFile::new(&format!("record_{n}.json"));

    eds().args(["audit", "sign-document", "--payload"])
        .arg(filled_v001.path())
        .args(["--key", PRIV_HEX, "--out"]).arg(record.path())
        .output().expect("sign-document");

    let out = eds()
        .args(["audit", "verify-document", "--payload"])
        .arg(filled_v002.path())  // different document
        .args(["--chain"]).arg(record.path())
        .output()
        .expect("eds audit verify-document mismatch");

    assert!(!out.status.success(), "must exit non-zero when not found");
    assert!(stderr(&out).contains("NOT FOUND"), "must print NOT FOUND");
}

#[test]
fn sign_document_chains_sequence_and_prev_hash() {
    let (_e1, filled_v001) = build_filled_document(&voyage_v001_csv());
    let (_e2, filled_v002) = build_filled_document(&voyage_v002_csv());
    let n = FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let chain1 = TmpFile::new(&format!("chain1_{n}.json"));
    let chain2 = TmpFile::new(&format!("chain2_{n}.json"));

    // Sign V001 — sequence 1.
    eds().args(["audit", "sign-document", "--payload"])
        .arg(filled_v001.path())
        .args(["--key", PRIV_HEX, "--out"]).arg(chain1.path())
        .output().expect("sign V001");

    // Sign V002 continuing from chain1 — sequence must be 2.
    let out = eds()
        .args(["audit", "sign-document", "--payload"])
        .arg(filled_v002.path())
        .args(["--key", PRIV_HEX, "--chain"]).arg(chain1.path())
        .args(["--out"]).arg(chain2.path())
        .output()
        .expect("sign V002 chained");

    assert!(out.status.success(), "{}", stderr(&out));

    let records: Vec<serde_json::Value> =
        serde_json::from_str(&fs::read_to_string(chain2.path()).unwrap()).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["sequence"].as_u64(), Some(2), "chained record must have sequence 2");

    // prev_record_hash must NOT be all zeros (i.e. it was taken from chain1).
    let prev: Vec<u8> = serde_json::from_value(records[0]["prev_record_hash"].clone()).unwrap();
    assert_ne!(prev, vec![0u8; 32], "prev_record_hash must reference chain1 record");

    // Both V001 and V002 must now be verifiable against their own chains.
    let v = eds()
        .args(["audit", "verify-document", "--payload"])
        .arg(filled_v002.path())
        .args(["--chain"]).arg(chain2.path())
        .output().expect("verify V002");
    assert!(v.status.success(), "V002 must verify: {}", stderr(&v));
    assert!(stdout(&v).contains("V002"), "must show V002 voyage_id");
}

// ── zone_member rule evaluation (generic test profile) ───────────────────────

#[test]
fn zone_test_profile_loads_and_exits_zero() {
    let frames = TmpFile::new("maritime_frames.jsonl");
    let r = eds()
        .args(["ingest", "replay", "--source"]).arg(vessel_fixture_csv())
        .args(["--profile"]).arg(zone_test_profile_dir())
        .args(["--out"]).arg(frames.path())
        .output().expect("eds ingest replay vessel");
    assert!(r.status.success(), "ingest failed: {}", stderr(&r));
    assert!(frames.path().exists(), "frames file not created");
}

#[test]
fn restricted_zone_approach_fires_when_vessel_enters_zone() {
    let frames = TmpFile::new("maritime_frames2.jsonl");
    let events = TmpFile::new("maritime_events.jsonl");

    let r = eds()
        .args(["ingest", "replay", "--source"]).arg(vessel_fixture_csv())
        .args(["--profile"]).arg(zone_test_profile_dir())
        .args(["--out"]).arg(frames.path())
        .output().unwrap();
    assert!(r.status.success(), "ingest: {}", stderr(&r));

    let r = eds()
        .args(["evaluate", "run", "--input"]).arg(frames.path())
        .args(["--profile"]).arg(zone_test_profile_dir())
        .args(["--out"]).arg(events.path())
        .output().expect("eds evaluate run vessel");
    assert!(r.status.success(), "evaluate: {}", stderr(&r));

    let content = fs::read_to_string(events.path()).unwrap();
    let events_text: Vec<&str> = content.lines()
        .filter(|l| !l.contains("eds_schema"))
        .collect();

    assert!(!events_text.is_empty(), "expected at least one RESTRICTED_ZONE_APPROACH event");

    let combined = events_text.join("\n");
    assert!(combined.contains("ZONE_ENTRY"),
        "ZONE_ENTRY must fire when entity enters zone");
    assert!(combined.contains("HIGH"),
        "severity must be HIGH");
    assert!(combined.contains("Site Safety Procedure"),
        "regulation citation must be present");
}

#[test]
fn restricted_zone_approach_does_not_fire_before_zone_entry() {
    let frames = TmpFile::new("maritime_frames3.jsonl");
    let events = TmpFile::new("maritime_events2.jsonl");

    let r = eds()
        .args(["ingest", "replay", "--source"]).arg(vessel_fixture_csv())
        .args(["--profile"]).arg(zone_test_profile_dir())
        .args(["--out"]).arg(frames.path())
        .output().unwrap();
    assert!(r.status.success(), "ingest: {}", stderr(&r));

    let r = eds()
        .args(["evaluate", "run", "--input"]).arg(frames.path())
        .args(["--profile"]).arg(zone_test_profile_dir())
        .args(["--out"]).arg(events.path())
        .output().unwrap();
    assert!(r.status.success(), "evaluate: {}", stderr(&r));

    // First 5 frames (t=0..120000ms) vessel is outside the zone (x < 300).
    // Alerts must only appear at t >= 152500 ms.
    let content = fs::read_to_string(events.path()).unwrap();
    for line in content.lines().filter(|l| !l.contains("eds_schema")) {
        let v: serde_json::Value = serde_json::from_str(line).unwrap();
        let ts = v["timestamp_ms"].as_u64().unwrap_or(0);
        assert!(ts >= 152500,
            "no alert should fire before vessel enters zone (t=152500ms), got t={ts}");
    }
}

// ── sg-maritime-security fixture (ais_maritime_approach.csv) ─────────────────

#[test]
fn ais_maritime_fixture_ingest_replay_produces_70_frames() {
    let frames = TmpFile::new("ais_frames.jsonl");
    let r = eds()
        .args(["ingest", "replay", "--source"]).arg(ais_maritime_fixture_csv())
        .args(["--out"]).arg(frames.path())
        .output().expect("eds ingest replay ais_maritime");
    assert!(r.status.success(), "ingest failed: {}", stderr(&r));
    let stderr_text = stderr(&r);
    assert!(stderr_text.contains("70 frame"), "expected 70 frames, got: {stderr_text}");
}

#[test]
fn restricted_zone_approach_fires_at_56s_on_ais_maritime_fixture() {
    let frames = TmpFile::new("ais_frames2.jsonl");
    let events = TmpFile::new("ais_events.jsonl");

    let r = eds()
        .args(["ingest", "replay", "--source"]).arg(ais_maritime_fixture_csv())
        .args(["--out"]).arg(frames.path())
        .output().unwrap();
    assert!(r.status.success(), "ingest: {}", stderr(&r));

    let r = eds()
        .args(["evaluate", "run", "--input"]).arg(frames.path())
        .args(["--profile"]).arg(sg_maritime_profile_dir())
        .args(["--out"]).arg(events.path())
        .output().expect("eds evaluate run ais_maritime");
    assert!(r.status.success(), "evaluate: {}", stderr(&r));

    let content = fs::read_to_string(events.path()).unwrap();
    let event_lines: Vec<serde_json::Value> = content.lines()
        .filter(|l| !l.contains("eds_schema"))
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();

    // RESTRICTED_ZONE_APPROACH must fire — vessel 563012345 crosses zone at t=56000 ms.
    let zone_events: Vec<&serde_json::Value> = event_lines.iter()
        .filter(|v| v["rule_id"].as_str() == Some("RESTRICTED_ZONE_APPROACH"))
        .collect();
    assert!(!zone_events.is_empty(), "RESTRICTED_ZONE_APPROACH must fire");

    let first_ts = zone_events[0]["timestamp_ms"].as_u64().unwrap_or(0);
    assert!(first_ts >= 56000,
        "must not fire before vessel reaches zone boundary (t=56000 ms), got t={first_ts}");
    assert!(first_ts <= 57000,
        "must fire within one frame of zone entry, got t={first_ts}");

    // Must not fire before zone entry.
    for ev in &zone_events {
        let ts = ev["timestamp_ms"].as_u64().unwrap_or(0);
        assert!(ts >= 56000, "early fire at t={ts}");
    }
}

#[test]
fn ais_track_gap_fires_on_ais_maritime_fixture() {
    let frames = TmpFile::new("ais_frames3.jsonl");
    let events = TmpFile::new("ais_events2.jsonl");

    let r = eds()
        .args(["ingest", "replay", "--source"]).arg(ais_maritime_fixture_csv())
        .args(["--out"]).arg(frames.path())
        .output().unwrap();
    assert!(r.status.success(), "ingest: {}", stderr(&r));

    let r = eds()
        .args(["evaluate", "run", "--input"]).arg(frames.path())
        .args(["--profile"]).arg(sg_maritime_profile_dir())
        .args(["--out"]).arg(events.path())
        .output().expect("eds evaluate run ais_gap");
    assert!(r.status.success(), "evaluate: {}", stderr(&r));

    let content = fs::read_to_string(events.path()).unwrap();
    let gap_events: Vec<serde_json::Value> = content.lines()
        .filter(|l| !l.contains("eds_schema"))
        .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap())
        .filter(|v| v["rule_id"].as_str() == Some("AIS_TRACK_GAP"))
        .collect();

    assert!(!gap_events.is_empty(), "AIS_TRACK_GAP must fire");
    let ev = &gap_events[0];
    let measured = ev["measured_value"].as_f64().unwrap_or(0.0);
    let threshold = ev["threshold"].as_f64().unwrap_or(0.0);
    assert!(measured > threshold,
        "measured gap ({measured}) must exceed threshold ({threshold})");
    assert_eq!(ev["timestamp_ms"].as_u64(), Some(60000),
        "AIS_TRACK_GAP must fire at t=60000 ms");
    assert!(ev["entity_ids"].as_array().is_some_and(|a| {
        a.iter().any(|id| id.as_str() == Some("563023456"))
    }), "AIS_TRACK_GAP must name vessel 563023456");
}

#[test]
fn ais_maritime_events_have_certified_evidence_quality() {
    let frames = TmpFile::new("ais_frames4.jsonl");
    let events = TmpFile::new("ais_events3.jsonl");

    let r = eds()
        .args(["ingest", "replay", "--source"]).arg(ais_maritime_fixture_csv())
        .args(["--out"]).arg(frames.path())
        .output().unwrap();
    assert!(r.status.success(), "ingest: {}", stderr(&r));

    let r = eds()
        .args(["evaluate", "run", "--input"]).arg(frames.path())
        .args(["--profile"]).arg(sg_maritime_profile_dir())
        .args(["--out"]).arg(events.path())
        .output().unwrap();
    assert!(r.status.success(), "evaluate: {}", stderr(&r));

    let content = fs::read_to_string(events.path()).unwrap();
    for line in content.lines().filter(|l| !l.contains("eds_schema")) {
        let v: serde_json::Value = serde_json::from_str(line).unwrap();
        let quality = v["evidence_quality"].as_str().unwrap_or("");
        assert_eq!(quality, "CERTIFIED",
            "AIS events must have CERTIFIED quality, got '{}' for rule '{}'",
            quality, v["rule_id"].as_str().unwrap_or("?"));
    }
}

#[test]
fn export_aims_exits_zero_and_writes_json_bundle() {
    let n = FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let chain = TmpFile::new(&format!("chain_{n}.json"));
    let bundle = TmpFile::new(&format!("aims_{n}.json"));
    let md = TmpFile::new(&format!("aims_{n}.md"));

    eds()
        .args(["audit", "demo-lift-inspection", "--out-file"])
        .arg(chain.path())
        .output()
        .expect("demo-lift-inspection");

    let out = eds()
        .args(["audit", "export-aims", "--chain"])
        .arg(chain.path())
        .args(["--out"])
        .arg(bundle.path())
        .args(["--md"])
        .arg(md.path())
        .output()
        .expect("eds audit export-aims");

    assert!(out.status.success(), "export-aims failed: {}", stderr(&out));
    let s = stdout(&out);
    assert!(s.contains("AIMS_EXPORT"), "stdout must contain AIMS_EXPORT");
    assert!(s.contains("chain_valid: true"), "chain must be valid");
    assert!(s.contains("controls:    A.4.2 A.4.3 A.4.4 A.4.5 A.4.6"), "all controls present");

    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(bundle.path()).expect("bundle.json"))
            .expect("bundle must be valid JSON");
    assert!(json["a4_2_resource_documentation"]["chain_valid"].as_bool() == Some(true));
    assert!(json["a4_2_resource_documentation"]["record_count"].as_u64() == Some(3));
    assert!(json["disclaimer"].is_string());

    let md_content = fs::read_to_string(md.path()).expect("aims.md");
    assert!(md_content.contains("A.4.2"), "markdown must have A.4.2 section");
    assert!(md_content.contains("A.4.6"), "markdown must have A.4.6 section");
    assert!(md_content.contains("Disclaimer"), "markdown must include disclaimer");
}

// ── port cyber clearance (W4) ─────────────────────────────────────────────────

fn clearance_manifest_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../edgesentry-audit/fixtures/clearance/vessel-hold_evaluation_manifest.json")
}

#[test]
fn sign_clearance_verify_chain_and_manifest() {
    let manifest_path = clearance_manifest_fixture();
    assert!(manifest_path.exists(), "fixture missing: {}", manifest_path.display());

    let chain = TmpFile::new("clearance_chain.json");
    let sign = eds()
        .args([
            "audit",
            "sign-clearance",
            "--manifest",
        ])
        .arg(&manifest_path)
        .args(["--key", PRIV_HEX, "--device-id", "port-clearance-poc", "--out"])
        .arg(chain.path())
        .output()
        .expect("sign-clearance");
    assert!(sign.status.success(), "sign-clearance: {}", stderr(&sign));

    let chain_check = eds()
        .args(["audit", "verify-chain", "--records-file"])
        .arg(chain.path())
        .output()
        .expect("verify-chain");
    assert!(chain_check.status.success(), "verify-chain: {}", stderr(&chain_check));
    assert!(stdout(&chain_check).contains("CHAIN_VALID"));

    let verify = eds()
        .args([
            "audit",
            "verify-clearance",
            "--manifest",
        ])
        .arg(&manifest_path)
        .args(["--chain"])
        .arg(chain.path())
        .output()
        .expect("verify-clearance");
    assert!(verify.status.success(), "verify-clearance: {}", stderr(&verify));
    let vout = stdout(&verify);
    assert!(vout.contains("VERIFIED"));
    assert!(vout.contains("vessel-hold"));
    assert!(vout.contains("hold"));
}

fn indago_repo_root() -> Option<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../indago");
    let eval = root.join("pipelines/maritime_cyber/eval.py");
    if eval.is_file() {
        Some(root)
    } else {
        None
    }
}

/// Generate a manifest via indago `evaluate_port_clearance`, then sign with eds (W4 smoke).
#[test]
fn sign_clearance_with_indago_generated_manifest() {
    let indago = match indago_repo_root() {
        Some(p) => p,
        None => {
            eprintln!("skip: sibling indago repo not found");
            return;
        }
    };

    let out_dir = TmpFile::new("indago_eval_out");
    fs::create_dir_all(out_dir.path()).expect("mkdir");

    let py = format!(
        r#"
import json
from pathlib import Path
from pipelines.maritime_cyber.eval import evaluate_port_clearance, write_evaluation_artifacts
from pipelines.maritime_cyber.graph import build_maritime_cyber_graph

g = build_maritime_cyber_graph(["vessel-hold"])
r = evaluate_port_clearance("vessel-hold", graph_result=g)
paths = write_evaluation_artifacts(r, Path("{}"))
print(json.dumps({{"manifest": str(paths["manifest"])}}))
"#,
        out_dir.path().display()
    );

    let gen = Command::new("uv")
        .args(["run", "python", "-c", &py])
        .current_dir(&indago)
        .output()
        .expect("uv run indago eval");

    if !gen.status.success() {
        let uv = Command::new("python3")
            .args(["-c", &py])
            .current_dir(&indago)
            .env("PYTHONPATH", indago.to_str().unwrap())
            .output()
            .expect("python indago eval");
        assert!(uv.status.success(), "indago eval failed:\n{}\n{}", stderr(&uv), stdout(&uv));
        let meta: serde_json::Value = serde_json::from_str(stdout(&uv).trim()).expect("json");
        let manifest = PathBuf::from(meta["manifest"].as_str().expect("manifest path"));
        run_sign_verify_clearance(&manifest);
        return;
    }

    let meta: serde_json::Value = serde_json::from_str(stdout(&gen).trim()).expect("json");
    let manifest = PathBuf::from(meta["manifest"].as_str().expect("manifest path"));
    run_sign_verify_clearance(&manifest);
}

fn run_sign_verify_clearance(manifest_path: &PathBuf) {
    assert!(manifest_path.exists(), "manifest missing: {}", manifest_path.display());
    let chain = TmpFile::new("indago_clearance_chain.json");
    let sign = eds()
        .args(["audit", "sign-clearance", "--manifest"])
        .arg(manifest_path)
        .args(["--key", PRIV_HEX, "--device-id", "port-clearance-poc", "--out"])
        .arg(chain.path())
        .output()
        .expect("sign-clearance");
    assert!(sign.status.success(), "sign-clearance: {}", stderr(&sign));

    let verify = eds()
        .args(["audit", "verify-clearance", "--manifest"])
        .arg(manifest_path)
        .args(["--chain"])
        .arg(chain.path())
        .output()
        .expect("verify-clearance");
    assert!(verify.status.success(), "verify-clearance: {}", stderr(&verify));
    assert!(stdout(&verify).contains("VERIFIED"));
}

// ── port cyber clearance document (W5) ────────────────────────────────────────

fn clearance_facts_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../edgesentry-document/fixtures/clearance")
        .join(name)
}

#[test]
fn render_clearance_hold_writes_html() {
    let facts = clearance_facts_fixture("vessel-hold_facts.json");
    assert!(facts.is_file(), "missing fixture: {}", facts.display());

    let out = TmpFile::new("clearance_hold.html");
    let render = eds()
        .args(["document", "render-clearance", "--facts"])
        .arg(&facts)
        .args([
            "--verify-url",
            "https://verify.example/clearance/hold-demo",
            "--out",
        ])
        .arg(out.path())
        .output()
        .expect("eds document render-clearance");

    assert!(render.status.success(), "render-clearance: {}", stderr(&render));
    assert!(
        stderr(&render).contains("HOLD"),
        "stderr should report HOLD outcome: {}",
        stderr(&render)
    );

    let html = fs::read_to_string(out.path()).expect("read html");
    assert!(html.contains("HOLD"));
    assert!(html.contains("vessel-hold"));
    assert!(html.contains("SG-CC-001"));
    assert!(html.contains("https://verify.example/clearance/hold-demo"));
    assert!(!html.contains("{{OUTCOME}}"));
}

#[test]
fn render_clearance_pass_writes_html() {
    let facts = clearance_facts_fixture("vessel-clean_facts.json");
    let out = TmpFile::new("clearance_pass.html");

    let render = eds()
        .args(["document", "render-clearance", "--facts"])
        .arg(&facts)
        .args(["--verify-url", "https://verify.example/clearance/pass-demo", "--out"])
        .arg(out.path())
        .output()
        .expect("eds document render-clearance");

    assert!(render.status.success(), "render-clearance: {}", stderr(&render));
    let html = fs::read_to_string(out.path()).expect("read html");
    assert!(html.contains("PASS"));
    assert!(html.contains("vessel-clean"));
    assert!(!html.contains("SG-CC-001"));
}
