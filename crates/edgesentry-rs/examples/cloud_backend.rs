// CLOUD BACKEND
//
// Receives forwarded records from the edge gateway and enforces the full
// ingest pipeline:
//
//   1. NetworkPolicy::check   — deny-by-default IP/CIDR allowlist (CLS-06)
//   2. IngestService::ingest  — route identity → signature → sequence → hash-chain
//   3. Persistence            — raw payloads + audit ledger + operation log
//
// Storage backend:
//   default             — in-memory (no external dependencies)
//   --features s3,postgres — PostgreSQL audit ledger + MinIO raw-data store
//
// Run (in-memory):
//   cargo run -p edgesentry-rs --example cloud_backend
//
// Run (PostgreSQL + MinIO — requires local_demo.sh step 1 to have completed):
//   cargo run -p edgesentry-rs --features s3,postgres --example cloud_backend
//
// Environment variables (s3,postgres mode only):
//   PG_URL           postgresql://trace:trace@localhost:5433/trace_audit
//   MINIO_ENDPOINT   http://localhost:9000
//   MINIO_BUCKET     bucket
//   MINIO_ACCESS_KEY minioadmin
//   MINIO_SECRET_KEY minioadmin

use std::net::IpAddr;

use ed25519_dalek::SigningKey;
use edgesentry_rs::{
    AuditLedger, AuditRecord, IngestService, IntegrityPolicyGate, NetworkPolicy,
    OperationLogStore, RawDataStore,
};
#[cfg(not(all(feature = "s3", feature = "postgres")))]
use edgesentry_rs::{InMemoryAuditLedger, InMemoryOperationLog, InMemoryRawDataStore};

const IN_RECORDS: &str = "/tmp/eds_fwd_records.json";
const IN_PAYLOADS: &str = "/tmp/eds_fwd_payloads.json";
const TAMPERED_FILE: &str = "/tmp/eds_tampered.json";
const DEVICE_ID: &str = "lift-01";
const GATEWAY_IP: &str = "10.0.1.42";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let records: Vec<AuditRecord> = serde_json::from_str(
        &std::fs::read_to_string(IN_RECORDS)
            .unwrap_or_else(|_| panic!("run edge_gateway first — {IN_RECORDS} not found")),
    )?;
    let payload_hexes: Vec<String> = serde_json::from_str(
        &std::fs::read_to_string(IN_PAYLOADS)
            .unwrap_or_else(|_| panic!("run edge_gateway first — {IN_PAYLOADS} not found")),
    )?;
    let tampered: Vec<AuditRecord> = serde_json::from_str(
        &std::fs::read_to_string(TAMPERED_FILE)
            .unwrap_or_else(|_| panic!("run edge_device first — {TAMPERED_FILE} not found")),
    )?;

    let payloads: Vec<Vec<u8>> = payload_hexes
        .iter()
        .map(|h| hex::decode(h).unwrap())
        .collect();

    // The cloud has the device's public key registered at device provisioning time.
    let verifying_key = SigningKey::from_bytes(&[1u8; 32]).verifying_key();

    // Deny-by-default network policy (CLS-06): only allow the known gateway subnet.
    let mut network_policy = NetworkPolicy::new();
    network_policy.allow_cidr("10.0.1.0/24")?;
    let gateway_ip: IpAddr = GATEWAY_IP.parse()?;
    println!("CLOUD BACKEND: NetworkPolicy — allow 10.0.1.0/24, deny all others");

    let mut policy = IntegrityPolicyGate::new();
    policy.register_device(DEVICE_ID, verifying_key);

    #[cfg(all(feature = "s3", feature = "postgres"))]
    {
        use edgesentry_rs::{
            PostgresAuditLedger, PostgresOperationLog, S3CompatibleRawDataStore, S3ObjectStoreConfig,
        };

        let pg_url = std::env::var("PG_URL")
            .unwrap_or_else(|_| "postgresql://trace:trace@localhost:5433/trace_audit".into());
        let minio_endpoint = std::env::var("MINIO_ENDPOINT")
            .unwrap_or_else(|_| "http://localhost:9000".into());
        let minio_bucket =
            std::env::var("MINIO_BUCKET").unwrap_or_else(|_| "bucket".into());
        let minio_access_key =
            std::env::var("MINIO_ACCESS_KEY").unwrap_or_else(|_| "minioadmin".into());
        let minio_secret_key =
            std::env::var("MINIO_SECRET_KEY").unwrap_or_else(|_| "minioadmin".into());

        println!("CLOUD BACKEND: storage — PostgreSQL ({pg_url}) + MinIO ({minio_endpoint})");

        let audit_ledger = PostgresAuditLedger::connect(&pg_url)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        let mut operation_log = PostgresOperationLog::connect(&pg_url)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        operation_log
            .reset()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        println!("CLOUD BACKEND: tables reset");

        let raw_data_store = S3CompatibleRawDataStore::new(S3ObjectStoreConfig::for_minio(
            &minio_bucket,
            "us-east-1",
            &minio_endpoint,
            &minio_access_key,
            &minio_secret_key,
        ))
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

        let mut service =
            IngestService::new(policy, raw_data_store, audit_ledger, operation_log);
        run_ingest(&network_policy, gateway_ip, &mut service, &records, &payloads, &tampered)?;
    }

    #[cfg(not(all(feature = "s3", feature = "postgres")))]
    {
        println!("CLOUD BACKEND: storage — in-memory (re-run with --features s3,postgres for PostgreSQL + MinIO)");
        let mut service = IngestService::new(
            policy,
            InMemoryRawDataStore::default(),
            InMemoryAuditLedger::default(),
            InMemoryOperationLog::default(),
        );
        run_ingest(&network_policy, gateway_ip, &mut service, &records, &payloads, &tampered)?;
    }

    Ok(())
}

fn run_ingest<R, A, O>(
    network_policy: &NetworkPolicy,
    gateway_ip: IpAddr,
    service: &mut IngestService<R, A, O>,
    records: &[AuditRecord],
    payloads: &[Vec<u8>],
    tampered: &[AuditRecord],
) -> Result<(), Box<dyn std::error::Error>>
where
    R: RawDataStore,
    A: AuditLedger,
    O: OperationLogStore,
{
    // Valid records
    println!("CLOUD BACKEND: --- ingesting valid records ---");
    for (record, payload) in records.iter().zip(payloads) {
        network_policy
            .check(gateway_ip)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        match service.ingest(record.clone(), payload, Some(DEVICE_ID)) {
            Ok(()) => println!(
                "CLOUD BACKEND: ACCEPTED  device={} seq={}",
                record.device_id, record.sequence
            ),
            Err(e) => println!(
                "CLOUD BACKEND: REJECTED  device={} seq={} reason={e}",
                record.device_id, record.sequence
            ),
        }
    }

    // Tampered records — must all be rejected
    println!("CLOUD BACKEND: --- tamper detection (expect all REJECTED) ---");
    for (record, payload) in tampered.iter().zip(payloads) {
        network_policy
            .check(gateway_ip)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        match service.ingest(record.clone(), payload, Some(DEVICE_ID)) {
            Ok(()) => println!(
                "CLOUD BACKEND: ACCEPTED  (unexpected!) device={} seq={}",
                record.device_id, record.sequence
            ),
            Err(e) => println!(
                "CLOUD BACKEND: REJECTED  (expected)    device={} seq={} reason={e}",
                record.device_id, record.sequence
            ),
        }
    }

    Ok(())
}
