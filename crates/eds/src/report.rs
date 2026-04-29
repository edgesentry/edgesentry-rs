use clap::Subcommand;
use std::fs;
use std::path::PathBuf;

use edgesentry_assess::Assessment;
use edgesentry_evaluate::RiskEvent;
use edgesentry_ingest::jsonl::JsonlReader;
use edgesentry_report::{generate_report, render_markdown, validate, ReportConfig};

#[derive(Debug, Subcommand)]
pub enum ReportCommand {
    /// Generate a Markdown safety report from events and assessment.
    Generate {
        #[arg(long)]
        events: PathBuf,
        #[arg(long)]
        assessment: PathBuf,
        #[arg(long)]
        site_name: Option<String>,
        #[arg(long)]
        period: Option<String>,
        #[arg(long)]
        chain_valid: bool,
        #[arg(long)]
        out: PathBuf,
    },
    /// Validate that events and assessment are non-empty.
    Validate {
        #[arg(long)]
        events: PathBuf,
        #[arg(long)]
        assessment: PathBuf,
    },
}

pub fn run(cmd: ReportCommand) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        ReportCommand::Generate { events, assessment, site_name, period, chain_valid, out } => {
            run_generate(&events, &assessment, site_name, period, chain_valid, &out)
        }
        ReportCommand::Validate { events, assessment } => {
            run_validate(&events, &assessment)
        }
    }
}

fn read_events(path: &PathBuf) -> Result<Vec<RiskEvent>, Box<dyn std::error::Error>> {
    let file = fs::File::open(path)?;
    let mut reader = JsonlReader::open(file)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let events: Vec<RiskEvent> = reader
        .records()
        .collect::<Result<Vec<_>, String>>()
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    Ok(events)
}

fn read_assessment(path: &PathBuf) -> Result<Assessment, Box<dyn std::error::Error>> {
    let file = fs::File::open(path)?;
    let mut reader = JsonlReader::open(file)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let assessment: Assessment = reader
        .records()
        .next()
        .ok_or_else(|| -> Box<dyn std::error::Error> { "assessment file is empty".into() })?
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    Ok(assessment)
}

fn run_generate(
    events_path: &PathBuf,
    assessment_path: &PathBuf,
    site_name: Option<String>,
    period: Option<String>,
    chain_valid_flag: bool,
    out: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let events = read_events(events_path)?;
    let assessment = read_assessment(assessment_path)?;

    let chain_valid = if chain_valid_flag { Some(true) } else { None };
    let config = ReportConfig { site_name, report_period: period, chain_valid };

    let report = generate_report(&events, &assessment, config);
    let md = render_markdown(&report);

    fs::write(out, md)?;
    eprintln!("report generate: {} event(s) → {}", events.len(), out.display());
    Ok(())
}

fn run_validate(
    events_path: &PathBuf,
    assessment_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let events = read_events(events_path)?;
    let assessment = read_assessment(assessment_path)?;

    match validate(&events, &assessment) {
        Ok(()) => {
            println!("OK");
        }
        Err(e) => {
            eprintln!("validation error: {e}");
            std::process::exit(1);
        }
    }
    Ok(())
}
