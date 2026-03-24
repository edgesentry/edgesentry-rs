//! CLI integration tests for the `eds` binary.
//!
//! Each test invokes the compiled binary via `std::process::Command`.
//! Cargo sets `CARGO_BIN_EXE_eds` to the binary path when running `cargo test`.

use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};

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
    let out = eds().arg("keygen").output().expect("eds keygen");
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
    let out = eds().args(["keygen", "--out"]).arg(tmp.path()).output().expect("eds keygen --out");
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(tmp.path().exists(), "output file not created");

    let content = fs::read_to_string(tmp.path()).expect("read file");
    let json: serde_json::Value = serde_json::from_str(&content).expect("file is JSON");
    assert_eq!(json["private_key_hex"].as_str().unwrap().len(), 64);
}

#[test]
fn keygen_produces_unique_pairs() {
    let run = |_| {
        let o = eds().arg("keygen").output().expect("eds keygen");
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
        .args(["inspect-key", "--private-key-hex", PRIV_HEX])
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
        .args(["inspect-key", "--private-key-hex", "not-valid-hex"])
        .output()
        .expect("eds inspect-key");
    assert!(!out.status.success(), "should exit non-zero for invalid hex");
}

#[test]
fn inspect_key_roundtrips_with_keygen() {
    // Generate a fresh keypair, then inspect-key must return the same public key.
    let kg = eds().arg("keygen").output().expect("keygen");
    let kj: serde_json::Value = serde_json::from_str(&stdout(&kg)).unwrap();
    let priv_hex = kj["private_key_hex"].as_str().unwrap();
    let expected_pub = kj["public_key_hex"].as_str().unwrap();

    let ik = eds()
        .args(["inspect-key", "--private-key-hex", priv_hex])
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
            "sign-record",
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
            "sign-record",
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
        .args(["inspect-key", "--private-key-hex", PRIV_HEX])
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
        .args(["verify-record", "--record-file"])
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
            .args(["inspect-key", "--private-key-hex", wrong_key])
            .output()
            .unwrap();
        let j: serde_json::Value = serde_json::from_str(&stdout(&o)).unwrap();
        j["public_key_hex"].as_str().unwrap().to_string()
    };

    let out = eds()
        .args(["verify-record", "--record-file"])
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
            "demo-lift-inspection",
            "--private-key-hex", PRIV_HEX,
            "--out-file",
        ])
        .arg(chain_file.path())
        .output()
        .expect("eds demo-lift-inspection");
    assert!(out.status.success(), "{}", stderr(&out));

    let verify = eds()
        .args(["verify-chain", "--records-file"])
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
            "demo-lift-inspection",
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
        .args(["verify-chain", "--records-file"])
        .arg(chain_file.path())
        .output()
        .expect("eds verify-chain tampered");

    assert!(!verify.status.success(), "tampered chain should exit non-zero");
}

#[test]
fn verify_chain_exits_nonzero_for_missing_file() {
    let out = eds()
        .args(["verify-chain", "--records-file", "/nonexistent/path/chain.json"])
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
            "demo-lift-inspection",
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
            "demo-lift-inspection",
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

// ── server commands ───────────────────────────────────────────────────────────

/// Bind an ephemeral port, drop the listener, and return the address string.
/// The port is free for the next bind — there is a brief TOCTOU window but
/// it is acceptable for test use.
fn free_addr() -> String {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    l.local_addr().unwrap().to_string()
}

/// Poll TCP connect until the address is accepting connections or the timeout
/// elapses.  Returns `true` if the server became ready in time.
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
    let out = eds().args(["serve", "--help"]).output().expect("eds serve --help");
    assert!(out.status.success(), "exit: {:?}\n{}", out.status, stderr(&out));
    let help = stdout(&out);
    assert!(help.contains("--addr"), "serve --help must list --addr");
    assert!(help.contains("--allowed-sources"), "serve --help must list --allowed-sources");
    assert!(help.contains("--device"), "serve --help must list --device");
}

#[cfg(feature = "transport-tls")]
#[test]
fn serve_tls_help_lists_expected_flags() {
    let out = eds().args(["serve-tls", "--help"]).output().expect("eds serve-tls --help");
    assert!(out.status.success(), "exit: {:?}\n{}", out.status, stderr(&out));
    let help = stdout(&out);
    assert!(help.contains("--tls-cert"), "serve-tls --help must list --tls-cert");
    assert!(help.contains("--tls-key"), "serve-tls --help must list --tls-key");
    assert!(help.contains("--addr"), "serve-tls --help must list --addr");
}

#[cfg(feature = "transport-mqtt")]
#[test]
fn serve_mqtt_help_lists_expected_flags() {
    let out = eds().args(["serve-mqtt", "--help"]).output().expect("eds serve-mqtt --help");
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
            "serve",
            "--addr", &addr,
            "--allowed-sources", "127.0.0.1",
            "--device", &format!("dev-cli={pub_hex}"),
        ])
        .spawn()
        .expect("eds serve must spawn");

    if !wait_for_tcp(&addr, 10) {
        child.kill().ok();
        panic!("eds serve did not bind within 10 s");
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
    std::fs::write(&key_path, cert.key_pair.serialize_pem()).expect("write key");

    let signing_key = SigningKey::from_bytes(&[2u8; 32]);
    let pub_hex = hex::encode(signing_key.verifying_key().to_bytes());
    let addr = free_addr();

    let mut child = eds()
        .args([
            "serve-tls",
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

    if !wait_for_tcp(&addr, 10) {
        child.kill().ok();
        panic!("eds serve-tls did not bind within 10 s");
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
        .args(["sign-record", "--sequence", "1", "--timestamp-ms", "1", "--payload", "x",
               "--object-ref", "s3://b/1", "--private-key-hex", PRIV_HEX])
        .output()
        .expect("sign-record missing --device-id");
    assert!(!out.status.success());
}
