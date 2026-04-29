//! `eds ingest stream` — receive EntityFrames via UDP and write JSONL.

use std::fs;
use std::path::PathBuf;

use edgesentry_ingest::csv_replay::EntityFrame;
use edgesentry_ingest::jsonl::JsonlWriter;
use edgesentry_ingest::udp::UnityUdpAdapter;

/// Run the UDP streaming ingest loop.
///
/// `source` must be a UDP address string, optionally prefixed with `udp://`
/// (e.g. `"udp://127.0.0.1:9000"` or `"127.0.0.1:9000"`).
///
/// Binds the adapter, then loops: receives one packet, builds an EntityFrame,
/// and writes it to the JSONL output file.  The loop runs until the process
/// is terminated (Ctrl-C / SIGTERM).
pub fn run_stream(
    source: &str,
    _profile_dir: &PathBuf,
    out: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = source.strip_prefix("udp://").unwrap_or(source);

    let adapter = UnityUdpAdapter::bind(addr)
        .map_err(|e| -> Box<dyn std::error::Error> { format!("failed to bind {addr}: {e}").into() })?;

    let file = fs::File::create(out)?;
    let mut writer = JsonlWriter::new(file, "eds.entity-frame", "0.1")
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    eprintln!("ingest stream: listening on {addr}, writing to {}", out.display());

    loop {
        let entities = adapter
            .recv_entities()
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

        let timestamp_ms = entities.first().map(|e| e.timestamp_ms).unwrap_or(0);
        let frame = EntityFrame { timestamp_ms, entities };

        writer.write_record(&frame)
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    }
}
