use clap::Subcommand;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use edgesentry_ingest::jsonl::JsonlWriter;
use edgesentry_parse::{document_to_entity_frames, parse_document_json, parse_maritime_csv};

#[derive(Debug, Subcommand)]
pub enum ParseCommand {
    /// Parse maritime CSV into DocumentEntity JSONL.
    Maritime {
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Parse a structured JSON document into EntityFrame JSONL.
    Document {
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Parse a structured JSON form into EntityFrame JSONL (same as document).
    Form {
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Image parsing stub — requires --features onnx.
    Image {
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
}

pub fn run(cmd: ParseCommand) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        ParseCommand::Maritime { source, out } => run_maritime(&source, &out),
        ParseCommand::Document { source, out } => run_document(&source, &out),
        ParseCommand::Form { source, out } => run_document(&source, &out),
        ParseCommand::Image { source: _, out } => run_image_stub(&out),
    }
}

fn run_maritime(source: &Path, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let file = fs::File::open(source)?;
    let entities = parse_maritime_csv(file).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    let out_file = fs::File::create(out)?;
    let mut writer = JsonlWriter::new(out_file, "eds.document-entity", "0.1")
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    for entity in &entities {
        writer.write_record(entity)
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    }

    eprintln!("parse maritime: {} entity(s) written to {}", entities.len(), out.display());
    Ok(())
}

fn run_document(source: &Path, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let file = fs::File::open(source)?;
    let doc = parse_document_json(file).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let frames = document_to_entity_frames(&doc);

    let out_file = fs::File::create(out)?;
    let mut writer = JsonlWriter::new(out_file, "eds.entity-frame", "0.1")
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    for frame in &frames {
        writer.write_record(frame)
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    }

    eprintln!("parse document: {} frame(s) written to {}", frames.len(), out.display());
    Ok(())
}

fn run_image_stub(out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("eds parse image: requires --features onnx; stub only");
    // Write an empty JSONL file (header only).
    let out_file = fs::File::create(out)?;
    JsonlWriter::new(out_file, "eds.entity-frame", "0.1")
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    Ok(())
}
