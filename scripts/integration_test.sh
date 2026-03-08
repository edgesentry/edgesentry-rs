#!/usr/bin/env bash
# Non-interactive integration test covering the full local demo flow.
# Requires: docker, cargo
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
COMPOSE_FILE="$ROOT_DIR/docker-compose.local.yml"
RECORDS_FILE="/tmp/lift_inspection_records.json"
TAMPERED_FILE="/tmp/lift_inspection_records_tampered.json"
INSERT_SQL="/tmp/insert_lift_records.sql"

cleanup() {
  echo "[cleanup] Stopping backend services..."
  docker compose -f "$COMPOSE_FILE" down -v >/dev/null 2>&1 || true
}
trap cleanup EXIT

# ── 1. Start services ────────────────────────────────────────────────────────
echo "[1/7] Starting PostgreSQL + MinIO..."
docker compose -f "$COMPOSE_FILE" up -d postgres minio minio-setup >/dev/null

# ── 2. Wait for PostgreSQL ───────────────────────────────────────────────────
echo "[2/7] Waiting for PostgreSQL..."
for _ in $(seq 1 30); do
  STATUS=$(docker inspect --format='{{.State.Health.Status}}' edgesentry-rs-postgres 2>/dev/null || true)
  [[ "$STATUS" == "healthy" ]] && break
  sleep 1
done
[[ "${STATUS:-}" == "healthy" ]] || { echo "PostgreSQL did not become healthy"; exit 1; }
echo "PostgreSQL: healthy"

# ── 3. Wait for MinIO setup ──────────────────────────────────────────────────
echo "[3/7] Waiting for MinIO bucket setup..."
for _ in $(seq 1 30); do
  STATE=$(docker inspect --format='{{.State.Status}}' edgesentry-rs-minio-setup 2>/dev/null || true)
  CODE=$(docker inspect --format='{{.State.ExitCode}}' edgesentry-rs-minio-setup 2>/dev/null || true)
  [[ "$STATE" == "exited" && "$CODE" == "0" ]] && break
  sleep 1
done
[[ "${STATE:-}" == "exited" && "${CODE:-}" == "0" ]] || { echo "MinIO setup did not complete"; exit 1; }
echo "MinIO: ready"

# ── 4. Build CLI ─────────────────────────────────────────────────────────────
echo "[4/7] Building CLI..."
cd "$ROOT_DIR"
cargo build -p edgesentry-rs --release >/dev/null

EDS="$ROOT_DIR/target/release/eds"

# ── 5. Generate chain and verify ─────────────────────────────────────────────
echo "[5/7] Generating and verifying chain..."
"$EDS" demo-lift-inspection --device-id lift-01 --out-file "$RECORDS_FILE"
"$EDS" verify-chain --records-file "$RECORDS_FILE"

# Tamper and confirm detection
python3 - <<'PY'
import json, pathlib
src = pathlib.Path('/tmp/lift_inspection_records.json')
dst = pathlib.Path('/tmp/lift_inspection_records_tampered.json')
records = json.loads(src.read_text())
records[0]["payload_hash"][0] ^= 0x01
dst.write_text(json.dumps(records, indent=2))
PY

set +e
"$EDS" verify-chain --records-file "$TAMPERED_FILE"
TAMPER_EXIT=$?
set -e
[[ "$TAMPER_EXIT" -ne 0 ]] || { echo "FAIL: tampered chain was accepted"; exit 1; }
echo "Tamper detection: PASSED"

# ── 6. Insert into PostgreSQL and verify ─────────────────────────────────────
echo "[6/7] Inserting records into PostgreSQL..."
python3 - <<'PY'
import json, pathlib

records = json.loads(pathlib.Path('/tmp/lift_inspection_records.json').read_text())

def esc(v): return v.replace("'", "''")

lines = ['BEGIN;', 'TRUNCATE TABLE operation_logs, audit_records RESTART IDENTITY;']
for rec in records:
    did = esc(rec['device_id'])
    lines.append(
        f"INSERT INTO audit_records (device_id, sequence, timestamp_ms, payload_hash, signature, prev_record_hash, object_ref) "
        f"VALUES ('{did}', {rec['sequence']}, {rec['timestamp_ms']}, "
        f"'{esc(json.dumps(rec['payload_hash']))}'::jsonb, "
        f"'{esc(json.dumps(rec['signature']))}'::jsonb, "
        f"'{esc(json.dumps(rec['prev_record_hash']))}'::jsonb, "
        f"'{esc(rec['object_ref'])}');"
    )
    lines.append(
        f"INSERT INTO operation_logs (decision, device_id, sequence, message) "
        f"VALUES ('Accepted', '{did}', {rec['sequence']}, 'integration test');"
    )
lines.append('COMMIT;')
pathlib.Path('/tmp/insert_lift_records.sql').write_text('\n'.join(lines) + '\n')
PY

docker exec -i edgesentry-rs-postgres psql -U trace -d trace_audit < "$INSERT_SQL" >/dev/null

AUDIT_COUNT=$(docker exec -i edgesentry-rs-postgres psql -U trace -d trace_audit -tAc "SELECT COUNT(*) FROM audit_records;")
LOG_COUNT=$(docker exec -i edgesentry-rs-postgres psql -U trace -d trace_audit -tAc "SELECT COUNT(*) FROM operation_logs;")

[[ "$AUDIT_COUNT" -eq 3 ]] || { echo "FAIL: expected 3 audit_records, got $AUDIT_COUNT"; exit 1; }
[[ "$LOG_COUNT" -eq 3 ]] || { echo "FAIL: expected 3 operation_logs, got $LOG_COUNT"; exit 1; }
echo "PostgreSQL: $AUDIT_COUNT audit records, $LOG_COUNT operation logs verified"

# ── 7. Done ──────────────────────────────────────────────────────────────────
echo "[7/7] All integration tests passed."
