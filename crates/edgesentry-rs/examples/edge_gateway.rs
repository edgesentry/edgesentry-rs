// EDGE GATEWAY
//
// Simulates the optional middle tier between the edge device and the cloud
// backend (e.g. an industrial PC, 5G gateway, or protocol translator).
//
// The gateway's role is ROUTING ONLY:
//   - receives signed records from the device over a local transport
//   - does NOT verify any signatures or hashes (that is the cloud's job)
//   - buffers records during connectivity loss (not shown here)
//   - forwards records to the cloud backend over HTTPS / MQTT
//
// In this demo the transport is simulated via JSON files on disk.
// HTTP transport is outside the scope of this library.
//
// Reads:
//   /tmp/eds_records.json   — signed records from the edge device
//   /tmp/eds_payloads.json  — raw payloads from the edge device
// Writes:
//   /tmp/eds_fwd_records.json  — forwarded records (unchanged)
//   /tmp/eds_fwd_payloads.json — forwarded payloads (unchanged)
//
// Run:
//   cargo run -p edgesentry-rs --example edge_gateway

use edgesentry_rs::AuditRecord;

const IN_RECORDS: &str = "/tmp/eds_records.json";
const IN_PAYLOADS: &str = "/tmp/eds_payloads.json";
const OUT_RECORDS: &str = "/tmp/eds_fwd_records.json";
const OUT_PAYLOADS: &str = "/tmp/eds_fwd_payloads.json";

// Source IP the cloud backend will see for all records forwarded by this gateway.
// The cloud's NetworkPolicy must allowlist this address or CIDR.
const GATEWAY_SOURCE_IP: &str = "10.0.1.42";

fn main() {
    let records: Vec<AuditRecord> = serde_json::from_str(
        &std::fs::read_to_string(IN_RECORDS)
            .unwrap_or_else(|_| panic!("run edge_device first — {IN_RECORDS} not found")),
    )
    .unwrap();

    let payloads: Vec<String> = serde_json::from_str(
        &std::fs::read_to_string(IN_PAYLOADS)
            .unwrap_or_else(|_| panic!("run edge_device first — {IN_PAYLOADS} not found")),
    )
    .unwrap();

    println!(
        "EDGE GATEWAY: received {} record(s) from device (via local transport)",
        records.len()
    );
    println!("EDGE GATEWAY: no signature verification here — routing only");
    println!(
        "EDGE GATEWAY: source IP for cloud NetworkPolicy check → {}",
        GATEWAY_SOURCE_IP
    );

    for record in &records {
        println!(
            "EDGE GATEWAY: forwarding  device={} seq={}",
            record.device_id, record.sequence
        );
    }

    // Forward: pass records and payloads to cloud backend input files, unchanged.
    std::fs::write(OUT_RECORDS, serde_json::to_string_pretty(&records).unwrap()).unwrap();
    std::fs::write(OUT_PAYLOADS, serde_json::to_string_pretty(&payloads).unwrap()).unwrap();

    println!(
        "EDGE GATEWAY: forwarded {} record(s) → {}",
        records.len(),
        OUT_RECORDS
    );
}
