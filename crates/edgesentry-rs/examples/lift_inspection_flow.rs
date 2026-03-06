use ed25519_dalek::SigningKey;
use edgesentry_rs::{
    build_signed_record, AuditRecord, InMemoryAuditLedger, InMemoryOperationLog,
    InMemoryRawDataStore, IngestService, IngestState,
};

fn main() {
    let device_id = "lift-01";
    let signing_key = SigningKey::from_bytes(&[1u8; 32]);
    let verifying_key = signing_key.verifying_key();

    let mut service = IngestService::new(
        IngestState::default(),
        InMemoryRawDataStore::default(),
        InMemoryAuditLedger::default(),
        InMemoryOperationLog::default(),
    );
    service.register_device(device_id, verifying_key);

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

    for record in &records {
        service
            .ingest(record.clone(), payloads[record.sequence as usize - 1])
            .expect("ingest should succeed");
    }

    println!("Stored {} audit records", service.audit_ledger().records().len());

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

    let result = service.ingest(tampered, tampered_payload);
    assert!(result.is_err(), "tampered record should be rejected");
    println!("Tampered record rejected: {:?}", result.unwrap_err());

    println!("Operation log entries: {}", service.operation_log().entries().len());
    for entry in service.operation_log().entries() {
        println!("  {:?} device={} seq={} msg={}", entry.decision, entry.device_id, entry.sequence, entry.message);
    }
}
