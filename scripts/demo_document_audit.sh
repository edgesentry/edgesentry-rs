#!/usr/bin/env bash
# demo_document_audit.sh — TC4: document compliance audit chain demo
#
# Demonstrates the full document audit pipeline:
#   parse → fill → check → gen → sign-document → verify-document
#
# Maps to the PIER71 TC4 scenario: an AI-assisted port-entry document
# (FAL Form 1) is generated, sealed into a tamper-evident audit chain,
# and the chain is queried to produce a human-readable audit trace.
#
# Usage:
#   ./scripts/demo_document_audit.sh              # all three voyages
#   ./scripts/demo_document_audit.sh --no-pause   # CI / non-interactive
#
# Prerequisites:
#   cargo build -p eds   (done automatically)
#
# Three test cases:
#   V001  compliant voyage — all fields filled, no flags
#   V002  expired BWM certificate — HIGH compliance alert, blocked
#   V003  low confidence cargo description — review required

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/target/debug/eds"
FIXTURES="$ROOT/crates/edgesentry-document/fixtures"
PROFILE="$ROOT/crates/edgesentry-profile/fixtures/sg-port-compliance"
TMPDIR_LOCAL="$(mktemp -d /tmp/eds_demo_doc_XXXXXX)"
trap 'rm -rf "$TMPDIR_LOCAL"' EXIT

NO_PAUSE=false
for arg in "$@"; do [[ "$arg" == "--no-pause" ]] && NO_PAUSE=true; done

pause() {
    $NO_PAUSE && return
    echo ""
    read -rp "  [press Enter to continue] " _
}

header() { echo ""; echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"; echo "  $1"; echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"; }
step()   { echo ""; echo "  ▶ $1"; }

# ── Build ────────────────────────────────────────────────────────────────────
header "Building eds…"
cargo build -p eds --quiet 2>&1 | tail -1

# ── Keypair ───────────────────────────────────────────────────────────────────
header "Setup — generate signing keypair"
KEYPAIR=$("$BIN" audit keygen)
KEY=$(echo "$KEYPAIR" | python3 -c "import sys,json; print(json.load(sys.stdin)['private_key_hex'])")
echo "  private_key: ${KEY:0:16}…  (first 8 bytes shown)"

# ── TC1: compliant voyage ─────────────────────────────────────────────────────
header "TC1 — Compliant voyage (V001)"

step "eds parse maritime  →  entity.jsonl"
"$BIN" parse maritime --source "$FIXTURES/voyage_V001_compliant.csv" \
  --out "$TMPDIR_LOCAL/entity1.jsonl"
echo "    $(wc -l < "$TMPDIR_LOCAL/entity1.jsonl") line(s) in entity1.jsonl"

step "eds document fill  →  filled.jsonl"
"$BIN" document fill --input "$TMPDIR_LOCAL/entity1.jsonl" \
  --template fal-form-1 --out "$TMPDIR_LOCAL/filled1.jsonl"
echo "    $(wc -l < "$TMPDIR_LOCAL/filled1.jsonl") line(s) in filled1.jsonl"

step "eds document check  →  compliance alerts"
"$BIN" document check --input "$TMPDIR_LOCAL/filled1.jsonl" \
  --profile "$PROFILE" --out "$TMPDIR_LOCAL/alerts1.jsonl"
ALERT_COUNT=$(python3 -c "
import sys
lines = [l for l in open('$TMPDIR_LOCAL/alerts1.jsonl') if l.strip() and not l.startswith('{\"eds_schema')]
print(len(lines))
" 2>/dev/null || echo 0)
echo "    compliance alerts: $ALERT_COUNT  (expected: 0)"

step "eds document gen  →  fal-form-1.html"
"$BIN" document gen --input "$TMPDIR_LOCAL/filled1.jsonl" \
  --template fal-form-1 --out "$TMPDIR_LOCAL/fal-form-1.html"
echo "    $(wc -c < "$TMPDIR_LOCAL/fal-form-1.html") bytes rendered"

step "eds audit sign-document  →  record.json"
"$BIN" audit sign-document --payload "$TMPDIR_LOCAL/filled1.jsonl" \
  --key "$KEY" --out "$TMPDIR_LOCAL/record.json"

step "eds audit verify-document  →  audit trace"
"$BIN" audit verify-document --payload "$TMPDIR_LOCAL/filled1.jsonl" \
  --chain "$TMPDIR_LOCAL/record.json"

pause

# ── TC2: expired BWM certificate ──────────────────────────────────────────────
header "TC2 — Expired BWM certificate (V002)"

step "eds parse maritime + eds document fill"
"$BIN" parse maritime --source "$FIXTURES/voyage_V002_bwm_expired.csv" \
  --out "$TMPDIR_LOCAL/entity2.jsonl"
"$BIN" document fill --input "$TMPDIR_LOCAL/entity2.jsonl" \
  --template fal-form-1 --out "$TMPDIR_LOCAL/filled2.jsonl"

step "eds document check  →  compliance alerts (expect BWM_D2_EXPIRED HIGH)"
"$BIN" document check --input "$TMPDIR_LOCAL/filled2.jsonl" \
  --profile "$PROFILE" --out "$TMPDIR_LOCAL/alerts2.jsonl" && \
  python3 -c "
import json, sys
lines = [l for l in open('$TMPDIR_LOCAL/alerts2.jsonl') if l.strip() and not l.startswith('{\"eds_schema')]
for l in lines: a = json.loads(l); print(f'    [{a[\"severity\"]}] {a[\"rule_id\"]} — {a[\"regulation\"]}')
"

step "eds audit sign-document  →  chain continues from TC1"
"$BIN" audit sign-document --payload "$TMPDIR_LOCAL/filled2.jsonl" \
  --key "$KEY" --chain "$TMPDIR_LOCAL/record.json" \
  --out "$TMPDIR_LOCAL/record2.json"

step "eds audit verify-document  →  audit trace (sequence 2)"
"$BIN" audit verify-document --payload "$TMPDIR_LOCAL/filled2.jsonl" \
  --chain "$TMPDIR_LOCAL/record2.json"

pause

# ── TC3: low confidence ───────────────────────────────────────────────────────
header "TC3 — Low confidence cargo description (V003)"

step "eds parse maritime + eds document fill (threshold 0.80)"
"$BIN" parse maritime --source "$FIXTURES/voyage_V003_low_confidence.csv" \
  --out "$TMPDIR_LOCAL/entity3.jsonl"
"$BIN" document fill --input "$TMPDIR_LOCAL/entity3.jsonl" \
  --template fal-form-1 --confidence-threshold 0.80 \
  --out "$TMPDIR_LOCAL/filled3.jsonl"

step "eds audit sign-document  →  chain continues from TC2"
"$BIN" audit sign-document --payload "$TMPDIR_LOCAL/filled3.jsonl" \
  --key "$KEY" --chain "$TMPDIR_LOCAL/record2.json" \
  --out "$TMPDIR_LOCAL/record3.json"

step "eds audit verify-document  →  audit trace (sequence 3, flagged fields visible)"
"$BIN" audit verify-document --payload "$TMPDIR_LOCAL/filled3.jsonl" \
  --chain "$TMPDIR_LOCAL/record3.json"

# ── Summary ───────────────────────────────────────────────────────────────────
header "Demo complete"
echo ""
echo "  Three documents signed into a tamper-evident chain:"
echo "    sequence 1  V001 (compliant)           → $TMPDIR_LOCAL/record.json"
echo "    sequence 2  V002 (BWM_D2_EXPIRED HIGH) → $TMPDIR_LOCAL/record2.json"
echo "    sequence 3  V003 (flagged fields)       → $TMPDIR_LOCAL/record3.json"
echo ""
echo "  Each AuditRecord links to the previous via prev_record_hash."
echo "  Run  eds audit verify-chain  on any output file to check integrity."
echo ""
