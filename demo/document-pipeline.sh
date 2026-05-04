#!/usr/bin/env bash
# document-pipeline.sh — End-to-end document pipeline demo
#
# Chain: CSV voyage data → DocumentEntity JSONL → FilledDocument JSONL
#        → ComplianceAlert JSONL → FAL Form 1 HTML → sealed AuditRecord
#
# Three test cases:
#   TC1  V001  Compliant vessel    — 0 alerts, AuditRecord sealed
#   TC2  V002  BWM cert expired    — HIGH alert fires, export still seals
#   TC3  V003  Low-confidence      — review_required: true, fields flagged
#
# Usage:
#   cargo build --release
#   bash demo/document-pipeline.sh
#
# Requirements: eds binary on PATH or built in target/release/

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EDS="${EDS_BIN:-${REPO_ROOT}/target/release/eds}"
FIXTURES="${REPO_ROOT}/crates/edgesentry-document/fixtures"
PROFILE="${REPO_ROOT}/crates/edgesentry-profile/fixtures/sg-port-compliance"
OUT="${REPO_ROOT}/demo/output"

if [[ ! -x "$EDS" ]]; then
  echo "eds binary not found at $EDS — run 'cargo build --release' first" >&2
  exit 1
fi

mkdir -p "$OUT"

# Generate a demo Ed25519 keypair (deterministic seed for reproducibility)
KEYPAIR=$("$EDS" audit keygen)
PRIVATE_KEY=$(echo "$KEYPAIR" | python3 -c "import sys,json; print(json.load(sys.stdin)['private_key_hex'])")

run_tc() {
  local tc="$1" fixture="$2" label="$3"
  local entity="${OUT}/${tc}_entity.jsonl"
  local filled="${OUT}/${tc}_filled.jsonl"
  local alerts="${OUT}/${tc}_alerts.jsonl"
  local html="${OUT}/${tc}_fal_form_1.html"
  local chain="${OUT}/${tc}_audit_chain.json"

  echo
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo "  ${tc} — ${label}"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

  # Step 1: Parse CSV → DocumentEntity JSONL
  "$EDS" parse maritime --source "${FIXTURES}/${fixture}" --out "$entity"
  echo "  [1/5] parse     → $(wc -l < "$entity") entity record(s)"

  # Step 2: Fill fields
  "$EDS" document fill --input "$entity" --template fal-form-1 --out "$filled"
  local review_required
  review_required=$(python3 -c "
import json, sys
records = [json.loads(l) for l in open('${filled}')]
flagged = [r for r in records if r.get('review_required')]
print('true' if flagged else 'false')
")
  echo "  [2/5] fill      → review_required: ${review_required}"

  # Step 3: Compliance check
  "$EDS" document check --input "$filled" --profile "$PROFILE" --out "$alerts"
  local alert_count
  alert_count=$(python3 -c "
import json
alerts = [json.loads(l) for l in open('${alerts}') if l.strip()]
print(sum(1 for a in alerts if 'rule_id' in a))
")
  if [[ "$alert_count" -gt 0 ]]; then
    echo "  [3/5] check     → ${alert_count} alert(s):"
    python3 -c "
import json
for line in open('${alerts}'):
    a = json.loads(line.strip())
    if 'rule_id' in a:
        print(f\"              [{a['severity']}] {a['rule_id']} — {a['message']}\")
"
  else
    echo "  [3/5] check     → 0 alerts"
  fi

  # Step 4: Render FAL Form 1 HTML
  "$EDS" document gen --input "$filled" --template fal-form-1 --out "$html"
  echo "  [4/5] gen html  → $(wc -c < "$html" | tr -d ' ') bytes → ${html}"

  # Step 5: Seal AuditRecord (BLAKE3 + Ed25519)
  "$EDS" audit sign-document \
    --payload "$filled" \
    --key "$PRIVATE_KEY" \
    --device-id "eds-demo" \
    --out "$chain"
  local record_count
  record_count=$(python3 -c "import json; print(len(json.load(open('${chain}'))))")
  local payload_hash
  payload_hash=$(python3 -c "
import json
records = json.load(open('${chain}'))
if records:
    h = records[0]['payload_hash']
    hex_str = ''.join(f'{b:02x}' for b in h) if isinstance(h, list) else str(h)
    print(hex_str[:16] + '...')
else:
    print('none')
")
  echo "  [5/5] seal      → ${record_count} AuditRecord(s), payload_hash: ${payload_hash}"

  echo
  echo "  RESULT: ${tc} $([ "$alert_count" -eq 0 ] && echo '✓ PASS' || echo "⚠ ${alert_count} ALERT(S)")"
}

echo
echo "edgesentry document pipeline demo"
echo "profile: sg-port-compliance"
echo "output:  ${OUT}/"

run_tc "TC1" "voyage_V001_compliant.csv"   "Compliant vessel — expect 0 alerts"
run_tc "TC2" "voyage_V002_bwm_expired.csv" "BWM certificate expired — expect HIGH alert"
run_tc "TC3" "voyage_V003_low_confidence.csv" "Low-confidence fields — expect review_required: true"

echo
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  All test cases complete. Output files:"
ls -lh "${OUT}/"
echo
echo "  Verify TC1 AuditRecord chain integrity:"
echo "    \$EDS audit verify-chain --chain ${OUT}/TC1_audit_chain.json"
