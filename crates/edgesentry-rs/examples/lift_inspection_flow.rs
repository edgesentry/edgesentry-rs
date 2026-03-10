// Three-role model
// ─────────────────────────────────────────────────────────────────────────────
// EDGE DEVICE  — a sensor or actuator that signs inspection records.
//                In production this runs on embedded hardware (microcontroller,
//                single-board computer). It only needs `build_signed_record`.
//
// EDGE GATEWAY — optional middle tier (industrial PC, 5G gateway) that
//                forwards signed records from the device to the cloud over
//                HTTPS / MQTT. The gateway does NOT verify content; it only
//                routes. This OSS core does not implement the transport layer.
//
// CLOUD BACKEND — receives records, enforces network policy, checks identity
//                 and integrity, and persists accepted records to storage.
//                 Runs `NetworkPolicy`, `IntegrityPolicyGate`, `IngestService`.
// ─────────────────────────────────────────────────────────────────────────────

use std::net::IpAddr;

use ed25519_dalek::SigningKey;
use edgesentry_rs::{
    build_signed_record, AuditRecord, InMemoryAuditLedger, InMemoryOperationLog,
    InMemoryRawDataStore, IngestService, IntegrityPolicyGate, NetworkPolicy,
};

fn main() {
    // ── EDGE DEVICE ──────────────────────────────────────────────────────────
    // The device holds a unique Ed25519 signing key. The private key never
    // leaves the device; the matching public key is registered on the cloud.

    let device_id = "lift-01";
    let signing_key = SigningKey::from_bytes(&[1u8; 32]);
    let verifying_key = signing_key.verifying_key();

    // Simulated source IP of the gateway that forwards this device's records.
    let gateway_ip: IpAddr = "10.0.1.42".parse().unwrap();

    // ── CLOUD BACKEND — setup ────────────────────────────────────────────────
    // The cloud configures a NetworkPolicy (deny-by-default) and registers
    // the device's public key before any records arrive.

    let mut network_policy = NetworkPolicy::new();
    network_policy
        .allow_cidr("10.0.1.0/24")
        .expect("valid CIDR");

    let mut service = IngestService::new(
        IntegrityPolicyGate::default(),
        InMemoryRawDataStore::default(),
        InMemoryAuditLedger::default(),
        InMemoryOperationLog::default(),
    );
    service.register_device(device_id, verifying_key);

    // ── EDGE DEVICE — sign records ───────────────────────────────────────────
    // For each physical inspection event the device builds a signed record,
    // links it to the previous record's hash, and emits it toward the gateway.

    let payloads = [
        b"check=door,status=ok" as &[u8],
        b"check=vibration,status=ok",
        b"check=emergency_brake,status=ok",
    ];

    let mut prev_hash = AuditRecord::zero_hash();
    let mut records = Vec::new();

    for (index, payload) in payloads.iter().enumerate() {
        let sequence = (index as u64) + 1;
        let record = build_signed_record(
            device_id,
            sequence,
            1_700_000_000_000 + sequence,
            payload,
            prev_hash,
            format!("s3://bucket/{device_id}/inspection-{sequence}.bin"),
            &signing_key,
        );
        prev_hash = record.hash();
        records.push(record);
    }

    // ── CLOUD BACKEND — ingest ───────────────────────────────────────────────
    // For each record arriving from the gateway the cloud backend:
    //   1. Checks the source IP against NetworkPolicy (CLS-06)
    //   2. Passes the record to IngestService which runs IntegrityPolicyGate:
    //      route identity → signature → sequence → hash-chain continuity

    for record in &records {
        // Step 1: network gate — runs before any cryptographic check.
        network_policy
            .check(gateway_ip)
            .expect("gateway IP should be allowed");

        // Step 2: integrity gate + persistence.
        service
            .ingest(record.clone(), payloads[record.sequence as usize - 1], Some(device_id))
            .expect("ingest should succeed");
    }

    println!("Stored {} audit records", service.audit_ledger().records().len());

    // ── CLOUD BACKEND — tamper rejection demo ────────────────────────────────
    // A tampered record (payload hash flipped) must be rejected by the gate.

    let tampered_payload = b"tampered";
    let tampered_record = build_signed_record(
        device_id,
        4,
        1_700_000_000_004,
        tampered_payload,
        prev_hash,
        "s3://bucket/lift-01/inspection-4.bin",
        &signing_key,
    );
    let mut tampered = tampered_record;
    tampered.payload_hash[0] ^= 0x01;

    // Network gate still passes (same gateway IP).
    network_policy.check(gateway_ip).expect("gateway IP should be allowed");

    // Integrity gate rejects the tampered record.
    let result = service.ingest(tampered, tampered_payload, Some(device_id));
    assert!(result.is_err(), "tampered record should be rejected");
    println!("Tampered record rejected: {:?}", result.unwrap_err());

    println!("Operation log entries: {}", service.operation_log().entries().len());
    for entry in service.operation_log().entries() {
        println!(
            "  {:?} device={} seq={} msg={}",
            entry.decision, entry.device_id, entry.sequence, entry.message
        );
    }
}
