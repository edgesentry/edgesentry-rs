//! `eds inspect` subcommands — IFC-based deviation analysis.

use clap::Subcommand;
use edgesentry_inspect::{config::load_config, pipeline::run_scan};

#[derive(Debug, Subcommand)]
pub enum InspectCommand {
    /// Run a full scan: load IFC + PLY, compute deviation, render heatmap.
    ///
    /// Reads configuration from a TOML file (see config.example.toml).
    Scan {
        /// Path to the TOML configuration file.
        #[arg(short, long, default_value = "config.toml")]
        config: std::path::PathBuf,
    },
}

pub fn run(cmd: InspectCommand) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        InspectCommand::Scan { config } => {
            let cfg = load_config(&config)
                .map_err(|e| format!("failed to load config '{}': {e}", config.display()))?;

            println!("eds inspect scan");
            println!("  IFC   : {}", cfg.ifc_path.display());
            println!("  scan  : {}", cfg.scan_path.display());
            println!("  output: {}", cfg.output.dir.display());

            let out = run_scan(&cfg)?;

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
    }

    Ok(())
}
