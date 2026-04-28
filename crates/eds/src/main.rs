//! `eds` — EdgeSentry unified CLI.
//!
//! # Usage
//!
//! ```
//! eds inspect scan --config config.toml
//! eds audit keygen
//! eds audit sign-record --device-id dev-01 ...
//! eds audit verify-chain --records-file records.json
//! eds ingest replay --source FILE --out FILE
//! eds profile validate --profile DIR
//! eds profile list --profile DIR
//! eds compute run --input FILE --out FILE
//! eds evaluate run --input FILE --profile DIR --out FILE
//! ```

mod audit;
mod inspect;
mod ingest;
mod profile;
mod compute;
mod evaluate;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "eds",
    about = "EdgeSentry CLI — IFC deviation analysis, tamper-evident audit trail, and safety evaluation",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// IFC deviation analysis, heatmap rendering, and field scan pipeline
    Inspect {
        #[command(subcommand)]
        command: inspect::InspectCommand,
    },
    /// Tamper-evident audit trail: sign, verify, and ingest records
    Audit {
        #[command(subcommand)]
        command: Box<audit::AuditCommand>,
    },
    /// Entity data ingestion — CSV replay and JSONL output
    Ingest {
        #[command(subcommand)]
        command: ingest::IngestCommand,
    },
    /// Profile management — validate and list rules
    Profile {
        #[command(subcommand)]
        command: profile::ProfileCommand,
    },
    /// Physics computations — distance, TTC, braking distance
    Compute {
        #[command(subcommand)]
        command: compute::ComputeCommand,
    },
    /// Rule evaluation — evaluate rules against entity frames
    Evaluate {
        #[command(subcommand)]
        command: evaluate::EvaluateCommand,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Inspect { command } => inspect::run(command),
        Commands::Audit { command } => audit::run(*command),
        Commands::Ingest { command } => ingest::run(command),
        Commands::Profile { command } => profile::run(command),
        Commands::Compute { command } => compute::run(command),
        Commands::Evaluate { command } => evaluate::run(command),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
