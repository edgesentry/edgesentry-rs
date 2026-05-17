//! `eds audit` subcommands — tamper-evident audit trail operations.

use std::path::PathBuf;

use clap::Subcommand;
use edgesentry_audit::{
    build_lift_inspection_demo_records_with_payloads, generate_keypair, inspect_key,
    parse_fixed_hex, sign_record, verify_chain_file, verify_chain_records, verify_record,
    write_record_json, write_records_json, AuditRecord,
};
use edgesentry_document::{build_audit_payload, FilledDocument};
use edgesentry_evaluate::RiskEvent;
use edgesentry_ingest::jsonl::JsonlReader;
use serde::Serialize;

#[derive(Debug, Subcommand)]
pub enum AuditCommand {
    /// Sign a single audit record
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
    /// Verify a single audit record's signature
    VerifyRecord {
        #[arg(long)]
        record_file: PathBuf,
        #[arg(long)]
        public_key_hex: String,
    },
    /// Verify an entire hash-chained records file
    VerifyChain {
        #[arg(long)]
        records_file: PathBuf,
    },
    /// Generate a fresh Ed25519 keypair (prints JSON with private_key_hex and public_key_hex)
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
    /// Generate a lift-inspection demo: signed records + optional payloads file
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
    #[cfg(feature = "transport-http")]
    /// Start the HTTP ingest server (requires transport-http feature)
    Serve {
        #[arg(long, default_value = "0.0.0.0:8080")]
        addr: std::net::SocketAddr,
        #[arg(long, default_value = "127.0.0.1")]
        allowed_sources: String,
        #[arg(long = "device", value_name = "ID=PUBKEY_HEX")]
        devices: Vec<String>,
    },
    #[cfg(feature = "transport-tls")]
    /// Start the HTTPS ingest server with TLS 1.2/1.3 (requires transport-tls feature)
    ServeTls {
        #[arg(long, default_value = "0.0.0.0:8443")]
        addr: std::net::SocketAddr,
        #[arg(long, default_value = "127.0.0.1")]
        allowed_sources: String,
        #[arg(long = "device", value_name = "ID=PUBKEY_HEX")]
        devices: Vec<String>,
        #[arg(long)]
        tls_cert: PathBuf,
        #[arg(long)]
        tls_key: PathBuf,
    },
    #[cfg(feature = "transport-mqtt")]
    /// Start the MQTT ingest subscriber (requires transport-mqtt feature)
    ServeMqtt {
        #[arg(long, default_value = "localhost")]
        broker: String,
        #[arg(long, default_value_t = 1883)]
        port: u16,
        #[arg(long, default_value = "edgesentry/ingest")]
        topic: String,
        #[arg(long, default_value = "eds-server")]
        client_id: String,
        #[arg(long = "device", value_name = "ID=PUBKEY_HEX")]
        devices: Vec<String>,
        #[cfg(feature = "transport-mqtt-tls")]
        #[arg(long)]
        tls_ca_cert: Option<PathBuf>,
    },
    /// Export an ISO/IEC 42001 Annex A.4 evidence pack from an audit chain.
    ///
    /// Produces a control-mapped JSON bundle (and optional Markdown summary) aligned to
    /// A.4.2–A.4.6. This is evidence for a *customer's* AIMS audit — not an ISO 42001
    /// certificate for Edge Sentry Pte. Ltd.
    ExportAims {
        /// AuditRecord JSON array produced by `eds audit sign-record` / `sign-document`.
        #[arg(long)]
        chain: PathBuf,
        /// Optional RiskEvent JSONL for A.4.3 data-resource documentation.
        #[arg(long)]
        events: Option<PathBuf>,
        /// Optional profile directory for A.4.4 tooling documentation.
        #[arg(long)]
        profile_dir: Option<PathBuf>,
        /// Output path for the JSON evidence bundle.
        #[arg(long)]
        out: PathBuf,
        /// Optional path for a human-readable Markdown summary.
        #[arg(long)]
        md: Option<PathBuf>,
    },
    /// Sign a filled compliance document and produce a tamper-evident AuditRecord.
    ///
    /// Reads FilledDocument JSONL, builds a DocumentAuditPayload for each record,
    /// serialises it to canonical JSON bytes, signs with Ed25519, and writes an
    /// AuditRecord JSON array. Each document in the input produces one AuditRecord.
    /// Records are chained: if --chain is provided the last record's hash is used
    /// as prev_record_hash for the first new record.
    SignDocument {
        /// FilledDocument JSONL produced by `eds document fill`.
        #[arg(long)]
        payload: PathBuf,
        /// Ed25519 private key hex (32 bytes = 64 hex chars).
        #[arg(long)]
        key: String,
        /// Device / signer identifier written into the AuditRecord.
        #[arg(long, default_value = "eds-document")]
        device_id: String,
        /// Existing AuditRecord JSON array to continue chaining from (optional).
        #[arg(long)]
        chain: Option<PathBuf>,
        /// Output file for the new AuditRecord(s) JSON array.
        #[arg(long)]
        out: PathBuf,
    },
    /// Verify that a filled document matches its AuditRecord in the chain.
    ///
    /// Rebuilds the DocumentAuditPayload from FilledDocument JSONL, computes its
    /// BLAKE3 hash, and finds the matching AuditRecord in the chain. Prints a
    /// human-readable trace: voyage_id, template, fields, confidence, flagged
    /// fields, timestamp. Exits non-zero if no matching record is found.
    VerifyDocument {
        /// FilledDocument JSONL to verify (same file passed to sign-document).
        #[arg(long)]
        payload: PathBuf,
        /// AuditRecord JSON array to search (output of sign-document).
        #[arg(long)]
        chain: PathBuf,
    },
    #[cfg(all(feature = "s3", feature = "postgres"))]
    /// Ingest records into PostgreSQL + MinIO (requires s3,postgres features)
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
        #[arg(long, default_value_t = false)]
        reset: bool,
        #[arg(long)]
        tampered_records_file: Option<PathBuf>,
    },
}

pub fn run(cmd: AuditCommand) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        AuditCommand::SignRecord {
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

        AuditCommand::VerifyRecord { record_file, public_key_hex } => {
            let content = std::fs::read_to_string(record_file)?;
            let record: AuditRecord = serde_json::from_str(&content)?;
            if verify_record(&record, &public_key_hex)? {
                println!("VALID");
            } else {
                println!("INVALID");
                std::process::exit(2);
            }
        }

        AuditCommand::VerifyChain { records_file } => {
            verify_chain_file(&records_file)?;
            println!("CHAIN_VALID");
        }

        AuditCommand::Keygen { out } => {
            let keypair = generate_keypair();
            let json = serde_json::to_string_pretty(&keypair)?;
            match out {
                Some(path) => std::fs::write(&path, &json)?,
                None => println!("{json}"),
            }
        }

        AuditCommand::InspectKey { private_key_hex, out } => {
            let keypair = inspect_key(&private_key_hex)?;
            let json = serde_json::to_string_pretty(&keypair)?;
            match out {
                Some(path) => std::fs::write(&path, &json)?,
                None => println!("{json}"),
            }
        }

        AuditCommand::DemoLiftInspection {
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
                let hexes: Vec<String> = pairs.iter().map(|(_, p)| hex::encode(p)).collect();
                std::fs::write(&pf, serde_json::to_string_pretty(&hexes)?)?;
            }
            println!("DEMO_CREATED:{}", out_file.display());
            println!("CHAIN_VALID");
        }

        #[cfg(feature = "transport-http")]
        AuditCommand::Serve { addr, allowed_sources, devices } => {
            use ed25519_dalek::VerifyingKey;
            use edgesentry_audit::{
                AsyncInMemoryAuditLedger, AsyncInMemoryOperationLog, AsyncInMemoryRawDataStore,
                AsyncIngestService, IntegrityPolicyGate, NetworkPolicy,
            };

            let mut network_policy = NetworkPolicy::new();
            for source in allowed_sources.split(',') {
                let source = source.trim();
                if source.contains('/') {
                    network_policy
                        .allow_cidr(source)
                        .map_err(|e| format!("invalid CIDR '{source}': {e}"))?;
                } else {
                    let ip: std::net::IpAddr =
                        source.parse().map_err(|e| format!("invalid IP '{source}': {e}"))?;
                    network_policy.allow_ip(ip);
                }
            }

            let mut policy = IntegrityPolicyGate::new();
            for spec in &devices {
                let (device_id, pubkey_hex) = spec
                    .split_once('=')
                    .ok_or_else(|| format!("--device must be ID=PUBKEY_HEX, got: {spec}"))?;
                let key_bytes = parse_fixed_hex::<32>(pubkey_hex)?;
                let verifying_key = VerifyingKey::from_bytes(&key_bytes)
                    .map_err(|e| format!("invalid public key for {device_id}: {e}"))?;
                policy.register_device(device_id, verifying_key);
            }

            let service = AsyncIngestService::new(
                policy,
                AsyncInMemoryRawDataStore::default(),
                AsyncInMemoryAuditLedger::default(),
                AsyncInMemoryOperationLog::default(),
            );

            tokio::runtime::Runtime::new()?
                .block_on(edgesentry_audit::transport::http::serve(
                    service,
                    network_policy,
                    addr,
                ))
                .map_err(|e| format!("server error: {e}"))?;
        }

        #[cfg(feature = "transport-tls")]
        AuditCommand::ServeTls { addr, allowed_sources, devices, tls_cert, tls_key } => {
            use ed25519_dalek::VerifyingKey;
            use edgesentry_audit::{
                AsyncInMemoryAuditLedger, AsyncInMemoryOperationLog, AsyncInMemoryRawDataStore,
                AsyncIngestService, IntegrityPolicyGate, NetworkPolicy,
            };
            use edgesentry_audit::transport::tls::{TlsConfig, serve_tls};

            let mut network_policy = NetworkPolicy::new();
            for source in allowed_sources.split(',') {
                let source = source.trim();
                if source.contains('/') {
                    network_policy
                        .allow_cidr(source)
                        .map_err(|e| format!("invalid CIDR '{source}': {e}"))?;
                } else {
                    let ip: std::net::IpAddr =
                        source.parse().map_err(|e| format!("invalid IP '{source}': {e}"))?;
                    network_policy.allow_ip(ip);
                }
            }

            let mut policy = IntegrityPolicyGate::new();
            for spec in &devices {
                let (device_id, pubkey_hex) = spec
                    .split_once('=')
                    .ok_or_else(|| format!("--device must be ID=PUBKEY_HEX, got: {spec}"))?;
                let key_bytes = parse_fixed_hex::<32>(pubkey_hex)?;
                let verifying_key = VerifyingKey::from_bytes(&key_bytes)
                    .map_err(|e| format!("invalid public key for {device_id}: {e}"))?;
                policy.register_device(device_id, verifying_key);
            }

            let service = AsyncIngestService::new(
                policy,
                AsyncInMemoryRawDataStore::default(),
                AsyncInMemoryAuditLedger::default(),
                AsyncInMemoryOperationLog::default(),
            );

            let tls = TlsConfig::from_pem_files(tls_cert, tls_key);
            tokio::runtime::Runtime::new()?
                .block_on(serve_tls(service, network_policy, addr, tls))
                .map_err(|e| format!("TLS server error: {e}"))?;
        }

        #[cfg(feature = "transport-mqtt")]
        AuditCommand::ServeMqtt {
            broker,
            port,
            topic,
            client_id,
            devices,
            #[cfg(feature = "transport-mqtt-tls")]
            tls_ca_cert,
        } => {
            use ed25519_dalek::VerifyingKey;
            use edgesentry_audit::{
                AsyncInMemoryAuditLedger, AsyncInMemoryOperationLog, AsyncInMemoryRawDataStore,
                AsyncIngestService, IntegrityPolicyGate,
            };
            use edgesentry_audit::transport::mqtt::MqttIngestConfig;

            let mut policy = IntegrityPolicyGate::new();
            for spec in &devices {
                let (device_id, pubkey_hex) = spec
                    .split_once('=')
                    .ok_or_else(|| format!("--device must be ID=PUBKEY_HEX, got: {spec}"))?;
                let key_bytes = parse_fixed_hex::<32>(pubkey_hex)?;
                let verifying_key = VerifyingKey::from_bytes(&key_bytes)
                    .map_err(|e| format!("invalid public key for {device_id}: {e}"))?;
                policy.register_device(device_id, verifying_key);
            }

            let service = AsyncIngestService::new(
                policy,
                AsyncInMemoryRawDataStore::default(),
                AsyncInMemoryAuditLedger::default(),
                AsyncInMemoryOperationLog::default(),
            );

            let mut config = MqttIngestConfig::new(broker, topic, client_id);
            config.broker_port = port;

            #[cfg(feature = "transport-mqtt-tls")]
            if let Some(ca_cert) = tls_ca_cert {
                use edgesentry_audit::transport::mqtt::MqttTlsConfig;
                config.tls = Some(MqttTlsConfig::from_ca_cert_file(ca_cert));
            }

            tokio::runtime::Runtime::new()?
                .block_on(edgesentry_audit::transport::mqtt::serve_mqtt(config, service))
                .map_err(|e| format!("mqtt server error: {e}"))?;
        }

        #[cfg(all(feature = "s3", feature = "postgres"))]
        AuditCommand::DemoIngest {
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
            use edgesentry_audit::{
                IngestService, IntegrityPolicyGate, PostgresAuditLedger, PostgresOperationLog,
                S3CompatibleRawDataStore, S3ObjectStoreConfig,
            };

            let key_bytes = parse_fixed_hex::<32>(&private_key_hex)?;
            let verifying_key = SigningKey::from_bytes(&key_bytes).verifying_key();

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

            let raw_data_store = S3CompatibleRawDataStore::new(S3ObjectStoreConfig::for_minio(
                &minio_bucket,
                "us-east-1",
                &minio_endpoint,
                &minio_access_key,
                &minio_secret_key,
            ))
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

            let mut policy = IntegrityPolicyGate::new();
            policy.register_device(&device_id, verifying_key);
            let mut service =
                IngestService::new(policy, raw_data_store, audit_ledger, operation_log);

            let records: Vec<AuditRecord> =
                serde_json::from_str(&std::fs::read_to_string(&records_file)?)?;
            let payload_hexes: Vec<String> =
                serde_json::from_str(&std::fs::read_to_string(&payloads_file)?)?;

            let mut accepted = 0usize;
            let mut rejected = 0usize;
            for (record, payload_hex) in records.iter().zip(payload_hexes.iter()) {
                let payload = hex::decode(payload_hex)
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
                match service.ingest(record.clone(), &payload, Some(&device_id)) {
                    Ok(()) => {
                        accepted += 1;
                        println!(
                            "ACCEPTED: device={} sequence={}",
                            record.device_id, record.sequence
                        );
                    }
                    Err(e) => {
                        rejected += 1;
                        println!(
                            "REJECTED: device={} sequence={} reason={}",
                            record.device_id, record.sequence, e
                        );
                    }
                }
            }

            if let Some(tampered_file) = tampered_records_file {
                let tampered: Vec<AuditRecord> =
                    serde_json::from_str(&std::fs::read_to_string(&tampered_file)?)?;
                for (record, payload_hex) in tampered.iter().zip(payload_hexes.iter()) {
                    let payload = hex::decode(payload_hex)
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
                    match service.ingest(record.clone(), &payload, Some(&device_id)) {
                        Ok(()) => {
                            accepted += 1;
                            println!(
                                "ACCEPTED: device={} sequence={}",
                                record.device_id, record.sequence
                            );
                        }
                        Err(e) => {
                            rejected += 1;
                            println!(
                                "REJECTED (expected): device={} sequence={} reason={}",
                                record.device_id, record.sequence, e
                            );
                        }
                    }
                }
            }

            println!("INGEST_COMPLETE: accepted={accepted} rejected={rejected}");
        }

        AuditCommand::SignDocument { payload, key, device_id, chain, out } => {
            // Read existing chain to determine starting sequence and prev_hash.
            let (mut sequence, mut prev_hash) = if let Some(ref chain_path) = chain {
                let existing: Vec<AuditRecord> =
                    serde_json::from_str(&std::fs::read_to_string(chain_path)?)?;
                let seq = existing.last().map(|r| r.sequence + 1).unwrap_or(1);
                let ph = existing.last().map(|r| r.hash()).unwrap_or(AuditRecord::zero_hash());
                (seq, ph)
            } else {
                (1u64, AuditRecord::zero_hash())
            };

            // Read FilledDocument JSONL (skip schema header via JsonlReader).
            let file = std::fs::File::open(&payload)?;
            let mut reader = JsonlReader::open(file)
                .map_err(|e| format!("JSONL open: {e}"))?;
            let docs: Vec<FilledDocument> = reader
                .records()
                .collect::<Result<_, _>>()
                .map_err(|e| Box::<dyn std::error::Error>::from(format!("JSONL read: {e}")))?;

            let mut records: Vec<AuditRecord> = Vec::with_capacity(docs.len());
            let timestamp_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            for doc in &docs {
                let audit_payload = build_audit_payload(doc);
                let payload_bytes = serde_json::to_vec(&audit_payload)?;
                let object_ref = format!("document:{}/{}", doc.voyage_id, doc.template);
                let record = sign_record(
                    device_id.clone(),
                    sequence,
                    timestamp_ms,
                    payload_bytes,
                    prev_hash,
                    object_ref,
                    &key,
                )?;
                prev_hash = record.hash();
                sequence += 1;
                records.push(record);
            }

            write_records_json(&out, &records)?;
            println!("SIGNED: {} document record(s) written to {}", records.len(), out.display());
        }

            AuditCommand::ExportAims { chain, events, profile_dir, out, md } => {
            run_export_aims(&chain, events.as_deref(), profile_dir.as_deref(), &out, md.as_deref())?;
        }

        AuditCommand::VerifyDocument { payload, chain } => {
            // Read FilledDocument JSONL (skip schema header via JsonlReader).
            let file = std::fs::File::open(&payload)?;
            let mut reader = JsonlReader::open(file)
                .map_err(|e| format!("JSONL open: {e}"))?;
            let docs: Vec<FilledDocument> = reader
                .records()
                .collect::<Result<_, _>>()
                .map_err(|e| Box::<dyn std::error::Error>::from(format!("JSONL read: {e}")))?;

            // Read audit chain.
            let chain_content = std::fs::read_to_string(&chain)?;
            let records: Vec<AuditRecord> = serde_json::from_str(&chain_content)?;

            let mut found_any = false;
            for doc in &docs {
                let audit_payload = build_audit_payload(doc);
                let payload_bytes = serde_json::to_vec(&audit_payload)?;
                let payload_hash = *blake3::hash(&payload_bytes).as_bytes();

                let matched = records.iter().find(|r| r.payload_hash == payload_hash);
                match matched {
                    Some(record) => {
                        found_any = true;
                        println!("VERIFIED");
                        println!("  voyage_id:       {}", audit_payload.voyage_id);
                        println!("  template:        {}", audit_payload.template_id);
                        println!("  timestamp_ms:    {}", record.timestamp_ms);
                        println!("  sequence:        {}", record.sequence);
                        println!("  device_id:       {}", record.device_id);
                        println!("  review_required: {}", audit_payload.review_required);
                        if !audit_payload.fields_flagged.is_empty() {
                            println!("  fields_flagged:  {}", audit_payload.fields_flagged.join(", "));
                        }
                        println!("  field confidence:");
                        for (field, conf) in &audit_payload.confidence_flags {
                            let flag = if audit_payload.fields_flagged.contains(field) { " [FLAGGED]" } else { "" };
                            println!("    {:<30} {:.2}{}", field, conf, flag);
                        }
                        println!("  payload_hash:    {}", hex::encode(record.payload_hash));
                    }
                    None => {
                        eprintln!(
                            "NOT FOUND: no AuditRecord matches voyage_id={} template={}",
                            doc.voyage_id, doc.template
                        );
                        std::process::exit(2);
                    }
                }
            }

            if !found_any {
                eprintln!("NO DOCUMENTS: payload file contained no FilledDocument records");
                std::process::exit(2);
            }
        }
    }

    Ok(())
}

// ── ISO/IEC 42001 Annex A.4 evidence pack ────────────────────────────────────

#[derive(Serialize)]
struct AimsEvidencePack {
    generated_at_ms: u64,
    eds_version: &'static str,
    disclaimer: &'static str,
    a4_2_resource_documentation: A42ResourceDocumentation,
    a4_3_data_resources: A43DataResources,
    a4_4_tooling_resources: A44ToolingResources,
    a4_5_system_resources: A45SystemResources,
    a4_6_human_resources: A46HumanResources,
}

#[derive(Serialize)]
struct A42ResourceDocumentation {
    audit_chain_file: String,
    record_count: usize,
    device_ids: Vec<String>,
    timestamp_first_ms: Option<u64>,
    timestamp_last_ms: Option<u64>,
    chain_valid: bool,
    chain_error: Option<String>,
    object_ref_types: Vec<String>,
}

#[derive(Serialize)]
struct A43DataResources {
    data_sources_from_object_refs: Vec<String>,
    regulations_referenced: Vec<String>,
    risk_event_count: usize,
    event_rule_ids: Vec<String>,
}

#[derive(Serialize)]
struct A44ToolingResources {
    eds_version: &'static str,
    rules_engine_crate: &'static str,
    profile_dir: Option<String>,
    rule_count: Option<usize>,
    rule_ids: Vec<String>,
}

#[derive(Serialize)]
struct A45SystemResources {
    note: &'static str,
}

#[derive(Serialize)]
struct A46HumanResources {
    document_audit_record_count: usize,
    document_object_refs: Vec<String>,
    note: &'static str,
}

fn run_export_aims(
    chain_path: &std::path::Path,
    events_path: Option<&std::path::Path>,
    profile_dir: Option<&std::path::Path>,
    out_path: &std::path::Path,
    md_path: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let records: Vec<AuditRecord> =
        serde_json::from_str(&std::fs::read_to_string(chain_path)?)?;

    let chain_valid_result = verify_chain_records(&records);
    let (chain_valid, chain_error) = match &chain_valid_result {
        Ok(()) => (true, None),
        Err(e) => (false, Some(e.to_string())),
    };

    let (ts_first, ts_last) = {
        let mut tss: Vec<u64> = records.iter().map(|r| r.timestamp_ms).collect();
        tss.sort_unstable();
        (tss.first().copied(), tss.last().copied())
    };

    let mut device_ids: Vec<String> = records.iter().map(|r| r.device_id.clone()).collect();
    device_ids.sort();
    device_ids.dedup();

    let mut object_ref_types: Vec<String> = records
        .iter()
        .map(|r| {
            r.object_ref
                .split_once(':')
                .map(|(prefix, _)| prefix.to_string())
                .unwrap_or_else(|| r.object_ref.clone())
        })
        .collect();
    object_ref_types.sort();
    object_ref_types.dedup();

    let document_refs: Vec<String> = records
        .iter()
        .filter(|r| r.object_ref.starts_with("document:"))
        .map(|r| r.object_ref.clone())
        .collect();

    let data_sources: Vec<String> = records
        .iter()
        .map(|r| r.object_ref.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();

    let (risk_event_count, regulations, event_rule_ids) = if let Some(ep) = events_path {
        let file = std::fs::File::open(ep)?;
        let mut reader = JsonlReader::open(file)
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        let events: Vec<RiskEvent> = reader
            .records()
            .collect::<Result<_, _>>()
            .map_err(|e| -> Box<dyn std::error::Error> { format!("JSONL read: {e}").into() })?;

        let mut regs: Vec<String> =
            events.iter().map(|e| e.regulation.clone()).collect();
        regs.sort();
        regs.dedup();

        let mut rule_ids: Vec<String> =
            events.iter().map(|e| e.rule_id.clone()).collect();
        rule_ids.sort();
        rule_ids.dedup();

        (events.len(), regs, rule_ids)
    } else {
        (0, vec![], vec![])
    };

    let (profile_dir_str, rule_count, rule_ids_from_profile) =
        if let Some(pd) = profile_dir {
            match edgesentry_profile::load_profile(pd) {
                Ok(rules) => {
                    let mut ids: Vec<String> =
                        rules.iter().map(|r| r.rule_id.clone()).collect();
                    ids.sort();
                    (Some(pd.display().to_string()), Some(ids.len()), ids)
                }
                Err(e) => {
                    eprintln!("warning: could not load profile: {e}");
                    (Some(pd.display().to_string()), None, vec![])
                }
            }
        } else {
            (None, None, vec![])
        };

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let pack = AimsEvidencePack {
        generated_at_ms: now_ms,
        eds_version: env!("CARGO_PKG_VERSION"),
        disclaimer: "Control-aligned evidence for customer AIMS audits — \
            not an ISO/IEC 42001 certificate for Edge Sentry Pte. Ltd.",
        a4_2_resource_documentation: A42ResourceDocumentation {
            audit_chain_file: chain_path.display().to_string(),
            record_count: records.len(),
            device_ids,
            timestamp_first_ms: ts_first,
            timestamp_last_ms: ts_last,
            chain_valid,
            chain_error,
            object_ref_types,
        },
        a4_3_data_resources: A43DataResources {
            data_sources_from_object_refs: data_sources,
            regulations_referenced: regulations,
            risk_event_count,
            event_rule_ids,
        },
        a4_4_tooling_resources: A44ToolingResources {
            eds_version: env!("CARGO_PKG_VERSION"),
            rules_engine_crate: "edgesentry-evaluate",
            profile_dir: profile_dir_str,
            rule_count,
            rule_ids: rule_ids_from_profile,
        },
        a4_5_system_resources: A45SystemResources {
            note: "Per-inference compute logging is Phase 2 (edgesentry-rs #399). \
                Instrument `eds evaluate` with --resource-log to capture CPU%, RSS, and wall time.",
        },
        a4_6_human_resources: A46HumanResources {
            document_audit_record_count: document_refs.len(),
            document_object_refs: document_refs,
            note: "HITL review records are identified from object_refs prefixed 'document:' \
                in the audit chain. Payload content (review_required, fields_flagged) is \
                sealed at signing time.",
        },
    };

    let json = serde_json::to_string_pretty(&pack)?;
    std::fs::write(out_path, &json)?;
    println!("AIMS_EXPORT: {} record(s) → {}", pack.a4_2_resource_documentation.record_count, out_path.display());
    println!("  chain_valid: {}", pack.a4_2_resource_documentation.chain_valid);
    println!("  controls:    A.4.2 A.4.3 A.4.4 A.4.5 A.4.6");

    if let Some(mp) = md_path {
        let markdown = render_aims_markdown(&pack);
        std::fs::write(mp, &markdown)?;
        println!("  markdown:    {}", mp.display());
    }

    Ok(())
}

fn render_aims_markdown(pack: &AimsEvidencePack) -> String {
    let a42 = &pack.a4_2_resource_documentation;
    let a43 = &pack.a4_3_data_resources;
    let a44 = &pack.a4_4_tooling_resources;
    let a46 = &pack.a4_6_human_resources;

    let ts_range = match (a42.timestamp_first_ms, a42.timestamp_last_ms) {
        (Some(f), Some(l)) => format!("{f} – {l} ms"),
        _ => "—".to_string(),
    };
    let chain_status = if a42.chain_valid {
        "VALID".to_string()
    } else {
        format!("INVALID — {}", a42.chain_error.as_deref().unwrap_or("unknown error"))
    };

    let mut lines = vec![
        "# ISO/IEC 42001 Annex A.4 — AI Resource Governance Evidence Pack".to_string(),
        "".to_string(),
        format!("> **Disclaimer:** {}", pack.disclaimer),
        "".to_string(),
        format!("Generated: `{}` (eds v{})", pack.generated_at_ms, pack.eds_version),
        "".to_string(),
        "---".to_string(),
        "".to_string(),
        "## A.4.2 — Resource Documentation".to_string(),
        "".to_string(),
        format!("| Field | Value |"),
        format!("|---|---|"),
        format!("| Audit chain file | `{}` |", a42.audit_chain_file),
        format!("| Record count | {} |", a42.record_count),
        format!("| Device IDs | {} |", a42.device_ids.join(", ")),
        format!("| Timestamp range | {} |", ts_range),
        format!("| Chain integrity | {} |", chain_status),
        format!("| Object ref types | {} |", a42.object_ref_types.join(", ")),
        "".to_string(),
        "## A.4.3 — Data Resources".to_string(),
        "".to_string(),
    ];

    if a43.risk_event_count > 0 {
        lines.push(format!("**Risk events:** {}", a43.risk_event_count));
        lines.push("".to_string());
        if !a43.event_rule_ids.is_empty() {
            lines.push(format!("**Rule IDs triggered:** {}", a43.event_rule_ids.join(", ")));
            lines.push("".to_string());
        }
        if !a43.regulations_referenced.is_empty() {
            lines.push("**Regulations referenced:**".to_string());
            lines.push("".to_string());
            for reg in &a43.regulations_referenced {
                lines.push(format!("- {reg}"));
            }
            lines.push("".to_string());
        }
    } else {
        lines.push("_No RiskEvent JSONL provided. Pass `--events` to populate._".to_string());
        lines.push("".to_string());
    }

    if !a43.data_sources_from_object_refs.is_empty() {
        lines.push("**Data sources (from audit chain object_refs):**".to_string());
        lines.push("".to_string());
        for src in &a43.data_sources_from_object_refs {
            lines.push(format!("- `{src}`"));
        }
        lines.push("".to_string());
    }

    lines.push("## A.4.4 — Tooling Resources".to_string());
    lines.push("".to_string());
    lines.push("| Field | Value |".to_string());
    lines.push("|---|---|".to_string());
    lines.push(format!("| eds version | {} |", a44.eds_version));
    lines.push(format!("| Rules engine | `{}` |", a44.rules_engine_crate));
    if let Some(pd) = &a44.profile_dir {
        lines.push(format!("| Profile dir | `{pd}` |"));
    }
    if let Some(rc) = a44.rule_count {
        lines.push(format!("| Rule count | {rc} |"));
    }
    if !a44.rule_ids.is_empty() {
        lines.push(format!("| Rule IDs | {} |", a44.rule_ids.join(", ")));
    }
    lines.push("".to_string());

    lines.push("## A.4.5 — System and Computing Resources".to_string());
    lines.push("".to_string());
    lines.push(format!("> {}", pack.a4_5_system_resources.note));
    lines.push("".to_string());

    lines.push("## A.4.6 — Human Resources".to_string());
    lines.push("".to_string());
    lines.push(format!(
        "**HITL document records in chain:** {}",
        a46.document_audit_record_count
    ));
    lines.push("".to_string());
    if !a46.document_object_refs.is_empty() {
        for r in &a46.document_object_refs {
            lines.push(format!("- `{r}`"));
        }
        lines.push("".to_string());
    }
    lines.push(format!("> {}", a46.note));
    lines.push("".to_string());

    lines.join("\n")
}
