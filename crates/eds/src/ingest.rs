//! `eds ingest` subcommands — entity ingestion pipeline.

use clap::Subcommand;
use std::fs;
use std::path::PathBuf;

use edgesentry_ingest::csv_replay::FileReplayAdapter;
use edgesentry_ingest::jsonl::JsonlWriter;

use crate::ingest_stream;

#[derive(Debug, Subcommand)]
pub enum IngestCommand {
    /// Replay entities from a CSV file and write EntityFrame JSONL.
    Replay {
        /// Input CSV file (id,class,x,y,vx,vy,timestamp_ms).
        #[arg(long)]
        source: PathBuf,

        /// Profile directory (reserved for future use).
        #[arg(long)]
        profile: Option<PathBuf>,

        /// Output JSONL file path.
        #[arg(long)]
        out: PathBuf,
    },
    /// Stream entities from a live UDP source and write EntityFrame JSONL.
    Stream {
        /// UDP source address, e.g. udp://127.0.0.1:9000.
        #[arg(long)]
        source: String,

        /// Profile directory.
        #[arg(long)]
        profile: PathBuf,

        /// Output JSONL file path.
        #[arg(long)]
        out: PathBuf,
    },
}

pub fn run(cmd: IngestCommand) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        IngestCommand::Replay { source, profile: _, out } => run_replay(source, out),
        IngestCommand::Stream { source, profile, out } => {
            ingest_stream::run_stream(&source, &profile, &out)
        }
    }
}

fn run_replay(source: PathBuf, out: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(&source)?;

    let adapter = FileReplayAdapter::from_csv(&content)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let frames = adapter.frames();

    let file = fs::File::create(&out)?;
    let mut writer = JsonlWriter::new(file, "eds.entity-frame", "0.1")
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    for frame in frames {
        writer.write_record(frame)
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    }

    eprintln!(
        "ingest replay: wrote {} frame(s) to {}",
        frames.len(),
        out.display()
    );
    Ok(())
}
