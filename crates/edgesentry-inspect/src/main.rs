//! CLI entry point for `edgesentry-inspect`.
//!
//! This binary is a thin wrapper around [`edgesentry_inspect::run_scan`].
//! All pipeline logic lives in the library crate so it can be embedded
//! directly in the Tauri backend or any other Rust caller.
//!
//! Usage:
//!   edgesentry-inspect scan --config config.toml

use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};

use edgesentry_inspect::{
    config::load_config,
    run_scan,
};

#[derive(Parser)]
#[command(
    name = "edgesentry-inspect",
    about = "Edge-first 3D scan vs. reference deviation tool"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compare a scan against an IFC design and write report.json, heatmap.png, points.json.
    Scan {
        /// Path to the TOML configuration file (see config.example.toml).
        #[arg(long)]
        config: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Scan { config: config_path } => {
            let cfg = match load_config(&config_path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("error: could not load config from {}: {e}", config_path.display());
                    process::exit(1);
                }
            };

            println!("edgesentry-inspect scan");
            println!("  IFC   : {}", cfg.ifc_path.display());
            println!("  scan  : {}", cfg.scan_path.display());
            println!("  output: {}", cfg.output.dir.display());

            match run_scan(&cfg) {
                Ok(out) => {
                    println!("\n=== Deviation Report ===");
                    println!("  point_count      : {}", out.report.point_count);
                    println!("  compliant_pct    : {:.1}%", out.report.compliant_pct);
                    println!("  max_deviation_mm : {:.3} mm", out.report.max_deviation_mm);
                    println!("  mean_deviation_mm: {:.3} mm", out.report.mean_deviation_mm);
                    if out.detection_count > 0 {
                        println!("  ai_detections    : {}", out.detection_count);
                    }
                    println!("\n  report  → {}", out.report_path.display());
                    println!("  heatmap → {}", out.heatmap_path.display());
                    println!("  points  → {}", out.points_path.display());
                    if let Some(m) = &out.reference_mesh_path {
                        println!("  mesh    → {}", m.display());
                    }
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    process::exit(1);
                }
            }
        }
    }
}
