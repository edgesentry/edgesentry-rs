//! `eds evaluate` subcommands — rule evaluation over entity frames.

use clap::Subcommand;
use std::fs;
use std::path::PathBuf;

use edgesentry_evaluate::{evaluate, RiskEvent};
use edgesentry_ingest::csv_replay::EntityFrame;
use edgesentry_ingest::jsonl::{JsonlReader, JsonlWriter};
use edgesentry_profile::load_profile;

#[derive(Debug, Subcommand)]
pub enum EvaluateCommand {
    /// Evaluate rules against EntityFrame JSONL and write RiskEvent JSONL.
    Run {
        /// Input EntityFrame JSONL file.
        #[arg(long)]
        input: PathBuf,

        /// Profile directory containing rules.json.
        #[arg(long)]
        profile: PathBuf,

        /// Output RiskEvent JSONL file.
        #[arg(long)]
        out: PathBuf,
    },
}

pub fn run(cmd: EvaluateCommand) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        EvaluateCommand::Run { input, profile, out } => run_evaluate(input, profile, out),
    }
}

fn run_evaluate(input: PathBuf, profile: PathBuf, out: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let rules = load_profile(&profile)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    let file = fs::File::open(&input)?;
    let mut reader = JsonlReader::open(file)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    let frames: Vec<EntityFrame> = reader
        .records()
        .collect::<Result<Vec<_>, String>>()
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    let out_file = fs::File::create(&out)?;
    let mut writer = JsonlWriter::new(out_file, "eds.risk-event", "0.1")
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    let mut total_events = 0usize;
    for frame in &frames {
        let events: Vec<RiskEvent> =
            evaluate(&rules, &frame.entities, frame.timestamp_ms);
        for event in &events {
            writer.write_record(event)
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            total_events += 1;
        }
    }

    eprintln!(
        "evaluate run: {} event(s) from {} frame(s) written to {}",
        total_events,
        frames.len(),
        out.display()
    );
    Ok(())
}
