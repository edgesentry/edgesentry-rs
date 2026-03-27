//! CLI entry point for `edgesentry-inspect`.
//!
//! Usage:
//!   edgesentry-inspect scan --config config.toml

use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};

use edgesentry_inspect::{config::load_config, pipeline::run_scan};

#[derive(Parser)]
#[command(name = "edgesentry-inspect", about = "Edge-first 3D scan vs. reference deviation tool")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run a scan: load IFC + PLY, compute deviation, write report.json, heatmap.png, points.json.
    Scan {
        /// Path to the TOML configuration file.
        #[arg(long)]
        config: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Scan { config } => {
            let cfg = match load_config(&config) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("error: failed to load config: {e}");
                    process::exit(1);
                }
            };

            match run_scan(&cfg) {
                Ok(out) => {
                    println!("Scan complete.");
                    println!("  points      : {}", out.report.point_count);
                    println!("  compliant   : {:.1}%", out.report.compliant_pct);
                    println!("  max dev     : {:.2} mm", out.report.max_deviation_mm);
                    println!("  mean dev    : {:.2} mm", out.report.mean_deviation_mm);
                    println!("  threshold   : {:.1} mm", out.report.threshold_mm);
                    println!("  detections  : {}", out.detection_count);
                    println!("  report      : {}", out.report_path.display());
                    println!("  heatmap     : {}", out.heatmap_path.display());
                    println!("  points.json : {}", out.points_path.display());
                }
                Err(e) => {
                    eprintln!("error: scan failed: {e}");
                    process::exit(1);
                }
            }
        }
    }
}
