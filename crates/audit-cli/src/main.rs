use std::path::PathBuf;

use immutable_trace::{
    build_lift_inspection_demo_records, parse_fixed_hex, sign_record, verify_chain_file,
    verify_chain_records, verify_record, write_record_json, write_records_json, AuditRecord,
};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "imt")]
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
        Commands::DemoLiftInspection {
            device_id,
            private_key_hex,
            start_timestamp_ms,
            object_prefix,
            out_file,
        } => {
            let records = build_lift_inspection_demo_records(
                &device_id,
                &private_key_hex,
                start_timestamp_ms,
                &object_prefix,
            )?;
            verify_chain_records(&records)?;
            write_records_json(&out_file, &records)?;
            println!("DEMO_CREATED:{}", out_file.display());
            println!("CHAIN_VALID");
        }
    }

    Ok(())
}
