use clap::Subcommand;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use edgesentry_assess::Assessment;
use edgesentry_evaluate::RiskEvent;
use edgesentry_ingest::jsonl::JsonlReader;
use edgesentry_report::{generate_report, render_markdown, render_pdf, validate, ReportConfig};

#[derive(Debug, Clone, clap::ValueEnum, PartialEq)]
pub enum ReportFormat {
    Md,
    Pdf,
}

#[derive(Debug, Subcommand)]
pub enum ReportCommand {
    /// Generate a Markdown or PDF safety report from events and assessment.
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
        /// Output format: md (default) or pdf
        #[arg(long, value_enum, default_value = "md")]
        format: ReportFormat,
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
        ReportCommand::Generate { events, assessment, site_name, period, chain_valid, format, out } => {
            run_generate(&events, &assessment, site_name, period, chain_valid, format, &out)
        }
        ReportCommand::Validate { events, assessment } => {
            run_validate(&events, &assessment)
        }
    }
}

fn read_events(path: &Path) -> Result<Vec<RiskEvent>, Box<dyn std::error::Error>> {
    let file = fs::File::open(path)?;
    let mut reader = JsonlReader::open(file)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let events: Vec<RiskEvent> = reader
        .records()
        .collect::<Result<Vec<_>, String>>()
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    Ok(events)
}

fn read_assessment(path: &Path) -> Result<Assessment, Box<dyn std::error::Error>> {
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
    events_path: &Path,
    assessment_path: &Path,
    site_name: Option<String>,
    period: Option<String>,
    chain_valid_flag: bool,
    format: ReportFormat,
    out: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let events = read_events(events_path)?;
    let assessment = read_assessment(assessment_path)?;

    let chain_valid = if chain_valid_flag { Some(true) } else { None };
    let config = ReportConfig { site_name, report_period: period, chain_valid, explanations: vec![] };

    let report = generate_report(&events, &assessment, config);

    match format {
        ReportFormat::Pdf => {
            let bytes = render_pdf(&report);
            fs::write(out, bytes)?;
            eprintln!("report generate: {} event(s) → {} (PDF)", events.len(), out.display());
        }
        ReportFormat::Md => {
            let md = render_markdown(&report);
            fs::write(out, md)?;
            eprintln!("report generate: {} event(s) → {}", events.len(), out.display());
        }
    }

    Ok(())
}

fn run_validate(
    events_path: &Path,
    assessment_path: &Path,
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
