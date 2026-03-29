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
    /// Generate offline demo data: synthetic IFC wall, matching PLY scan, and config.toml.
    ///
    /// All files are created locally with no external dependencies.
    /// After generation, run the pipeline with:
    ///
    ///   eds inspect scan --config <DIR>/config.toml
    GenerateFixtures {
        /// Directory to write generated files into (created if absent).
        #[arg(short, long, default_value = "demo-data")]
        dir: std::path::PathBuf,
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

        InspectCommand::GenerateFixtures { dir } => {
            use edgesentry_inspect::fixtures::generate_fixtures;

            let summary = generate_fixtures(&dir)
                .map_err(|e| format!("failed to generate fixtures in '{}': {e}", dir.display()))?;

            println!("Generated demo data in '{}':", dir.display());
            println!(
                "  wall_slab.ifc      — 3 m × 2 m IFC reference wall ({} points)",
                summary.point_count
            );
            println!(
                "  wall_slab_scan.ply — matching scan with {} points displaced 20 mm (centre defect)",
                summary.defect_point_count
            );
            println!("  config.toml        — pre-configured for eds inspect scan");
            println!();
            println!("Run the pipeline:");
            println!(
                "  cd {} && eds inspect scan --config config.toml",
                dir.display()
            );
        }
    }

    Ok(())
}
