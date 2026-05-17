#!/usr/bin/env bash
# rpi5_live_ais_benchmark.sh — Live/recorded AIS UDP → eds pipeline benchmark (issue #404)
#
# Offline (default): replays demo/sg-strait-15min.nmea to local UDP :9100
# Live: set USE_LIVE=1 and AISSTREAM_API_KEY (runs aisstream_udp_bridge.py)
#
# Usage:
#   ./demo/rpi5_live_ais_benchmark.sh
#   ./demo/rpi5_live_ais_benchmark.sh --duration 30 --speed 30
#   USE_LIVE=1 AISSTREAM_API_KEY=... ./demo/rpi5_live_ais_benchmark.sh
#
# RPi5: run on device after `cargo build --release -p eds` (aarch64).

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# Prefer debug build (release profile may be absent in some dev trees)
if [[ -z "${EDS_BIN:-}" ]]; then
  if [[ -x "$ROOT/target/release/eds" ]]; then
    BIN="$ROOT/target/release/eds"
  else
    BIN="$ROOT/target/debug/eds"
  fi
else
  BIN="$EDS_BIN"
fi
PROFILE="${EDS_PROFILE:-$ROOT/crates/edgesentry-profile/fixtures/sg-maritime-security}"
NMEA="${NMEA_FILE:-$ROOT/demo/sg-strait-15min.nmea}"
TOOLS="$ROOT/tools"

DURATION=60
SPEED=15
UDP_HOST="127.0.0.1"
UDP_PORT=9100
SKIP_BUILD=0
USE_LIVE="${USE_LIVE:-0}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --duration) DURATION="$2"; shift 2 ;;
    --speed) SPEED="$2"; shift 2 ;;
    --host) UDP_HOST="$2"; shift 2 ;;
    --port) UDP_PORT="$2"; shift 2 ;;
    --nmea) NMEA="$2"; shift 2 ;;
    --skip-build) SKIP_BUILD=1; shift ;;
    -h|--help)
      sed -n '2,14p' "$0"
      exit 0
      ;;
    *) echo "Unknown option: $1" >&2; exit 1 ;;
  esac
done

if [[ "$SKIP_BUILD" -eq 0 ]]; then
  echo "==> cargo build -p eds"
  (cd "$ROOT" && cargo build -p eds)
  BIN="$ROOT/target/debug/eds"
fi

if [[ ! -x "$BIN" ]]; then
  echo "error: eds binary not found (run: cargo build -p eds)" >&2
  exit 1
fi
if [[ ! -f "$NMEA" ]] && [[ "$USE_LIVE" != "1" ]]; then
  echo "==> generating $NMEA"
  python3 "$TOOLS/generate_sg_strait_fixture.py"
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"; kill $(jobs -p) 2>/dev/null || true' EXIT

ENTITY="$TMP/entity.jsonl"
EVENTS="$TMP/events.jsonl"
AUDIT="$TMP/audit.jsonl"
INGEST_LOG="$TMP/ingest.log"
DEMO_KEY="0101010101010101010101010101010101010101010101010101010101010101"
AIS_SOURCE="ais://${UDP_HOST}:${UDP_PORT}"

echo "==> Starting eds ingest stream ($AIS_SOURCE)"
"$BIN" ingest stream \
  --source "$AIS_SOURCE" \
  --profile "$PROFILE" \
  --out "$ENTITY" \
  >>"$INGEST_LOG" 2>&1 &
INGEST_PID=$!
sleep 1

if ! kill -0 "$INGEST_PID" 2>/dev/null; then
  echo "error: ingest failed to start" >&2
  cat "$INGEST_LOG" >&2
  exit 1
fi

echo "==> Sampling CPU/RSS for ${DURATION}s (pid $INGEST_PID)"
CPU_MAX=0
RSS_MAX_KB=0
SAMPLES=0
END=$((SECONDS + DURATION))

feed() {
  if [[ "$USE_LIVE" == "1" ]]; then
    python3 "$TOOLS/aisstream_udp_bridge.py" \
      --host "$UDP_HOST" --port "$UDP_PORT" --duration "$DURATION"
  else
    python3 "$TOOLS/nmea_udp_replay.py" \
      "$NMEA" --host "$UDP_HOST" --port "$UDP_PORT" \
      --speed "$SPEED" --duration "$DURATION" --loop
  fi
}

feed &
FEED_PID=$!

while [[ $SECONDS -lt $END ]]; do
  if kill -0 "$INGEST_PID" 2>/dev/null; then
    # Linux ps: RSS in KB; macOS RSS in bytes — normalise to KB
    read -r CPU RSS <<<"$(ps -p "$INGEST_PID" -o %cpu= -o rss= 2>/dev/null | tr -d ' ')" || true
    if [[ -n "${CPU:-}" ]]; then
      SAMPLES=$((SAMPLES + 1))
      CPU_INT=$(python3 -c "import math; print(int(math.ceil(float('${CPU}' or 0))))")
      (( CPU_INT > CPU_MAX )) && CPU_MAX=$CPU_INT
      RSS_KB=$RSS
      if [[ "$(uname -s)" == "Darwin" ]] && [[ -n "$RSS" ]]; then
        RSS_KB=$((RSS / 1024))
      fi
      (( RSS_KB > RSS_MAX_KB )) && RSS_MAX_KB=$RSS_KB
    fi
  fi
  sleep 1
done

wait "$FEED_PID" 2>/dev/null || true
kill "$INGEST_PID" 2>/dev/null || true
wait "$INGEST_PID" 2>/dev/null || true

FRAME_COUNT=$(wc -l <"$ENTITY" | tr -d ' ')
echo "==> ingest: $FRAME_COUNT entity frames"

echo "==> eds evaluate run"
"$BIN" evaluate run \
  --input "$ENTITY" \
  --profile "$PROFILE" \
  --out "$EVENTS"

EVENT_COUNT=$(python3 -c "
import json
print(sum(1 for line in open('$EVENTS') if line.strip() and 'rule_id' in line))
")
echo "==> evaluate: $EVENT_COUNT risk events"

echo "==> eds audit sign-record (per event)"
SEQ=$(python3 "$TOOLS/seal_events.py" "$EVENTS" "$BIN" "$DEMO_KEY" "$TMP")

RULES=$(python3 -c "
import json
rules = set()
for line in open('$EVENTS'):
    line = line.strip()
    if not line or 'rule_id' not in line:
        continue
    rules.add(json.loads(line)['rule_id'])
print(', '.join(sorted(rules)) or '(none)')
")

RSS_MB=$(python3 -c "print(f'{$RSS_MAX_KB / 1024:.1f}')")

echo ""
echo "┌─────────────────────────────────────────────────────────────┐"
echo "│  RPi5 live AIS benchmark (issue #404)                       │"
echo "├─────────────────────────────────────────────────────────────┤"
printf "│  Source        %-44s │\n" "$([ "$USE_LIVE" = 1 ] && echo live aisstream || echo replay $NMEA)"
printf "│  Duration      %-44s │\n" "${DURATION}s @ ${SPEED}×"
printf "│  Entity frames %-44s │\n" "$FRAME_COUNT"
printf "│  Risk events   %-44s │\n" "$EVENT_COUNT"
printf "│  Rules fired   %-44s │\n" "$RULES"
printf "│  CPU (max)     %-44s │\n" "${CPU_MAX}%"
printf "│  RSS (max)     %-44s │\n" "${RSS_MB} MB"
printf "│  Audit sealed  %-44s │\n" "$SEQ records"
echo "└─────────────────────────────────────────────────────────────┘"

if [[ "$EVENT_COUNT" -eq 0 ]]; then
  echo "warning: no risk events — check profile zone / replay speed" >&2
  exit 1
fi

echo "OK"
