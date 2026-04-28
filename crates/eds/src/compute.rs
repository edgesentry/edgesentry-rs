//! `eds compute` subcommands — physics measurements over entity frames.

use clap::Subcommand;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use edgesentry_compute::{braking_distance, euclidean_distance, relative_velocity, time_to_collision};
use edgesentry_ingest::csv_replay::EntityFrame;
use edgesentry_ingest::jsonl::{JsonlReader, JsonlWriter};

#[derive(Debug, Serialize, Deserialize)]
pub struct Measurement {
    pub timestamp_ms: u64,
    pub entity_a: String,
    pub entity_b: Option<String>,
    pub kind: MeasurementKind,
    pub value: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MeasurementKind {
    Distance,
    Ttc,
    BrakingDistance,
}

#[derive(Debug, Subcommand)]
pub enum ComputeCommand {
    /// Compute measurements (distances, TTC, braking distances) from EntityFrame JSONL.
    Run {
        /// Input EntityFrame JSONL file.
        #[arg(long)]
        input: PathBuf,

        /// Output Measurement JSONL file.
        #[arg(long)]
        out: PathBuf,
    },
}

pub fn run(cmd: ComputeCommand) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        ComputeCommand::Run { input, out } => run_compute(input, out),
    }
}

fn run_compute(input: PathBuf, out: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let file = fs::File::open(&input)?;
    let mut reader = JsonlReader::open(file)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    let frames: Vec<EntityFrame> = reader
        .records()
        .collect::<Result<Vec<_>, String>>()
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    let out_file = fs::File::create(&out)?;
    let mut writer = JsonlWriter::new(out_file, "eds.measurement", "0.1")
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    let mut total = 0usize;
    for frame in &frames {
        let ts = frame.timestamp_ms;
        let entities = &frame.entities;

        // Pairwise: distance and TTC
        for i in 0..entities.len() {
            for j in (i + 1)..entities.len() {
                let a = &entities[i];
                let b = &entities[j];

                let dist = euclidean_distance(a, b);
                writer.write_record(&Measurement {
                    timestamp_ms: ts,
                    entity_a: a.id.clone(),
                    entity_b: Some(b.id.clone()),
                    kind: MeasurementKind::Distance,
                    value: dist,
                }).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
                total += 1;

                let rv = relative_velocity(a, b);
                let ttc = time_to_collision(dist, rv);
                writer.write_record(&Measurement {
                    timestamp_ms: ts,
                    entity_a: a.id.clone(),
                    entity_b: Some(b.id.clone()),
                    kind: MeasurementKind::Ttc,
                    value: ttc,
                }).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
                total += 1;
            }
        }

        // Per-entity: braking distance
        for entity in entities {
            let speed = entity.velocity.x.hypot(entity.velocity.y);
            let bd = braking_distance(speed, &entity.class);
            writer.write_record(&Measurement {
                timestamp_ms: ts,
                entity_a: entity.id.clone(),
                entity_b: None,
                kind: MeasurementKind::BrakingDistance,
                value: bd,
            }).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            total += 1;
        }
    }

    eprintln!(
        "compute run: wrote {total} measurement(s) from {} frame(s) to {}",
        frames.len(),
        out.display()
    );
    Ok(())
}
