//! AIS NMEA 0183 VDM/VDO parser for Message Type 1/2/3 (position reports).
//!
//! Supports single-sentence messages only.  Multi-part VDM sentences are
//! silently ignored (returns `None`).

use std::collections::HashMap;
use std::net::UdpSocket;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::entity::{Entity, EntityClass, Vec2};
use edgesentry_compute::{latlon_to_local, cog_sog_to_velocity};

// ── AIS sentinel values ───────────────────────────────────────────────────────

const SOG_NOT_AVAILABLE: u64 = 1023;
const LON_NOT_AVAILABLE: i64 = 0x6791AC0;
const LAT_NOT_AVAILABLE: i64 = 0x3412140;
const COG_NOT_AVAILABLE: u64 = 3600;

// ── Public types ──────────────────────────────────────────────────────────────

/// Decoded AIS Type 1/2/3 position report.
#[derive(Debug, Clone, PartialEq)]
pub struct AisPositionReport {
    pub mmsi: u32,
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub sog_knots: f32,
    pub cog_deg: f32,
}

/// Port reference point loaded from params.toml.
#[derive(Debug, Clone)]
pub struct PortRef {
    pub lat_deg: f64,
    pub lon_deg: f64,
    /// Gap threshold in seconds before emitting an AisGap entity (default 480).
    pub ais_gap_threshold_s: u64,
}

// ── 6-bit payload decoding ────────────────────────────────────────────────────

/// Convert one ASCII character from the NMEA 6-bit payload encoding to its
/// 6-bit value.
#[inline]
fn decode_sixbit(c: u8) -> u8 {
    let v = c.wrapping_sub(48);
    if v > 40 { v - 8 } else { v }
}

/// Extract `len` bits starting at bit `start` from a slice of 6-bit values
/// (MSB first within each 6-bit element).  Returns up to 64 bits.
fn get_bits(sixbits: &[u8], start: usize, len: usize) -> u64 {
    let mut result: u64 = 0;
    for i in 0..len {
        let bit_pos = start + i;
        let byte_idx = bit_pos / 6;
        let bit_idx = 5 - (bit_pos % 6); // MSB first within each 6-bit char
        if byte_idx < sixbits.len() {
            let bit = ((sixbits[byte_idx] >> bit_idx) & 1) as u64;
            result = (result << 1) | bit;
        }
    }
    result
}

/// Extract a signed (two's-complement) integer of `len` bits starting at `start`.
fn get_signed(sixbits: &[u8], start: usize, len: usize) -> i64 {
    let raw = get_bits(sixbits, start, len);
    // Sign-extend if the MSB is set
    if len < 64 && (raw >> (len - 1)) & 1 == 1 {
        let mask = !((1u64 << len) - 1);
        (raw | mask) as i64
    } else {
        raw as i64
    }
}

// ── NMEA checksum ─────────────────────────────────────────────────────────────

/// Compute XOR checksum over the characters between `!` and `*` (exclusive).
fn nmea_checksum(sentence: &str) -> u8 {
    let body = sentence
        .trim_start_matches('!')
        .split('*')
        .next()
        .unwrap_or("");
    body.bytes().fold(0u8, |acc, b| acc ^ b)
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Parse a single `!AIVDM` or `!AIVDO` NMEA sentence.
///
/// Returns `None` for:
/// - non-1/2/3 message types
/// - malformed or multi-part sentences
/// - unavailable position / SOG / COG fields
pub fn parse_vdm(line: &str) -> Option<AisPositionReport> {
    let line = line.trim();

    // Must start with !AIVDM or !AIVDO
    if !line.starts_with("!AIVDM") && !line.starts_with("!AIVDO") {
        return None;
    }

    // Validate checksum: last 3 chars must be *XX
    let star_pos = line.rfind('*')?;
    if star_pos + 3 != line.len() {
        return None;
    }
    let expected_cs = u8::from_str_radix(&line[star_pos + 1..], 16).ok()?;
    if nmea_checksum(line) != expected_cs {
        return None;
    }

    // Split fields: !AIVDM,total,num,,channel,payload,pad*checksum
    let body = &line[1..star_pos]; // strip leading '!' and trailing '*XX'
    let fields: Vec<&str> = body.split(',').collect();
    if fields.len() < 7 {
        return None;
    }

    // Only handle single-sentence messages
    let total: u8 = fields[1].parse().ok()?;
    let num: u8 = fields[2].parse().ok()?;
    if total != 1 || num != 1 {
        return None;
    }

    let payload = fields[5];
    if payload.is_empty() {
        return None;
    }

    // Decode payload into 6-bit array
    let sixbits: Vec<u8> = payload.bytes().map(decode_sixbit).collect();
    if sixbits.len() < 20 {
        return None; // need at least enough bits for all fields
    }

    // Message type [0..6]
    let msg_type = get_bits(&sixbits, 0, 6);
    if msg_type != 1 && msg_type != 2 && msg_type != 3 {
        return None;
    }

    // MMSI [8..38] — 30 bits unsigned
    let mmsi = get_bits(&sixbits, 8, 30) as u32;

    // SOG [50..60] — 10 bits unsigned, in 0.1 knots; 1023 = not available
    let sog_raw = get_bits(&sixbits, 50, 10);
    if sog_raw == SOG_NOT_AVAILABLE {
        return None;
    }
    let sog_knots = sog_raw as f32 / 10.0;

    // Longitude [61..89] — 28 bits signed, in 1/10000 minutes (×600000 degrees)
    let lon_raw = get_signed(&sixbits, 61, 28);
    if lon_raw == LON_NOT_AVAILABLE {
        return None;
    }
    let lon_deg = lon_raw as f64 / 600_000.0;

    // Latitude [89..116] — 27 bits signed, in 1/10000 minutes (×600000 degrees)
    let lat_raw = get_signed(&sixbits, 89, 27);
    if lat_raw == LAT_NOT_AVAILABLE {
        return None;
    }
    let lat_deg = lat_raw as f64 / 600_000.0;

    // COG [116..128] — 12 bits unsigned, in 0.1 degrees; 3600 = not available
    let cog_raw = get_bits(&sixbits, 116, 12);
    if cog_raw == COG_NOT_AVAILABLE {
        return None;
    }
    let cog_deg = cog_raw as f32 / 10.0;

    Some(AisPositionReport {
        mmsi,
        lat_deg,
        lon_deg,
        sog_knots,
        cog_deg,
    })
}

// ── params.toml parser ────────────────────────────────────────────────────────

/// Parse params.toml: expects `[reference_point]` with `lat_deg` and `lon_deg`.
/// Optional `[ais_gap]` section with `threshold_s` (default 480).
///
/// Uses a minimal hand-rolled line-by-line parser — no external TOML crate.
pub fn load_port_ref(toml_str: &str) -> Result<PortRef, String> {
    let find_value = |key: &str| -> Option<f64> {
        toml_str
            .lines()
            .find(|l| l.trim_start().starts_with(key))
            .and_then(|l| l.split('=').nth(1))
            .map(|v| v.trim().trim_matches('#').split('#').next().unwrap_or("").trim().to_string())
            .and_then(|v| v.parse::<f64>().ok())
    };

    let lat_deg = find_value("lat_deg")
        .ok_or_else(|| "missing lat_deg in [reference_point]".to_string())?;
    let lon_deg = find_value("lon_deg")
        .ok_or_else(|| "missing lon_deg in [reference_point]".to_string())?;

    let ais_gap_threshold_s = toml_str
        .lines()
        .find(|l| l.trim_start().starts_with("threshold_s"))
        .and_then(|l| l.split('=').nth(1))
        .map(|v| v.trim().split('#').next().unwrap_or("").trim().to_string())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(480);

    Ok(PortRef {
        lat_deg,
        lon_deg,
        ais_gap_threshold_s,
    })
}

// ── UDP adapter ───────────────────────────────────────────────────────────────

/// UDP socket adapter that receives NMEA AIS sentences and emits `Entity` values.
pub struct AisAdapter {
    socket: UdpSocket,
    port_ref: PortRef,
    /// mmsi_str -> last timestamp_ms
    last_seen_ms: HashMap<String, u64>,
}

impl AisAdapter {
    /// Bind to a local UDP address (e.g. `"127.0.0.1:9100"`).
    pub fn bind(addr: &str, port_ref: PortRef) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(addr)?;
        Ok(Self {
            socket,
            port_ref,
            last_seen_ms: HashMap::new(),
        })
    }

    /// Receive one UDP datagram (one NMEA sentence) and return entities:
    ///
    /// - A normal `Entity` (class=`Vessel`) if a valid position was received.
    /// - Zero or more `AisGap` entities for any known MMSI that has been
    ///   silent for longer than the configured threshold.
    ///
    /// Gap entities are emitted at most once per gap event: the MMSI is
    /// removed from `last_seen_ms` after a gap fires.
    pub fn recv_entities(&mut self) -> Result<Vec<Entity>, String> {
        let mut buf = [0u8; 65535];
        let (len, _) = self
            .socket
            .recv_from(&mut buf)
            .map_err(|e| format!("UDP recv error: {e}"))?;

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let line = std::str::from_utf8(&buf[..len])
            .map_err(|e| format!("UTF-8 decode error: {e}"))?;

        let mut entities: Vec<Entity> = Vec::new();

        // Parse the incoming sentence
        if let Some(report) = parse_vdm(line) {
            let (x, y) = latlon_to_local(
                report.lat_deg,
                report.lon_deg,
                self.port_ref.lat_deg,
                self.port_ref.lon_deg,
            );
            let vel = cog_sog_to_velocity(report.cog_deg, report.sog_knots);
            let mmsi_str = report.mmsi.to_string();

            // Update last-seen timestamp
            self.last_seen_ms.insert(mmsi_str.clone(), now_ms);

            entities.push(Entity {
                id: mmsi_str,
                class: EntityClass::Vessel,
                position: Vec2::new(x, y),
                velocity: vel,
                timestamp_ms: now_ms,
            });
        }

        // Check all known MMSIs for gaps
        let threshold_ms = self.port_ref.ais_gap_threshold_s * 1000;
        let overdue: Vec<String> = self
            .last_seen_ms
            .iter()
            .filter(|(_, &ts)| now_ms.saturating_sub(ts) > threshold_ms)
            .map(|(k, _)| k.clone())
            .collect();

        for mmsi_str in overdue {
            let last = self.last_seen_ms.remove(&mmsi_str).unwrap_or(0);
            let gap_s = (now_ms.saturating_sub(last) / 1000) as f32;
            entities.push(Entity {
                id: mmsi_str,
                class: EntityClass::AisGap,
                position: Vec2::new(0.0, 0.0),
                velocity: Vec2::new(gap_s, 0.0),
                timestamp_ms: now_ms,
            });
        }

        Ok(entities)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Encoder helper (test-only) ────────────────────────────────────────

    /// Encode a value into `len` bits and append to the bit buffer.
    fn push_bits(bits: &mut Vec<u8>, value: u64, len: usize) {
        for i in (0..len).rev() {
            bits.push(((value >> i) & 1) as u8);
        }
    }

    /// Pack a bit vector into 6-bit characters (AIS payload encoding).
    fn bits_to_payload(bits: &[u8]) -> String {
        let mut payload = String::new();
        let mut i = 0;
        while i + 6 <= bits.len() {
            let mut v: u8 = 0;
            for j in 0..6 {
                v = (v << 1) | bits[i + j];
            }
            // Encode: add 48; if >= 88 add 8 more (skip chars 96–103)
            let c = if v + 48 >= 88 { v + 48 + 8 } else { v + 48 };
            payload.push(c as char);
            i += 6;
        }
        payload
    }

    /// Encode a Type-1 AIS position report as a valid `!AIVDM` sentence.
    #[cfg(test)]
    pub fn encode_type1(mmsi: u32, lat: f64, lon: f64, sog_knots: f32, cog_deg: f32) -> String {
        let mut bits: Vec<u8> = Vec::new();

        // [0..6]  message type = 1
        push_bits(&mut bits, 1, 6);
        // [6..8]  repeat indicator = 0
        push_bits(&mut bits, 0, 2);
        // [8..38] MMSI (30 bits)
        push_bits(&mut bits, mmsi as u64, 30);
        // [38..42] navigation status = 0
        push_bits(&mut bits, 0, 4);
        // [42..50] rate of turn = 0 (8 bits)
        push_bits(&mut bits, 0, 8);
        // [50..60] SOG × 10 (10 bits)
        let sog_raw = (sog_knots * 10.0).round() as u64;
        push_bits(&mut bits, sog_raw, 10);
        // [60]    position accuracy = 0
        push_bits(&mut bits, 0, 1);
        // [61..89] longitude × 600000 (28 bits signed)
        let lon_raw = (lon * 600_000.0).round() as i64;
        let lon_u = (lon_raw as u64) & ((1u64 << 28) - 1);
        push_bits(&mut bits, lon_u, 28);
        // [89..116] latitude × 600000 (27 bits signed)
        let lat_raw = (lat * 600_000.0).round() as i64;
        let lat_u = (lat_raw as u64) & ((1u64 << 27) - 1);
        push_bits(&mut bits, lat_u, 27);
        // [116..128] COG × 10 (12 bits)
        let cog_raw = (cog_deg * 10.0).round() as u64;
        push_bits(&mut bits, cog_raw, 12);
        // [128..137] true heading = 0 (9 bits)
        push_bits(&mut bits, 0, 9);
        // [137..143] time stamp = 0 (6 bits)
        push_bits(&mut bits, 0, 6);
        // [143..145] special manoeuvre = 0 (2 bits)
        push_bits(&mut bits, 0, 2);
        // [145..148] spare = 0 (3 bits)
        push_bits(&mut bits, 0, 3);
        // [148]   RAIM flag = 0 (1 bit)
        push_bits(&mut bits, 0, 1);
        // [149..168] radio status = 0 (19 bits) — pad to 168 total
        push_bits(&mut bits, 0, 19);

        let payload = bits_to_payload(&bits);
        let pad = 0; // 168 bits / 6 = 28 chars exactly, no padding needed

        let body = format!("AIVDM,1,1,,A,{payload},{pad}");
        let cs = body.bytes().fold(0u8, |a, b| a ^ b);
        format!("!{body}*{cs:02X}")
    }

    // ── parse_vdm tests ───────────────────────────────────────────────────

    #[test]
    fn parse_vdm_returns_none_for_garbage() {
        assert!(parse_vdm("this is not an NMEA sentence").is_none());
        assert!(parse_vdm("").is_none());
        assert!(parse_vdm("!AIVDM,1,1,,A,,0*00").is_none()); // empty payload
    }

    #[test]
    fn parse_vdm_type1_known_sentence() {
        // Encode a known position and verify round-trip decoding.
        let mmsi = 366_773_160u32;
        let lat = 37.7862_f64;
        let lon = -122.4168_f64;
        let sog = 0.0_f32;
        let cog = 0.0_f32;

        let sentence = encode_type1(mmsi, lat, lon, sog, cog);
        let report = parse_vdm(&sentence).expect("should parse encoded sentence");

        assert_eq!(report.mmsi, mmsi);
        assert!((report.lat_deg - lat).abs() < 1e-4,
            "lat: got {}, expected {lat}", report.lat_deg);
        assert!((report.lon_deg - lon).abs() < 1e-4,
            "lon: got {}, expected {lon}", report.lon_deg);
        assert!((report.sog_knots - sog).abs() < 0.1,
            "sog: got {}, expected {sog}", report.sog_knots);
        assert!((report.cog_deg - cog).abs() < 0.1,
            "cog: got {}, expected {cog}", report.cog_deg);
    }

    #[test]
    fn parse_vdm_roundtrip_moving_vessel() {
        let sentence = encode_type1(563_012_345, 1.2658, 103.8200, 5.0, 0.0);
        let report = parse_vdm(&sentence).expect("should parse");
        assert_eq!(report.mmsi, 563_012_345);
        assert!((report.sog_knots - 5.0).abs() < 0.1);
        assert!((report.lat_deg - 1.2658).abs() < 1e-4);
    }

    #[test]
    fn parse_vdm_bad_checksum_returns_none() {
        let good = encode_type1(123456789, 1.2640, 103.8200, 3.0, 180.0);
        // Corrupt the checksum digit
        let bad = good[..good.len() - 1].to_string() + "X";
        assert!(parse_vdm(&bad).is_none());
    }

    #[test]
    fn parse_vdm_multi_sentence_returns_none() {
        // total=2, num=1 — multi-part, should be skipped
        let payload = "15M67N0000000000000000000"; // arbitrary
        let body = format!("AIVDM,2,1,,A,{payload},0");
        let cs = body.bytes().fold(0u8, |a, b| a ^ b);
        let sentence = format!("!{body}*{cs:02X}");
        assert!(parse_vdm(&sentence).is_none());
    }

    // ── load_port_ref tests ───────────────────────────────────────────────

    #[test]
    fn load_port_ref_parses_lat_lon() {
        let toml = r#"
[reference_point]
lat_deg = 1.2640
lon_deg = 103.8200

[ais_gap]
threshold_s = 480
"#;
        let port_ref = load_port_ref(toml).expect("should parse");
        assert!((port_ref.lat_deg - 1.2640).abs() < 1e-6);
        assert!((port_ref.lon_deg - 103.8200).abs() < 1e-6);
        assert_eq!(port_ref.ais_gap_threshold_s, 480);
    }

    #[test]
    fn load_port_ref_defaults_threshold_to_480() {
        let toml = "[reference_point]\nlat_deg = 1.0\nlon_deg = 103.0\n";
        let port_ref = load_port_ref(toml).expect("should parse");
        assert_eq!(port_ref.ais_gap_threshold_s, 480);
    }

    #[test]
    fn load_port_ref_missing_lat_returns_error() {
        let toml = "[reference_point]\nlon_deg = 103.0\n";
        assert!(load_port_ref(toml).is_err());
    }

    #[test]
    fn load_port_ref_missing_lon_returns_error() {
        let toml = "[reference_point]\nlat_deg = 1.0\n";
        assert!(load_port_ref(toml).is_err());
    }
}
