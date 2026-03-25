//! `eds` — EdgeSentry unified CLI.
//!
//! # Usage
//!
//! ```
//! eds inspect scan --config config.toml
//! eds audit keygen
//! eds audit sign-record --device-id dev-01 ...
//! eds audit verify-chain --records-file records.json
//! ```

mod audit;
mod inspect;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "eds",
    about = "EdgeSentry CLI — IFC deviation analysis and tamper-evident audit trail",
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
        command: audit::AuditCommand,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Inspect { command } => inspect::run(command),
        Commands::Audit { command } => audit::run(command),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
