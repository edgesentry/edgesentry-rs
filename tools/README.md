# edgesentry-rs tools

Utilities for **issue [#404](https://github.com/edgesentry/edgesentry-rs/issues/404)** — live AIS UDP → `eds` pipeline demos.

| Script | Purpose |
|--------|---------|
| `ais_nmea.py` | AIS Type 1 `!AIVDM` encoder (matches `edgesentry-ingest` parser) |
| `nmea_udp_replay.py` | Replay `.nmea` logs to UDP `:9100` (offline / air-gapped) |
| `aisstream_udp_bridge.py` | aisstream.io WebSocket → NMEA UDP (needs `AISSTREAM_API_KEY`) |
| `generate_sg_strait_fixture.py` | Regenerate `demo/sg-strait-15min.nmea` |

## Quick start (offline)

```bash
cargo build --release -p eds
./demo/rpi5_live_ais_benchmark.sh
```

## Live stream (dev machine → RPi5 via SSH tunnel)

```bash
# Terminal A on RPi5
./target/release/eds ingest stream \
  --source ais://0.0.0.0:9100 \
  --profile crates/edgesentry-profile/fixtures/sg-maritime-security \
  --out /tmp/entity.jsonl

# Terminal B on laptop (forward UDP to Pi)
export AISSTREAM_API_KEY=...
python3 tools/aisstream_udp_bridge.py --host <pi-ip> --port 9100
```
