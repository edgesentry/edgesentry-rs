use std::path::PathBuf;

use clap::{Parser, Subcommand};
use edgesentry_rs::{
    build_lift_inspection_demo_records_with_payloads, generate_keypair, inspect_key,
    parse_fixed_hex, sign_record, verify_chain_file, verify_chain_records, verify_record,
    write_record_json, write_records_json, AuditRecord,
};

#[derive(Debug, Parser)]
#[command(name = "eds")]
#[command(about = "CLI tools for tamper-evident audit records")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    SignRecord {
        #[arg(long)]
        device_id: String,
        #[arg(long)]
        sequence: u64,
        #[arg(long)]
        timestamp_ms: u64,
        #[arg(long)]
        payload: String,
        #[arg(long, default_value = "0000000000000000000000000000000000000000000000000000000000000000")]
        prev_hash_hex: String,
        #[arg(long)]
        object_ref: String,
        #[arg(long)]
        private_key_hex: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    VerifyRecord {
        #[arg(long)]
        record_file: PathBuf,
        #[arg(long)]
        public_key_hex: String,
    },
    VerifyChain {
        #[arg(long)]
        records_file: PathBuf,
    },
    DemoLiftInspection {
        #[arg(long, default_value = "lift-01")]
        device_id: String,
        #[arg(
            long,
            default_value = "0101010101010101010101010101010101010101010101010101010101010101"
        )]
        private_key_hex: String,
        #[arg(long, default_value_t = 1_700_000_000_000)]
        start_timestamp_ms: u64,
        #[arg(long, default_value = "s3://bucket/lift-01")]
        object_prefix: String,
        #[arg(long, default_value = "lift_inspection_records.json")]
        out_file: PathBuf,
        /// Optional: write raw payloads as a JSON array of hex strings (required for demo-ingest)
        #[arg(long)]
        payloads_file: Option<PathBuf>,
    },
    /// Generate a fresh Ed25519 keypair; prints JSON with private_key_hex and public_key_hex
    Keygen {
        /// Write output to this file instead of stdout
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Derive the public key from an existing private key hex
    InspectKey {
        #[arg(long)]
        private_key_hex: String,
        /// Write output to this file instead of stdout
        #[arg(long)]
        out: Option<PathBuf>,
    },
    #[cfg(all(feature = "s3", feature = "postgres"))]
    /// Ingest records through IngestService into PostgreSQL + MinIO (requires s3,postgres features)
    DemoIngest {
        #[arg(long)]
        records_file: PathBuf,
        #[arg(long)]
        payloads_file: PathBuf,
        #[arg(long, default_value = "lift-01")]
        device_id: String,
        #[arg(
            long,
            default_value = "0101010101010101010101010101010101010101010101010101010101010101"
        )]
        private_key_hex: String,
        #[arg(long, default_value = "postgresql://trace:trace@localhost:5433/trace_audit")]
        pg_url: String,
        #[arg(long, default_value = "http://localhost:9000")]
        minio_endpoint: String,
        #[arg(long, default_value = "bucket")]
        minio_bucket: String,
        #[arg(long, default_value = "minioadmin")]
        minio_access_key: String,
        #[arg(long, default_value = "minioadmin")]
        minio_secret_key: String,
        /// Truncate audit_records and operation_logs before ingesting
        #[arg(long, default_value_t = false)]
        reset: bool,
        /// Optional: also try ingesting these records to demonstrate rejection
        #[arg(long)]
        tampered_records_file: Option<PathBuf>,
    },
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::SignRecord {
            device_id,
            sequence,
            timestamp_ms,
            payload,
            prev_hash_hex,
            object_ref,
            private_key_hex,
            out,
        } => {
            let prev_hash = parse_fixed_hex::<32>(&prev_hash_hex)?;
            let record = sign_record(
                device_id,
                sequence,
                timestamp_ms,
                payload.into_bytes(),
                prev_hash,
                object_ref,
                &private_key_hex,
            )?;
            write_record_json(out.as_deref(), &record)?;
        }
        Commands::VerifyRecord {
            record_file,
            public_key_hex,
        } => {
            let content = std::fs::read_to_string(record_file)?;
            let record: AuditRecord = serde_json::from_str(&content)?;
            let valid = verify_record(&record, &public_key_hex)?;
            if valid {
                println!("VALID");
            } else {
                println!("INVALID");
                std::process::exit(2);
            }
        }
        Commands::VerifyChain { records_file } => {
            verify_chain_file(&records_file)?;
            println!("CHAIN_VALID");
        }
        Commands::Keygen { out } => {
            let keypair = generate_keypair();
            let json = serde_json::to_string_pretty(&keypair)?;
            match out {
                Some(path) => std::fs::write(&path, &json)?,
                None => println!("{json}"),
            }
        }
        Commands::InspectKey { private_key_hex, out } => {
            let keypair = inspect_key(&private_key_hex)?;
            let json = serde_json::to_string_pretty(&keypair)?;
            match out {
                Some(path) => std::fs::write(&path, &json)?,
                None => println!("{json}"),
            }
        }
        Commands::DemoLiftInspection {
            device_id,
            private_key_hex,
            start_timestamp_ms,
            object_prefix,
            out_file,
            payloads_file,
        } => {
            let pairs = build_lift_inspection_demo_records_with_payloads(
                &device_id,
                &private_key_hex,
                start_timestamp_ms,
                &object_prefix,
            )?;
            let records: Vec<AuditRecord> = pairs.iter().map(|(r, _)| r.clone()).collect();
            verify_chain_records(&records)?;
            write_records_json(&out_file, &records)?;
            if let Some(pf) = payloads_file {
                let hexes: Vec<String> =
                    pairs.iter().map(|(_, p)| hex::encode(p)).collect();
                std::fs::write(&pf, serde_json::to_string_pretty(&hexes)?)?;
            }
            println!("DEMO_CREATED:{}", out_file.display());
            println!("CHAIN_VALID");
        }
        #[cfg(all(feature = "s3", feature = "postgres"))]
        Commands::DemoIngest {
            records_file,
            payloads_file,
            device_id,
            private_key_hex,
            pg_url,
            minio_endpoint,
            minio_bucket,
            minio_access_key,
            minio_secret_key,
            reset,
            tampered_records_file,
        } => {
            use ed25519_dalek::SigningKey;
            use edgesentry_rs::{
                IngestService, IntegrityPolicyGate, PostgresAuditLedger, PostgresOperationLog,
                S3CompatibleRawDataStore, S3ObjectStoreConfig,
            };

            // Derive verifying key from private key
            let key_bytes = parse_fixed_hex::<32>(&private_key_hex)?;
            let verifying_key = SigningKey::from_bytes(&key_bytes).verifying_key();

            // Connect to PostgreSQL
            let audit_ledger = PostgresAuditLedger::connect(&pg_url)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            let mut operation_log = PostgresOperationLog::connect(&pg_url)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

            if reset {
                operation_log
                    .reset()
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
                println!("RESET: audit_records and operation_logs truncated");
            }

            // Connect to MinIO
            let raw_data_store = S3CompatibleRawDataStore::new(
                S3ObjectStoreConfig::for_minio(
                    &minio_bucket,
                    "us-east-1",
                    &minio_endpoint,
                    &minio_access_key,
                    &minio_secret_key,
                ),
            )
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

            // Build IngestService
            let mut policy = IntegrityPolicyGate::new();
            policy.register_device(&device_id, verifying_key);
            let mut service =
                IngestService::new(policy, raw_data_store, audit_ledger, operation_log);

            // Load records and payloads
            let records: Vec<AuditRecord> =
                serde_json::from_str(&std::fs::read_to_string(&records_file)?)?;
            let payload_hexes: Vec<String> =
                serde_json::from_str(&std::fs::read_to_string(&payloads_file)?)?;

            // Ingest valid records
            let mut accepted = 0usize;
            let mut rejected = 0usize;
            for (record, payload_hex) in records.iter().zip(payload_hexes.iter()) {
                let payload =
                    hex::decode(payload_hex).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
                match service.ingest(record.clone(), &payload, Some(&device_id)) {
                    Ok(()) => {
                        accepted += 1;
                        println!("ACCEPTED: device={} sequence={}", record.device_id, record.sequence);
                    }
                    Err(e) => {
                        rejected += 1;
                        println!("REJECTED: device={} sequence={} reason={}", record.device_id, record.sequence, e);
                    }
                }
            }

            // Optionally demonstrate rejection with tampered records
            if let Some(tampered_file) = tampered_records_file {
                let tampered: Vec<AuditRecord> =
                    serde_json::from_str(&std::fs::read_to_string(&tampered_file)?)?;
                // Use the original payloads — the tampered hash won't match, triggering rejection
                for (record, payload_hex) in tampered.iter().zip(payload_hexes.iter()) {
                    let payload = hex::decode(payload_hex)
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
                    match service.ingest(record.clone(), &payload, Some(&device_id)) {
                        Ok(()) => {
                            accepted += 1;
                            println!("ACCEPTED: device={} sequence={}", record.device_id, record.sequence);
                        }
                        Err(e) => {
                            rejected += 1;
                            println!("REJECTED (expected): device={} sequence={} reason={}", record.device_id, record.sequence, e);
                        }
                    }
                }
            }

            println!("INGEST_COMPLETE: accepted={accepted} rejected={rejected}");
        }
    }

    Ok(())
}
