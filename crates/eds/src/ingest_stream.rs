//! `eds ingest stream` — receive EntityFrames via UDP and write JSONL.

use std::fs;
use std::path::{Path, PathBuf};

use edgesentry_ingest::csv_replay::EntityFrame;
use edgesentry_ingest::jsonl::JsonlWriter;
use edgesentry_ingest::udp::UnityUdpAdapter;

/// Run the UDP streaming ingest loop.
///
/// `source` must be a UDP address string, optionally prefixed with `udp://` or
/// `ais://`.
///
/// - `udp://` (or bare address): Unity JSON packet stream.
/// - `ais://`: NMEA 0183 VDM/VDO AIS sentence stream.  Requires a `params.toml`
///   in `profile_dir` with a `[reference_point]` section.
///
/// Binds the adapter, then loops: receives one packet, builds an EntityFrame,
/// and writes it to the JSONL output file.  The loop runs until the process
/// is terminated (Ctrl-C / SIGTERM).
pub fn run_stream(
    source: &str,
    profile_dir: &PathBuf,
    out: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    if source.starts_with("ais://") {
        return run_ais_stream(source, profile_dir, out);
    }

    // ── legacy UDP (Unity JSON) path ──────────────────────────────────────
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

// ── AIS stream ────────────────────────────────────────────────────────────────

fn run_ais_stream(
    source: &str,
    profile_dir: &Path,
    out: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use edgesentry_ingest::ais_nmea::{load_port_ref, AisAdapter};

    let addr = source.strip_prefix("ais://").unwrap_or(source);

    // Load params.toml from profile_dir
    let params_path = profile_dir.join("params.toml");
    let params_str = std::fs::read_to_string(&params_path)
        .map_err(|e| format!("cannot read {}: {e}", params_path.display()))?;
    let port_ref = load_port_ref(&params_str)
        .map_err(|e| format!("params.toml: {e}"))?;

    let mut adapter = AisAdapter::bind(addr, port_ref)
        .map_err(|e| format!("failed to bind {addr}: {e}"))?;

    let file = std::fs::File::create(out)?;
    let mut writer = JsonlWriter::new(file, "eds.entity-frame", "0.1")
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    eprintln!(
        "ingest stream (AIS): listening on {addr}, writing to {}",
        out.display()
    );

    loop {
        let entities = adapter
            .recv_entities()
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

        if !entities.is_empty() {
            let timestamp_ms = entities
                .iter()
                .find(|e| e.class != edgesentry_ingest::entity::EntityClass::AisGap)
                .map(|e| e.timestamp_ms)
                .unwrap_or_else(|| {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64
                });

            let frame = EntityFrame { timestamp_ms, entities };
            writer
                .write_record(&frame)
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        }
    }
}
