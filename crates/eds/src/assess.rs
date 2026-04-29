//! `eds assess` subcommands — trend and correlation analysis over RiskEvents.

use clap::Subcommand;
use std::fs;
use std::path::PathBuf;

use edgesentry_assess::assess;
use edgesentry_evaluate::RiskEvent;
use edgesentry_ingest::jsonl::{JsonlReader, JsonlWriter};

#[derive(Debug, Subcommand)]
pub enum AssessCommand {
    /// Analyse RiskEvent JSONL for trends and correlations.
    Run {
        /// Input RiskEvent JSONL file.
        #[arg(long)]
        input: PathBuf,

        /// Output Assessment JSONL file.
        #[arg(long)]
        out: PathBuf,

        /// Additional RiskEvent JSONL history files to merge with input.
        #[arg(long)]
        history: Vec<PathBuf>,

        /// Time window in seconds; only events within this window are analysed.
        #[arg(long)]
        window_sec: Option<u64>,
    },
}

pub fn run(cmd: AssessCommand) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        AssessCommand::Run { input, out, history, window_sec } => {
            run_assess(&input, &out, &history, window_sec)
        }
    }
}

fn read_risk_events(path: &PathBuf) -> Result<Vec<RiskEvent>, Box<dyn std::error::Error>> {
    let file = fs::File::open(path)?;
    let mut reader = JsonlReader::open(file)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let events: Vec<RiskEvent> = reader
        .records()
        .collect::<Result<Vec<_>, String>>()
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    Ok(events)
}

fn run_assess(
    input: &PathBuf,
    out: &PathBuf,
    history: &[PathBuf],
    window_sec: Option<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut all_events = read_risk_events(input)?;

    for hist_path in history {
        let hist_events = read_risk_events(hist_path)?;
        all_events.extend(hist_events);
    }

    let assessment = assess(&all_events, window_sec);

    let out_file = fs::File::create(out)?;
    let mut writer = JsonlWriter::new(out_file, "eds.assessment", "0.1")
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    writer.write_record(&assessment)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    eprintln!(
        "assess run: {} event(s) analysed, {} repeated rule(s), {} correlated entity pair(s), trend={:?} → {}",
        assessment.event_count,
        assessment.repeated_rules.len(),
        assessment.correlated_entities.len(),
        assessment.trend,
        out.display()
    );
    Ok(())
}
