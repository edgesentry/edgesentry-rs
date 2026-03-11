// EDGE DEVICE
//
// Simulates a lift sensor that signs inspection records and emits them toward
// the cloud. In production this code runs on the embedded device itself.
// The device holds a private key that never leaves the hardware.
//
// Outputs three files consumed by subsequent demo steps:
//   /tmp/eds_records.json   — signed audit record chain
//   /tmp/eds_payloads.json  — raw payload bytes (hex-encoded)
//   /tmp/eds_tampered.json  — same chain with a flipped bit (tamper demo)
//
// Run:
//   cargo run -p edgesentry-rs --example edge_device

use ed25519_dalek::SigningKey;
use edgesentry_rs::{build_signed_record, AuditRecord};

const DEVICE_ID: &str = "lift-01";
const RECORDS_FILE: &str = "/tmp/eds_records.json";
const PAYLOADS_FILE: &str = "/tmp/eds_payloads.json";
const TAMPERED_FILE: &str = "/tmp/eds_tampered.json";

fn main() {
    // The private key is unique to this device.
    // In production it is stored in a secure element or HSM (see docs/src/key_management.md).
    let signing_key = SigningKey::from_bytes(&[1u8; 32]);

    let payloads: Vec<Vec<u8>> = vec![
        b"check=door,status=ok,open_close_cycle=3".to_vec(),
        b"check=vibration,status=ok,rms=0.18".to_vec(),
        b"check=emergency_brake,status=ok,response_ms=120".to_vec(),
    ];

    let mut prev_hash = AuditRecord::zero_hash();
    let mut records = Vec::new();

    for (i, payload) in payloads.iter().enumerate() {
        let sequence = (i as u64) + 1;
        let record = build_signed_record(
            DEVICE_ID,
            sequence,
            1_700_000_000_000 + sequence * 60_000,
            payload,
            prev_hash,
            format!("s3://bucket/{DEVICE_ID}/inspection-{sequence}.bin"),
            &signing_key,
        );
        prev_hash = record.hash();
        println!(
            "EDGE DEVICE: signed   seq={} payload={}",
            record.sequence,
            String::from_utf8_lossy(payload)
        );
        records.push(record);
    }

    // Write record chain
    std::fs::write(RECORDS_FILE, serde_json::to_string_pretty(&records).unwrap()).unwrap();

    // Write payloads as hex strings (cloud backend needs them to verify payload hash)
    let hexes: Vec<String> = payloads.iter().map(hex::encode).collect();
    std::fs::write(PAYLOADS_FILE, serde_json::to_string_pretty(&hexes).unwrap()).unwrap();

    // Write a tampered copy for the tamper-detection step
    let mut tampered = records.clone();
    tampered[0].payload_hash[0] ^= 0x01;
    std::fs::write(TAMPERED_FILE, serde_json::to_string_pretty(&tampered).unwrap()).unwrap();

    println!("EDGE DEVICE: emitted  {} records → {}", records.len(), RECORDS_FILE);
    println!("EDGE DEVICE: payloads → {}", PAYLOADS_FILE);
    println!("EDGE DEVICE: tampered → {} (for tamper-detection demo)", TAMPERED_FILE);
}
