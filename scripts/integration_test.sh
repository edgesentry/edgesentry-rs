#!/usr/bin/env bash
# Non-interactive integration test covering the full local demo flow.
# Requires: docker, cargo
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
COMPOSE_FILE="$ROOT_DIR/docker-compose.local.yml"
RECORDS_FILE="/tmp/lift_inspection_records.json"
PAYLOADS_FILE="/tmp/lift_inspection_payloads.json"
TAMPERED_FILE="/tmp/lift_inspection_records_tampered.json"
PG_URL="postgresql://trace:trace@localhost:5433/trace_audit"
MINIO_ENDPOINT="http://localhost:9000"
MINIO_BUCKET="bucket"
MINIO_ACCESS_KEY="minioadmin"
MINIO_SECRET_KEY="minioadmin"
PRIVATE_KEY_HEX="0101010101010101010101010101010101010101010101010101010101010101"

cleanup() {
  echo "[cleanup] Stopping backend services..."
  docker compose -f "$COMPOSE_FILE" down -v >/dev/null 2>&1 || true
}
trap cleanup EXIT

# ── 1. Start services ────────────────────────────────────────────────────────
echo "[1/8] Starting PostgreSQL + MinIO..."
docker compose -f "$COMPOSE_FILE" up -d postgres minio minio-setup >/dev/null

# ── 2. Wait for PostgreSQL ───────────────────────────────────────────────────
echo "[2/8] Waiting for PostgreSQL..."
for _ in $(seq 1 30); do
  STATUS=$(docker inspect --format='{{.State.Health.Status}}' edgesentry-rs-postgres 2>/dev/null || true)
  [[ "$STATUS" == "healthy" ]] && break
  sleep 1
done
[[ "${STATUS:-}" == "healthy" ]] || { echo "PostgreSQL did not become healthy"; exit 1; }
echo "PostgreSQL: healthy"

# ── 3. Wait for MinIO setup ──────────────────────────────────────────────────
echo "[3/8] Waiting for MinIO bucket setup..."
for _ in $(seq 1 30); do
  STATE=$(docker inspect --format='{{.State.Status}}' edgesentry-rs-minio-setup 2>/dev/null || true)
  CODE=$(docker inspect --format='{{.State.ExitCode}}' edgesentry-rs-minio-setup 2>/dev/null || true)
  [[ "$STATE" == "exited" && "$CODE" == "0" ]] && break
  sleep 1
done
[[ "${STATE:-}" == "exited" && "${CODE:-}" == "0" ]] || { echo "MinIO setup did not complete"; exit 1; }
echo "MinIO: ready"

# ── 4. Build CLI ─────────────────────────────────────────────────────────────
echo "[4/8] Building CLI (s3,postgres features)..."
cd "$ROOT_DIR"
cargo build -p edgesentry-rs --features s3,postgres --release >/dev/null

EDS="$ROOT_DIR/target/release/eds"

# ── 5. Generate chain, verify, and tamper detection ──────────────────────────
echo "[5/8] Generating and verifying chain..."
"$EDS" demo-lift-inspection \
  --device-id lift-01 \
  --private-key-hex "$PRIVATE_KEY_HEX" \
  --out-file "$RECORDS_FILE" \
  --payloads-file "$PAYLOADS_FILE" \
  >/dev/null

"$EDS" verify-chain --records-file "$RECORDS_FILE"

# Tamper and confirm detection
python3 - <<'PY'
import json, pathlib
src = pathlib.Path('/tmp/lift_inspection_records.json')
dst = pathlib.Path('/tmp/lift_inspection_records_tampered.json')
records = json.loads(src.read_text(encoding='utf-8'))
records[0]["payload_hash"][0] ^= 0x01
dst.write_text(json.dumps(records, indent=2), encoding='utf-8')
PY

set +e
"$EDS" verify-chain --records-file "$TAMPERED_FILE"
TAMPER_EXIT=$?
set -e
[[ "$TAMPER_EXIT" -ne 0 ]] || { echo "FAIL: tampered chain was accepted"; exit 1; }
echo "Tamper detection: PASSED"

# ── 6. Ingest via IngestService into PostgreSQL + MinIO ──────────────────────
echo "[6/8] Ingesting records via IngestService (PostgreSQL + MinIO)..."
(
  cd "$ROOT_DIR"
  "$EDS" demo-ingest \
    --records-file "$RECORDS_FILE" \
    --payloads-file "$PAYLOADS_FILE" \
    --device-id lift-01 \
    --private-key-hex "$PRIVATE_KEY_HEX" \
    --pg-url "$PG_URL" \
    --minio-endpoint "$MINIO_ENDPOINT" \
    --minio-bucket "$MINIO_BUCKET" \
    --minio-access-key "$MINIO_ACCESS_KEY" \
    --minio-secret-key "$MINIO_SECRET_KEY" \
    --reset \
    --tampered-records-file "$TAMPERED_FILE"
)

# ── 7. Verify persisted counts in PostgreSQL ─────────────────────────────────
echo "[7/8] Verifying persisted record counts in PostgreSQL..."
AUDIT_COUNT=$(docker exec -i edgesentry-rs-postgres psql -U trace -d trace_audit -tAc "SELECT COUNT(*) FROM audit_records;")
LOG_COUNT=$(docker exec -i edgesentry-rs-postgres psql -U trace -d trace_audit -tAc "SELECT COUNT(*) FROM operation_logs;")

# 3 valid records accepted + 3 tampered records rejected = 6 operation log entries
[[ "$AUDIT_COUNT" -eq 3 ]] || { echo "FAIL: expected 3 audit_records, got $AUDIT_COUNT"; exit 1; }
[[ "$LOG_COUNT" -eq 6 ]] || { echo "FAIL: expected 6 operation_logs, got $LOG_COUNT"; exit 1; }
echo "PostgreSQL: $AUDIT_COUNT audit records, $LOG_COUNT operation logs verified"

# ── 8. Run Rust S3 integration tests ─────────────────────────────────────────
echo "[8/8] Running Rust S3 integration tests..."
(
  cd "$ROOT_DIR"
  TEST_S3_ENDPOINT="$MINIO_ENDPOINT" \
  TEST_S3_ACCESS_KEY="$MINIO_ACCESS_KEY" \
  TEST_S3_SECRET_KEY="$MINIO_SECRET_KEY" \
  TEST_S3_BUCKET="$MINIO_BUCKET" \
  cargo test -p edgesentry-rs --features s3 --test s3_integration -- --nocapture
)

echo ""
echo "All integration tests passed."
