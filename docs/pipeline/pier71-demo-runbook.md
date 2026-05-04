# PIER71 Demo Runbook — documaris (PIER71-11)

**Purpose:** Verify the full documaris demo pipeline before PIER71 SPC Accelerate 2026 submission (due 15 June 2026).

**Test cases:**
- TC1 — FAL Form 1 generated from compliant voyage (0 alerts)
- TC2 — Expired BWM certificate → `BWM_D2_EXPIRED` HIGH alert
- TC3 — Low-confidence fields → `review_required: true`
- TC4 — AuditRecord sealed and chain verified (BLAKE3 + Ed25519)

---

## Fast path

Run all four TCs in one command:

```bash
cargo build --release
bash demo/document-pipeline.sh
```

Expected output: TC1 `✓ PASS`, TC2 `⚠ 1 ALERT(S)`, TC3 `⚠ 1 ALERT(S)`.
Then verify TC4 audit chain:

```bash
./target/release/eds audit verify-chain \
  --chain demo/output/TC1_audit_chain.json
```

Expected: exits 0, prints `chain OK`.

---

## Manual step-by-step

Use this path to inspect each intermediate file or to run with a live LLM.

### Prerequisites

```bash
cargo build --release
export EDS=./target/release/eds
```

To use a local LLM for field fill (optional — falls back to direct mapping without it):

```bash
ollama serve          # separate terminal
ollama pull llama3.2
export LLM_URL=http://localhost:11434/v1
```

### Generate a keypair (used in TC4)

```bash
$EDS audit keygen > /tmp/pier71_keypair.json
DEMO_KEY=$(python3 -c "import sys,json; print(json.load(open('/tmp/pier71_keypair.json'))['private_key_hex'])")
```

---

### TC1 — FAL Form 1, compliant voyage

**Fixture:** `voyage_V001_compliant.csv` — MV Horizon · IMO9876543 · SGP · SGSIN · arrives 2026-06-15

```bash
$EDS parse maritime \
  --source crates/edgesentry-document/fixtures/voyage_V001_compliant.csv \
  --out /tmp/entity_v001.jsonl

$EDS document fill \
  --input /tmp/entity_v001.jsonl \
  --template fal-form-1 \
  --llm-url "${LLM_URL:-}" \
  --out /tmp/filled_v001.jsonl

$EDS document check \
  --input /tmp/filled_v001.jsonl \
  --profile crates/edgesentry-profile/fixtures/sg-port-compliance \
  --out /tmp/alerts_v001.jsonl

$EDS document gen \
  --input /tmp/filled_v001.jsonl \
  --template fal-form-1 \
  --out /tmp/fal-form-1.html
```

**Checks:**
- [ ] `entity_v001.jsonl` — vessel_name `MV Horizon`, imo `IMO9876543`
- [ ] `filled_v001.jsonl` — `review_required: false`
- [ ] `alerts_v001.jsonl` — 0 compliance alerts
- [ ] `fal-form-1.html` — opens in browser, vessel and cargo fields populated

---

### TC2 — Expired BWM certificate → HIGH alert

**Fixture:** `voyage_V002_bwm_expired.csv` — MV Pacific Star · BWM expiry 2026-04-30 (past)

```bash
$EDS parse maritime \
  --source crates/edgesentry-document/fixtures/voyage_V002_bwm_expired.csv \
  --out /tmp/entity_v002.jsonl

$EDS document fill \
  --input /tmp/entity_v002.jsonl \
  --template fal-form-1 \
  --llm-url "${LLM_URL:-}" \
  --out /tmp/filled_v002.jsonl

$EDS document check \
  --input /tmp/filled_v002.jsonl \
  --profile crates/edgesentry-profile/fixtures/sg-port-compliance \
  --out /tmp/alerts_v002.jsonl
```

**Checks:**
- [ ] `alerts_v002.jsonl` — contains `BWM_D2_EXPIRED` with `severity: HIGH`
- [ ] Regulation cited: BWM Convention D-2 / MPA Port Circular No. 19 of 2023

---

### TC3 — Low-confidence fields → human review gate

**Fixture:** `voyage_V003_low_confidence.csv` — MV Venture · `crew_count` and `gross_tonnage` missing, cargo `"goods"` (vague)

```bash
$EDS parse maritime \
  --source crates/edgesentry-document/fixtures/voyage_V003_low_confidence.csv \
  --out /tmp/entity_v003.jsonl

$EDS document fill \
  --input /tmp/entity_v003.jsonl \
  --template fal-form-1 \
  --llm-url "${LLM_URL:-}" \
  --confidence-threshold 0.80 \
  --out /tmp/filled_v003.jsonl

$EDS document check \
  --input /tmp/filled_v003.jsonl \
  --profile crates/edgesentry-profile/fixtures/sg-port-compliance \
  --out /tmp/alerts_v003.jsonl
```

**Checks:**
- [ ] `filled_v003.jsonl` — `review_required: true`
- [ ] `alerts_v003.jsonl` — contains `CREW_COUNT_PRESENT` with `severity: HIGH`

---

### TC4 — AuditRecord sealed and chain verified

```bash
$EDS audit sign-document \
  --payload /tmp/filled_v001.jsonl \
  --key "$DEMO_KEY" \
  --device-id pier71-demo \
  --out /tmp/audit_chain.json

$EDS audit verify-document \
  --payload /tmp/filled_v001.jsonl \
  --chain /tmp/audit_chain.json

$EDS audit verify-chain \
  --chain /tmp/audit_chain.json
```

**Checks:**
- [ ] `audit_chain.json` — non-empty; record contains `voyage_id: V001`
- [ ] `verify-document` exits 0 — payload matches AuditRecord
- [ ] `verify-chain` exits 0 — prints `chain OK`

---

## Expected timing

| Step | Time |
|------|------|
| `cargo build --release` | ~40 s (first build) / ~2 s (cached) |
| Full demo script (TC1–TC4) | < 5 s |
| With LLM (`llama3.2`) | ~10–30 s per TC depending on hardware |

---

## Evaluator talking points

| TC | What to show | Why it matters |
|----|---|---|
| TC1 | HTML form rendered in browser | One click → all fields populated from vessel data |
| TC2 | `BWM_D2_EXPIRED` HIGH alert with MPA regulation citation | Errors caught before submission, not after rejection |
| TC3 | `review_required: true`, flagged fields highlighted | AI proposes; human must confirm low-confidence output |
| TC4 | `verify-chain` exits 0 | Tamper-proof record: neither documaris nor agent can alter it after generation |

---

## References

- Fixtures: `crates/edgesentry-document/fixtures/`
- Compliance profile: `crates/edgesentry-profile/fixtures/sg-port-compliance/rules.json`
- Automated script: `demo/document-pipeline.sh`
- PIER71 business brief: `edgesentry-commercial/docs/programs/pier71-spc-2026/documaris/pier71-business-brief.md`
