use clap::Subcommand;
use std::fs;
use std::path::PathBuf;

use edgesentry_ingest::jsonl::JsonlWriter;
use edgesentry_parse::parse_maritime_csv;

#[derive(Debug, Subcommand)]
pub enum ParseCommand {
    /// Parse maritime CSV into DocumentEntity JSONL.
    Maritime {
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
}

pub fn run(cmd: ParseCommand) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        ParseCommand::Maritime { source, out } => run_maritime(&source, &out),
    }
}

fn run_maritime(source: &PathBuf, out: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
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
