//! `eds audit` subcommands — tamper-evident audit trail operations.

use std::path::PathBuf;

use clap::Subcommand;
use edgesentry_audit::{
    build_lift_inspection_demo_records_with_payloads, generate_keypair, inspect_key,
    parse_fixed_hex, sign_record, verify_chain_file, verify_chain_records, verify_record,
    write_record_json, write_records_json, AuditRecord,
};

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
    }

    Ok(())
}
