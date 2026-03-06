use device_agent::build_signed_record;
use ed25519_dalek::SigningKey;
use ingest_api::{InMemoryAuditLedger, InMemoryOperationLog, InMemoryRawDataStore, IngestState, IngestService};
use ledger_core::AuditRecord;

fn main() {
    let device_id = "lift-01";
    let signing_key = SigningKey::from_bytes(&[1u8; 32]);
    let verifying_key = signing_key.verifying_key();

    let verifier = IngestState::default();
    let raw_data_store = InMemoryRawDataStore::default();
    let audit_ledger = InMemoryAuditLedger::default();
    let operation_log = InMemoryOperationLog::default();

    let mut service = IngestService::new(verifier, raw_data_store, audit_ledger, operation_log);
    service.register_device(device_id, verifying_key);

    let checks = [
        ("door cycle: ok", "lift-01/inspection-1.bin"),
        ("vibration: ok", "lift-01/inspection-2.bin"),
        ("emergency brake: ok", "lift-01/inspection-3.bin"),
    ];

    let mut prev_hash = AuditRecord::zero_hash();

    for (index, (payload_text, object_key)) in checks.iter().enumerate() {
        let payload = payload_text.as_bytes();
        let record = build_signed_record(
            device_id,
            (index as u64) + 1,
            1_700_000_000_000 + (index as u64) * 1_000,
            payload,
            prev_hash,
            format!("s3://bucket/{object_key}"),
            &signing_key,
        );

        service
            .ingest(record.clone(), payload)
            .expect("expected accepted ingest in normal flow");
        prev_hash = record.hash();
    }

    let tampered_payload = b"vibration: tampered";
    let mut tampered_record = build_signed_record(
        device_id,
        4,
        1_700_000_004_000,
        tampered_payload,
        prev_hash,
        "s3://bucket/lift-01/inspection-4.bin",
        &signing_key,
    );
    tampered_record.payload_hash[0] ^= 0x01;

    let tampered_result = service.ingest(tampered_record, tampered_payload);
    assert!(tampered_result.is_err(), "tampered record must be rejected");

    println!("=== Demo Result ===");
    println!("accepted_records={}", service.audit_ledger().records().len());
    println!("operation_logs={}", service.operation_log().entries().len());
    println!();

    println!("=== Stored Audit Records ===");
    for record in service.audit_ledger().records() {
        println!(
            "device={} sequence={} object_ref={}",
            record.device_id, record.sequence, record.object_ref
        );
    }
    println!();

    println!("=== Operation Logs ===");
    for entry in service.operation_log().entries() {
        println!(
            "decision={:?} device={} sequence={} message={}",
            entry.decision, entry.device_id, entry.sequence, entry.message
        );
    }
}
