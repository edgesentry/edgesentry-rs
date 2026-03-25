//! `edgesentry-inspect` — field PC CLI for IFC-based deviation detection.
//!
//! # Usage
//!
//! ```sh
//! edgesentry-inspect scan --config config.toml
//! ```

use clap::{Parser, Subcommand};

use edgesentry_inspect::{
    config::load_config,
    pipeline::run_scan,
};

#[derive(Parser)]
#[command(
    name = "edgesentry-inspect",
    about = "Edge-first 3D scan vs. IFC deviation detection",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a full scan: load IFC + PLY, compute deviation, render heatmap.
    Scan {
        /// Path to the TOML configuration file.
        #[arg(short, long, default_value = "config.toml")]
        config: std::path::PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan { config } => {
            let cfg = match load_config(&config) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("error: failed to load config '{}': {e}", config.display());
                    std::process::exit(1);
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
                }
                Err(e) => {
                    eprintln!("error: scan failed: {e}");
                    std::process::exit(1);
                }
            }
        }
    }
}
