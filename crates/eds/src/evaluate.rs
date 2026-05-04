//! `eds evaluate` subcommands — rule evaluation over entity frames.

use clap::Subcommand;
use std::fs;
use std::path::PathBuf;

use edgesentry_compute::{compute_entity_confidence, ConfidenceContext};
use edgesentry_evaluate::{evaluate, EvidenceQuality, RiskEvent};
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

        /// Minimum evidence quality to include: certified, degraded, or rejected (default: rejected = all).
        #[arg(long, default_value = "rejected")]
        min_quality: String,
    },
}

fn parse_min_quality(s: &str) -> Result<EvidenceQuality, Box<dyn std::error::Error>> {
    match s.to_lowercase().as_str() {
        "certified" => Ok(EvidenceQuality::Certified),
        "degraded"  => Ok(EvidenceQuality::Degraded),
        "rejected"  => Ok(EvidenceQuality::Rejected),
        other => Err(format!("unknown quality level '{}'; use certified, degraded, or rejected", other).into()),
    }
}

fn quality_passes(event: &RiskEvent, min: &EvidenceQuality) -> bool {
    if event.evidence_quality == EvidenceQuality::NotApplicable {
        return true;
    }
    match min {
        EvidenceQuality::Rejected      => true,
        EvidenceQuality::Degraded      => event.evidence_quality != EvidenceQuality::Rejected,
        EvidenceQuality::Certified     => event.evidence_quality == EvidenceQuality::Certified,
        EvidenceQuality::NotApplicable => true,
    }
}

pub fn run(cmd: EvaluateCommand) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        EvaluateCommand::Run { input, profile, out, min_quality } => {
            let min_q = parse_min_quality(&min_quality)?;
            run_evaluate(input, profile, out, min_q)
        }
    }
}

fn run_evaluate(input: PathBuf, profile: PathBuf, out: PathBuf, min_quality: EvidenceQuality) -> Result<(), Box<dyn std::error::Error>> {
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
    let mut filtered_events = 0usize;
    for frame in &frames {
        // Populate computed_confidence on each entity before evaluation.
        // Use zero drift (no calibration data in replay mode).
        let ctx = ConfidenceContext { now_ms: frame.timestamp_ms, drift_score: 0.0 };
        let enriched: Vec<_> = frame.entities.iter().map(|e| {
            let mut e = e.clone();
            e.computed_confidence = compute_entity_confidence(&e, &ctx);
            e
        }).collect();
        let events: Vec<RiskEvent> =
            evaluate(&rules, &enriched, frame.timestamp_ms);
        for event in &events {
            if quality_passes(event, &min_quality) {
                writer.write_record(event)
                    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
                total_events += 1;
            } else {
                filtered_events += 1;
            }
        }
    }

    eprintln!(
        "evaluate run: {} event(s) from {} frame(s) written to {} ({} filtered by --min-quality)",
        total_events,
        frames.len(),
        out.display(),
        filtered_events,
    );
    Ok(())
}
