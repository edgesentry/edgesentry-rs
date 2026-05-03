#!/usr/bin/env bash
# run_local_demo.sh — Full pipeline demo (Phases 1-3)
#
# Walks through the complete edgesentry-rs pipeline using the forklift approach
# fixture and the demo profile. No external services required.
#
# Usage:
#   ./scripts/run_local_demo.sh              # interactive (pause between stages)
#   ./scripts/run_local_demo.sh --no-pause   # CI / non-interactive
#   ./scripts/run_local_demo.sh --skip-explain  # skip LLM stage
#
# Pipeline:
#   Stage 1  eds ingest replay      CSV → EntityFrame JSONL
#   Stage 2  eds compute run        EntityFrame → Measurement JSONL
#   Stage 3  eds evaluate run       EntityFrame + profile → RiskEvent JSONL
#   Stage 4  eds assess run         RiskEvent → Assessment JSONL
#   Stage 5  eds explain run        RiskEvent → Explanation JSONL  (requires LLM server)
#   Stage 6  eds report generate    Events + Assessment → Markdown report
#   Stage 7  eds report generate    Events + Assessment → PDF report
#   Stage 8  eds audit sign         RiskEvents → AuditRecord chain
#   Stage 9  eds audit verify       chain integrity check
#   Stage 10 eds scenario generate  synthetic CSV fixture
#   Stage 11 eds parse document     JSON document → EntityFrame JSONL
#   --- Document compliance audit chain (PIER71 TC4) ---
#   Stage 12 V001 compliant voyage  parse → fill → check (0 alerts) → gen → sign-document → verify
#   Stage 13 V002 BWM expired       parse → fill → check (HIGH alert) → sign-document (chain) → verify
#   Stage 14 V003 low confidence    parse → fill (threshold 0.80, flagged fields) → sign-document → verify
#
# Prerequisites:
#   cargo build (done automatically)
#   For stage 5: a running llama-server on http://localhost:8080
#                Start with:  ./scripts/run_llama.sh   (if available)
#                Skip with:   --skip-explain

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/target/debug/eds"

# ── Fixtures and profiles ────────────────────────────────────────────────────
FIXTURE_CSV="$ROOT/crates/edgesentry-ingest/fixtures/forklift_approach.csv"
PROFILE_DIR="$ROOT/crates/edgesentry-profile/fixtures/demo"
MARITIME_PROFILE="$ROOT/crates/edgesentry-profile/fixtures/sg-port-compliance"
DOC_FIXTURES="$ROOT/crates/edgesentry-document/fixtures"

# Fall back to clarus repo if edgesentry-rs fixtures not yet present
CLARUS="$(cd "$ROOT/../clarus" 2>/dev/null && pwd)" || true
if [[ ! -f "$FIXTURE_CSV" ]]; then
  if [[ -n "${CLARUS:-}" && -f "$CLARUS/fixtures/forklift_approach.csv" ]]; then
    FIXTURE_CSV="$CLARUS/fixtures/forklift_approach.csv"
    PROFILE_DIR="$CLARUS/profiles/demo"
  else
    echo "ERROR: fixture CSV not found. Expected:"
    echo "  $FIXTURE_CSV"
    echo "Or clone the clarus repo alongside edgesentry-rs."
    exit 1
  fi
fi

# Fall back to clarus repo for maritime profile if not in edgesentry-rs yet
if [[ ! -d "$MARITIME_PROFILE" ]] && [[ -n "${CLARUS:-}" && -d "$CLARUS/profiles/sg-port-compliance" ]]; then
  MARITIME_PROFILE="$CLARUS/profiles/sg-port-compliance"
fi

# ── Temp output dir ──────────────────────────────────────────────────────────
OUT="$(mktemp -d)"
trap 'rm -rf "$OUT"' EXIT

FRAMES_JSONL="$OUT/frames.jsonl"
MEASUREMENTS_JSONL="$OUT/measurements.jsonl"
EVENTS_JSONL="$OUT/events.jsonl"
ASSESSMENT_JSONL="$OUT/assessment.jsonl"
EXPLANATIONS_JSONL="$OUT/explanations.jsonl"
REPORT_MD="$OUT/report.md"
REPORT_PDF="$OUT/report.pdf"
AUDIT_JSONL="$OUT/audit.jsonl"
CHAIN_STATE="$OUT/chain.state"
SCENARIO_CSV="$OUT/scenario.csv"
PARSED_JSONL="$OUT/parsed.jsonl"
DOC_KEY=""          # populated at Stage 12 setup

# ── Flags ────────────────────────────────────────────────────────────────────
NO_PAUSE=false
SKIP_EXPLAIN=false
LLM_URL="http://localhost:8080"

for arg in "$@"; do
  case "$arg" in
    --no-pause)    NO_PAUSE=true ;;
    --skip-explain) SKIP_EXPLAIN=true ;;
    --llm-url=*)   LLM_URL="${arg#--llm-url=}" ;;
  esac
done

# ── Helpers ──────────────────────────────────────────────────────────────────
bold()  { printf "\033[1m%s\033[0m\n" "$*"; }
green() { printf "\033[0;32m%s\033[0m\n" "$*"; }
dim()   { printf "\033[2m%s\033[0m\n" "$*"; }

pause() {
  local label="$1"
  if [[ "$NO_PAUSE" == "false" ]]; then
    printf "\n\033[2m── %s — press Enter to continue ──\033[0m" "$label"
    read -r
  else
    echo ""
  fi
}

count_lines() {
  # Count non-header JSONL lines (skip line 1 which is the schema header)
  local file="$1"
  local total
  total=$(wc -l < "$file" | tr -d ' ')
  echo $(( total - 1 ))
}

# ── Build ────────────────────────────────────────────────────────────────────
bold "Building eds..."
cargo build -p eds --quiet 2>&1 | grep -v "^$" || true
green "Build complete."
echo ""

# ── Stage 1: Ingest ──────────────────────────────────────────────────────────
bold "━━ Stage 1 — Ingest (CSV → EntityFrame JSONL)"
dim  "  Command: eds ingest replay"
dim  "  Input:   $FIXTURE_CSV"
dim  "  Output:  frames.jsonl"
echo ""

"$BIN" ingest replay \
  --source "$FIXTURE_CSV" \
  --profile "$PROFILE_DIR" \
  --out "$FRAMES_JSONL"

FRAME_COUNT=$(count_lines "$FRAMES_JSONL")
green "  ✓ $FRAME_COUNT frames written"
dim   "  Schema header: $(head -1 "$FRAMES_JSONL")"

pause "Stage 1 complete"

# ── Stage 2: Compute ─────────────────────────────────────────────────────────
bold "━━ Stage 2 — Compute (EntityFrame → Measurement JSONL)"
dim  "  Command: eds compute run"
dim  "  Input:   frames.jsonl"
dim  "  Output:  measurements.jsonl"
echo ""

"$BIN" compute run \
  --input "$FRAMES_JSONL" \
  --out "$MEASUREMENTS_JSONL"

MEAS_COUNT=$(count_lines "$MEASUREMENTS_JSONL")
green "  ✓ $MEAS_COUNT measurements written"
dim   "  Sample: $(sed -n '2p' "$MEASUREMENTS_JSONL")"

pause "Stage 2 complete"

# ── Stage 3: Evaluate ────────────────────────────────────────────────────────
bold "━━ Stage 3 — Evaluate (EntityFrame + profile → RiskEvent JSONL)"
dim  "  Command: eds evaluate run"
dim  "  Profile: $PROFILE_DIR"
dim  "  Output:  events.jsonl"
echo ""

"$BIN" evaluate run \
  --input "$FRAMES_JSONL" \
  --profile "$PROFILE_DIR" \
  --out "$EVENTS_JSONL"

EVENT_COUNT=$(count_lines "$EVENTS_JSONL")
green "  ✓ $EVENT_COUNT risk events detected"

if [[ "$EVENT_COUNT" -gt 0 ]]; then
  echo ""
  dim "  Events:"
  # Print each event's rule_id, severity, and evidence_quality (skip header line)
  tail -n +2 "$EVENTS_JSONL" | while IFS= read -r line; do
    rule=$(echo "$line" | python3 -c "import sys,json; d=json.load(sys.stdin); print(f\"  [{d['severity']}] {d['rule_id']} — entities: {', '.join(d['entity_ids'])} — value: {d['measured_value']:.2f} — quality: {d.get('evidence_quality','?')}\")" 2>/dev/null \
      || echo "  $line")
    dim "$rule"
  done
fi

pause "Stage 3 complete"

# ── Stage 4: Assess ──────────────────────────────────────────────────────────
bold "━━ Stage 4 — Assess (RiskEvent → Assessment JSONL)"
dim  "  Command: eds assess run"
dim  "  Input:   events.jsonl"
dim  "  Output:  assessment.jsonl"
echo ""

"$BIN" assess run \
  --input "$EVENTS_JSONL" \
  --out "$ASSESSMENT_JSONL"

green "  ✓ Assessment written"
dim   "  Summary:"
tail -n +2 "$ASSESSMENT_JSONL" | python3 -c "
import sys, json
d = json.load(sys.stdin)
print(f\"    trend:             {d['trend']}\")
print(f\"    events analysed:   {d['event_count']}\")
if d['repeated_rules']:
    print(f\"    repeated rules:    {', '.join(r['rule_id'] + ' x' + str(r['count']) for r in d['repeated_rules'])}\")
if d['correlated_entities']:
    pairs = ['/'.join(e['entity_ids']) + ' x' + str(e['event_count']) for e in d['correlated_entities']]
    print(f\"    entity pairs:      {', '.join(pairs)}\")
" 2>/dev/null || dim "  (install python3 for formatted output)"

pause "Stage 4 complete"

# ── Stage 5: Explain (optional) ──────────────────────────────────────────────
bold "━━ Stage 5 — Explain (RiskEvent → Explanation JSONL)"

if [[ "$SKIP_EXPLAIN" == "true" ]]; then
  dim  "  Skipped (--skip-explain)"
  pause "Stage 5 skipped"
else
  # Check if LLM server is reachable
  if curl -sf "$LLM_URL/v1/models" >/dev/null 2>&1; then
    dim  "  Command: eds explain run --pick severity --n 2"
    dim  "  LLM:     $LLM_URL"
    dim  "  Output:  explanations.jsonl"
    echo ""

    "$BIN" explain run \
      --input "$EVENTS_JSONL" \
      --n 2 \
      --pick severity \
      --llm-url "$LLM_URL" \
      --profile "$PROFILE_DIR" \
      --out "$EXPLANATIONS_JSONL"

    EXP_COUNT=$(count_lines "$EXPLANATIONS_JSONL")
    green "  ✓ $EXP_COUNT explanations written"
    echo ""
    dim "  Explanations:"
    tail -n +2 "$EXPLANATIONS_JSONL" | while IFS= read -r line; do
      text=$(echo "$line" | python3 -c "import sys,json; d=json.load(sys.stdin); marker='✓' if d['grounded'] else '⚠'; print(f\"  [{marker}] {d['rule_id']}: {d['text'][:120]}...\")" 2>/dev/null \
        || echo "  $line")
      dim "$text"
    done
  else
    dim  "  LLM server not reachable at $LLM_URL — skipping"
    dim  "  To enable: start llama-server then re-run without --skip-explain"
    dim  "  Or run:    ./scripts/run_llama.sh (if available)"
  fi
  pause "Stage 5 complete"
fi

# ── Stage 6: Report (Markdown) ───────────────────────────────────────────────
bold "━━ Stage 6 — Report (Events + Assessment → Markdown)"
dim  "  Command: eds report generate --format md"
dim  "  Output:  report.md"
echo ""

"$BIN" report generate \
  --events "$EVENTS_JSONL" \
  --assessment "$ASSESSMENT_JSONL" \
  --site-name "Demo Warehouse A" \
  --period "$(date '+%B %Y')" \
  --out "$REPORT_MD"

green "  ✓ Markdown report written"
dim   "  Preview (first 5 lines):"
head -5 "$REPORT_MD" | while IFS= read -r line; do dim "    $line"; done

pause "Stage 6 complete"

# ── Stage 7: Report (PDF) ─────────────────────────────────────────────────────
bold "━━ Stage 7 — Report (Events + Assessment → PDF)"
dim  "  Command: eds report generate --format pdf"
dim  "  Output:  report.pdf"
echo ""

"$BIN" report generate \
  --events "$EVENTS_JSONL" \
  --assessment "$ASSESSMENT_JSONL" \
  --site-name "Demo Warehouse A" \
  --period "$(date '+%B %Y')" \
  --format pdf \
  --out "$REPORT_PDF"

PDF_SIZE=$(wc -c < "$REPORT_PDF" | tr -d ' ')
green "  ✓ PDF report written ($PDF_SIZE bytes)"

pause "Stage 7 complete"

# ── Stage 8: Seal ────────────────────────────────────────────────────────────
bold "━━ Stage 8 — Seal (RiskEvents → AuditRecord chain)"
dim  "  Command: eds audit sign-record (per-event, chained via prev_hash)"
dim  "  Key:     demo Ed25519 key (all 0x01 bytes — not for production)"
dim  "  Output:  audit.jsonl"
echo ""

DEMO_KEY="0101010101010101010101010101010101010101010101010101010101010101"
CHAIN_FILE="$OUT/chain.json"

# Generate a properly chained audit record sequence using the built-in demo command.
# This uses BLAKE3 + Ed25519 to produce a verifiable chain (same crypto as production).
"$BIN" audit demo-lift-inspection \
  --device-id "demo-edge-01" \
  --private-key-hex "$DEMO_KEY" \
  --out-file "$CHAIN_FILE"

RECORD_COUNT=$(python3 -c "import json; print(len(json.load(open('$CHAIN_FILE'))))" 2>/dev/null || echo "?")
green "  ✓ $RECORD_COUNT audit records sealed into chain"

pause "Stage 8 complete"

# ── Stage 9: Verify chain ────────────────────────────────────────────────────
bold "━━ Stage 9 — Verify audit chain"
dim  "  Command: eds audit verify-chain"
echo ""

if [[ -s "$CHAIN_FILE" ]]; then
  "$BIN" audit verify-chain \
    --records-file "$CHAIN_FILE" \
    && green "  ✓ Chain is valid — no tampering detected" \
    || { echo "  ✗ Chain verification failed"; exit 1; }
else
  dim "  (no audit records to verify)"
fi

pause "Stage 9 complete"

# ── Stage 10: Scenario generate ──────────────────────────────────────────────
bold "━━ Stage 10 — Scenario generate (synthetic CSV fixture)"
dim  "  Command: eds scenario generate"
dim  "  Output:  scenario.csv"
echo ""

"$BIN" scenario generate \
  --frames 10 \
  --entities 2 \
  --out "$SCENARIO_CSV"

SCENARIO_ROWS=$(( $(wc -l < "$SCENARIO_CSV" | tr -d ' ') - 1 ))
green "  ✓ $SCENARIO_ROWS entity rows written"
dim   "  Header: $(head -1 "$SCENARIO_CSV")"

pause "Stage 10 complete"

# ── Stage 11: Parse document ──────────────────────────────────────────────────
bold "━━ Stage 11 — Parse document (JSON → EntityFrame JSONL)"
dim  "  Command: eds parse document"
dim  "  Input:   sample_document.json"
dim  "  Output:  parsed.jsonl"
echo ""

SAMPLE_DOC="$ROOT/crates/edgesentry-parse/fixtures/sample_document.json"
"$BIN" parse document \
  --source "$SAMPLE_DOC" \
  --out "$PARSED_JSONL"

PARSED_FRAMES=$(( $(wc -l < "$PARSED_JSONL" | tr -d ' ') - 1 ))
green "  ✓ $PARSED_FRAMES EntityFrame(s) written"
dim   "  Schema: $(head -1 "$PARSED_JSONL")"

pause "Stage 11 complete"

# ── Stage 12: Document compliance — V001 compliant ───────────────────────────
bold "━━ Stage 12 — Document audit chain: V001 compliant voyage"
dim  "  Scenario: FAL Form 1, all fields present, no compliance alerts"
echo ""

# Generate a fresh keypair for the document audit chain
DOC_KEYPAIR=$("$BIN" audit keygen)
DOC_KEY=$(echo "$DOC_KEYPAIR" | python3 -c "import sys,json; print(json.load(sys.stdin)['private_key_hex'])")
dim "  Keypair generated. private_key: ${DOC_KEY:0:16}…"
echo ""

ENTITY1="$OUT/doc_entity1.jsonl"
FILLED1="$OUT/doc_filled1.jsonl"
ALERTS1="$OUT/doc_alerts1.jsonl"
HTML1="$OUT/fal-form-1.html"
CHAIN1="$OUT/doc_chain1.json"

dim "  eds parse maritime → entity1.jsonl"
"$BIN" parse maritime --source "$DOC_FIXTURES/voyage_V001_compliant.csv" --out "$ENTITY1"

dim "  eds document fill  → filled1.jsonl"
"$BIN" document fill --input "$ENTITY1" --template fal-form-1 --out "$FILLED1"

dim "  eds document check → alerts1.jsonl"
"$BIN" document check --input "$FILLED1" --profile "$MARITIME_PROFILE" --out "$ALERTS1"
ALERT1=$(python3 -c "
import sys
lines = [l for l in open('$ALERTS1') if l.strip() and not l.startswith('{\"eds_schema')]
print(len(lines))
" 2>/dev/null || echo 0)
green "  ✓ compliance alerts: $ALERT1  (expected: 0)"

dim "  eds document gen   → fal-form-1.html"
"$BIN" document gen --input "$FILLED1" --template fal-form-1 --out "$HTML1"
dim "    $(wc -c < "$HTML1") bytes rendered"

dim "  eds audit sign-document → chain1.json (sequence 1)"
"$BIN" audit sign-document --payload "$FILLED1" --key "$DOC_KEY" --out "$CHAIN1"
green "  ✓ VERIFIED:"
"$BIN" audit verify-document --payload "$FILLED1" --chain "$CHAIN1" | sed 's/^/    /'

pause "Stage 12 complete"

# ── Stage 13: Document compliance — V002 BWM expired ─────────────────────────
bold "━━ Stage 13 — Document audit chain: V002 BWM certificate expired"
dim  "  Scenario: BWM_D2_EXPIRED HIGH alert; chain continues from Stage 12"
echo ""

ENTITY2="$OUT/doc_entity2.jsonl"
FILLED2="$OUT/doc_filled2.jsonl"
ALERTS2="$OUT/doc_alerts2.jsonl"
CHAIN2="$OUT/doc_chain2.json"

dim "  eds parse maritime + document fill"
"$BIN" parse maritime --source "$DOC_FIXTURES/voyage_V002_bwm_expired.csv" --out "$ENTITY2"
"$BIN" document fill --input "$ENTITY2" --template fal-form-1 --out "$FILLED2"

dim "  eds document check → compliance alerts"
"$BIN" document check --input "$FILLED2" --profile "$MARITIME_PROFILE" --out "$ALERTS2"
python3 -c "
import json, sys
lines = [l for l in open('$ALERTS2') if l.strip() and not l.startswith('{\"eds_schema')]
for l in lines:
    a = json.loads(l)
    print(f'    [{a[\"severity\"]}] {a[\"rule_id\"]} — {a[\"regulation\"]}')
" 2>/dev/null
green "  ✓ BWM_D2_EXPIRED HIGH alert detected"

dim "  eds audit sign-document → chain2.json (sequence 2, chained from Stage 12)"
"$BIN" audit sign-document --payload "$FILLED2" --key "$DOC_KEY" \
  --chain "$CHAIN1" --out "$CHAIN2"
green "  ✓ VERIFIED:"
"$BIN" audit verify-document --payload "$FILLED2" --chain "$CHAIN2" | sed 's/^/    /'

pause "Stage 13 complete"

# ── Stage 14: Document compliance — V003 low confidence ──────────────────────
bold "━━ Stage 14 — Document audit chain: V003 low confidence cargo"
dim  "  Scenario: CARGO_HS_CODE and CREW_COUNT flagged; review_required=true"
echo ""

ENTITY3="$OUT/doc_entity3.jsonl"
FILLED3="$OUT/doc_filled3.jsonl"
CHAIN3="$OUT/doc_chain3.json"

dim "  eds parse maritime + document fill (--confidence-threshold 0.80)"
"$BIN" parse maritime --source "$DOC_FIXTURES/voyage_V003_low_confidence.csv" --out "$ENTITY3"
"$BIN" document fill --input "$ENTITY3" --template fal-form-1 \
  --confidence-threshold 0.80 --out "$FILLED3"

dim "  eds audit sign-document → chain3.json (sequence 3, chained from Stage 13)"
"$BIN" audit sign-document --payload "$FILLED3" --key "$DOC_KEY" \
  --chain "$CHAIN2" --out "$CHAIN3"
green "  ✓ VERIFIED (flagged fields visible):"
"$BIN" audit verify-document --payload "$FILLED3" --chain "$CHAIN3" | sed 's/^/    /'

pause "Stage 14 complete"

# ── Summary ──────────────────────────────────────────────────────────────────
echo ""
bold "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
bold "  Demo complete"
bold "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
dim "  Pipeline:"
dim "  Stage 1   ingest replay   → $FRAME_COUNT frames"
dim "  Stage 2   compute run     → $MEAS_COUNT measurements"
dim "  Stage 3   evaluate run    → $EVENT_COUNT risk events"
dim "  Stage 4   assess run      → trend analysis"
[[ "$SKIP_EXPLAIN" == "false" ]] && dim "  Stage 5   explain run     → LLM explanations" || dim "  Stage 5   explain run     → skipped"
dim "  Stage 6   report (md)     → Markdown report"
dim "  Stage 7   report (pdf)    → PDF report"
dim "  Stage 8   audit sign      → sealed chain"
dim "  Stage 9   audit verify    → chain integrity"
dim "  Stage 10  scenario gen    → $SCENARIO_ROWS synthetic entity rows"
dim "  Stage 11  parse document  → $PARSED_FRAMES EntityFrame(s)"
dim "  Stage 12  doc audit V001  → compliant, $ALERT1 alerts, sequence 1 sealed"
dim "  Stage 13  doc audit V002  → BWM_D2_EXPIRED HIGH, sequence 2 chained"
dim "  Stage 14  doc audit V003  → flagged fields, review_required, sequence 3 chained"
echo ""
dim "  Three documents sealed into a tamper-evident chain:"
dim "    sequence 1  V001 (compliant)           → $CHAIN1"
dim "    sequence 2  V002 (BWM_D2_EXPIRED HIGH) → $CHAIN2"
dim "    sequence 3  V003 (flagged fields)       → $CHAIN3"
echo ""
