use clap::Subcommand;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use edgesentry_scenario::{generate_entity_csv, simulate_from_csv, ScenarioConfig};

#[derive(Debug, Subcommand)]
pub enum ScenarioCommand {
    /// Generate a synthetic entity CSV scenario.
    Generate {
        /// Scenario type (currently only "entity" is supported).
        #[arg(long, default_value = "entity")]
        scenario_type: String,
        /// Number of entities to simulate.
        #[arg(long, default_value = "2")]
        entities: usize,
        /// Number of frames to generate.
        #[arg(long, default_value = "10")]
        frames: usize,
        /// Random seed for reproducible output.
        #[arg(long, default_value = "0")]
        seed: u64,
        /// Output file path for the generated CSV.
        #[arg(long)]
        out: PathBuf,
    },
    /// Simulate a scenario by sending entity frames over UDP.
    Simulate {
        /// Path to source CSV file.
        #[arg(long)]
        source: PathBuf,
        /// Target address, e.g. udp://127.0.0.1:4200.
        #[arg(long)]
        target: String,
        /// Frames per second.
        #[arg(long, default_value = "10")]
        fps: u32,
    },
}

pub fn run(cmd: ScenarioCommand) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        ScenarioCommand::Generate { scenario_type: _, entities, frames, seed, out } => {
            run_generate(entities, frames, seed, &out)
        }
        ScenarioCommand::Simulate { source, target, fps } => {
            run_simulate(&source, &target, fps)
        }
    }
}

fn run_generate(
    entity_count: usize,
    frame_count: usize,
    seed: u64,
    out: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = ScenarioConfig {
        entity_count,
        frame_count,
        fps: 10,
        bounds: 20.0,
    };
    let csv = generate_entity_csv(&config, seed);
    fs::write(out, &csv)?;
    eprintln!(
        "scenario generate: {} entity(s) × {} frame(s) → {}",
        entity_count, frame_count, out.display()
    );
    Ok(())
}

fn run_simulate(
    source: &Path,
    target: &str,
    fps: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let csv = fs::read_to_string(source)?;
    // Strip the "udp://" prefix if present.
    let addr = target.strip_prefix("udp://").unwrap_or(target);
    let frames_sent = simulate_from_csv(&csv, addr, fps)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    eprintln!("scenario simulate: {} frame(s) sent to {}", frames_sent, target);
    Ok(())
}
