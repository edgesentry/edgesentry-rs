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

    /// Download buildingSMART sample IFC files for offline use.
    ///
    /// Downloads the PCERT sample architecture model from the buildingSMART
    /// Sample-Test-Files repository and saves it locally. Files already present
    /// are skipped. After downloading, extract the mesh with:
    ///
    ///   eds inspect extract-mesh --ifc <DIR>/Building-Architecture.ifc \
    ///                             --out <DIR>/reference.json
    DownloadSamples {
        /// Directory to save downloaded IFC files (created if absent).
        #[arg(short, long, default_value = "ifc-samples")]
        dir: std::path::PathBuf,
    },

    /// Extract triangulated mesh from an IFC file via the IfcOpenShell Python sidecar.
    ///
    /// Prerequisite: pip install ifcopenshell
    ///
    /// Writes reference.json consumed by the Inspect App viewer for 3D overlay.
    /// Pass the output path as mesh_path in config.toml to include it in scan output.
    ExtractMesh {
        /// Input IFC file.
        #[arg(long)]
        ifc: std::path::PathBuf,
        /// Output reference.json path.
        #[arg(long)]
        out: std::path::PathBuf,
        /// Path to the Python sidecar script.
        #[arg(long, default_value = "scripts/extract_mesh.py")]
        script: std::path::PathBuf,
    },
}

// Verified URL — buildingSMART Sample-Test-Files repository (IFC 4, ~220 KB)
const SAMPLE_URL: &str = "https://raw.githubusercontent.com/buildingSMART/Sample-Test-Files/main/IFC%204.0.2.1%20(IFC%204)/PCERT-Sample-Scene/Building-Architecture.ifc";
const SAMPLE_FILENAME: &str = "Building-Architecture.ifc";

pub fn run(cmd: InspectCommand) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        InspectCommand::Scan { config } => {
            let cfg = load_config(&config)
                .map_err(|e| format!("failed to load config '{}': {e}", config.display()))?;

            println!("eds inspect scan");
            println!("  IFC   : {}", cfg.ifc_path.display());
            println!("  scan  : {}", cfg.scan_path.display());
            println!("  output: {}", cfg.output.dir.display());
            if let Some(m) = &cfg.mesh_path {
                println!("  mesh  : {}", m.display());
            }

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
            println!("  points  → {}", out.points_path.display());
            if let Some(m) = &out.reference_mesh_path {
                println!("  mesh    → {}", m.display());
            }
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

        InspectCommand::DownloadSamples { dir } => {
            use std::io::Write as _;

            std::fs::create_dir_all(&dir)?;

            let dest = dir.join(SAMPLE_FILENAME);
            if dest.exists() {
                println!("  {} already present, skipping", SAMPLE_FILENAME);
            } else {
                print!("  Downloading {} … ", SAMPLE_FILENAME);
                std::io::stdout().flush().ok();

                let resp = ureq::get(SAMPLE_URL)
                    .call()
                    .map_err(|e| format!("download failed: {e}"))?;
                let mut reader = resp.into_reader();
                let mut file = std::fs::File::create(&dest)?;
                let bytes = std::io::copy(&mut reader, &mut file)?;
                println!("done ({bytes} bytes)");
            }

            println!();
            println!("Saved to '{}'", dir.display());
            println!();
            println!("Next step — extract the mesh (requires: pip install ifcopenshell):");
            println!(
                "  eds inspect extract-mesh \\\n      --ifc {}/{} \\\n      --out {}/reference.json",
                dir.display(),
                SAMPLE_FILENAME,
                dir.display()
            );
        }

        InspectCommand::ExtractMesh { ifc, out, script } => {
            if !script.exists() {
                return Err(format!(
                    "Python sidecar not found at '{}'\n\
                     Run from the repository root, or pass --script <path>",
                    script.display()
                )
                .into());
            }

            let status = std::process::Command::new("python3")
                .arg(&script)
                .arg("--ifc")
                .arg(&ifc)
                .arg("--out")
                .arg(&out)
                .status()
                .map_err(|e| format!("failed to run python3: {e}"))?;

            if !status.success() {
                return Err("extract-mesh script exited with non-zero status".into());
            }

            println!();
            println!("Reference mesh written to '{}'", out.display());
            println!();
            println!("Add to your config.toml to include it in scan output:");
            println!("  mesh_path = \"{}\"", out.display());
        }
    }

    Ok(())
}
