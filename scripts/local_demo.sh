#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
COMPOSE_FILE="$ROOT_DIR/docker-compose.local.yml"
RECORDS_FILE="/tmp/lift_inspection_records.json"
PAYLOADS_FILE="/tmp/lift_inspection_payloads.json"
TAMPERED_RECORDS_FILE="/tmp/lift_inspection_records_tampered.json"
PG_URL="postgresql://trace:trace@localhost:5433/trace_audit"
MINIO_ENDPOINT="http://localhost:9000"
MINIO_BUCKET="bucket"
MINIO_ACCESS_KEY="minioadmin"
MINIO_SECRET_KEY="minioadmin"
PRIVATE_KEY_HEX="0101010101010101010101010101010101010101010101010101010101010101"

wait_for_ok() {
  local step_label="$1"
  while true; do
    printf "\n[%s] Press Enter to continue: " "$step_label"
    read -r answer
    if [[ -z "$answer" || "$answer" == "OK" ]]; then
      break
    fi
    echo "Please press Enter only, or type 'OK'."
  done
}

if ! command -v docker >/dev/null 2>&1; then
  echo "docker command not found"
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo command not found. Run: source \"$HOME/.cargo/env\""
  exit 1
fi

# Three-role model
# ════════════════════════════════════════════════════════════════════════════
# EDGE DEVICE   — lift-01 sensor (simulated by the eds CLI on this machine).
#                 Signs inspection records with an Ed25519 private key.
# EDGE GATEWAY  — not implemented in this demo; in production a gateway would
#                 forward signed records from the device to the cloud over
#                 HTTPS / MQTT.
# CLOUD BACKEND — PostgreSQL (audit ledger) + MinIO (raw payload store).
#                 Runs NetworkPolicy, IntegrityPolicyGate, and IngestService.
# ════════════════════════════════════════════════════════════════════════════

echo "[1/7] Starting PostgreSQL + MinIO..."
docker compose -f "$COMPOSE_FILE" up -d postgres minio minio-setup >/dev/null
echo "Backend started."
wait_for_ok "1/7"

echo "[2/7] Waiting for PostgreSQL healthcheck..."
for _ in $(seq 1 30); do
  STATUS=$(docker inspect --format='{{.State.Health.Status}}' edgesentry-rs-postgres 2>/dev/null || true)
  if [[ "$STATUS" == "healthy" ]]; then
    break
  fi
  sleep 1
done

if [[ "${STATUS:-}" != "healthy" ]]; then
  echo "PostgreSQL did not become healthy in time"
  exit 1
fi
echo "PostgreSQL healthcheck: healthy"
wait_for_ok "2/7"

echo "[3/7] Waiting for MinIO bucket setup..."
for _ in $(seq 1 30); do
  MINIO_SETUP_STATE=$(docker inspect --format='{{.State.Status}}' edgesentry-rs-minio-setup 2>/dev/null || true)
  MINIO_SETUP_EXIT_CODE=$(docker inspect --format='{{.State.ExitCode}}' edgesentry-rs-minio-setup 2>/dev/null || true)
  if [[ "$MINIO_SETUP_STATE" == "exited" && "$MINIO_SETUP_EXIT_CODE" == "0" ]]; then
    break
  fi
  sleep 1
done

if [[ "${MINIO_SETUP_STATE:-}" != "exited" || "${MINIO_SETUP_EXIT_CODE:-}" != "0" ]]; then
  echo "MinIO setup container did not complete successfully"
  exit 1
fi
echo "MinIO setup: completed"
wait_for_ok "3/7"

echo "──────────────────────────────────────────────────────────────────────────"
echo "EDGE DEVICE: generating and signing lift inspection records"
echo "──────────────────────────────────────────────────────────────────────────"
echo "[4/7] Generating demo lift inspection records + payloads..."
(
  cd "$ROOT_DIR"
  cargo run -p edgesentry-rs --features s3,postgres -- \
    demo-lift-inspection \
    --device-id lift-01 \
    --private-key-hex "$PRIVATE_KEY_HEX" \
    --out-file "$RECORDS_FILE" \
    --payloads-file "$PAYLOADS_FILE" \
    >/dev/null
)
echo "CLI check: chain generated -> $RECORDS_FILE"
echo "CLI check: payloads generated -> $PAYLOADS_FILE"
(cd "$ROOT_DIR" && cargo run -p edgesentry-rs -- verify-chain --records-file "$RECORDS_FILE")
wait_for_ok "4/7"

echo "──────────────────────────────────────────────────────────────────────────"
echo "EDGE DEVICE (tamper simulation): flipping a bit to simulate data corruption"
echo "──────────────────────────────────────────────────────────────────────────"
echo "[4.5/7] Tampering the chain and verifying detection..."
python3 - <<'PY'
import json
import pathlib

src = pathlib.Path('/tmp/lift_inspection_records.json')
dst = pathlib.Path('/tmp/lift_inspection_records_tampered.json')
records = json.loads(src.read_text(encoding='utf-8'))
records[0]["payload_hash"][0] ^= 0x01
dst.write_text(json.dumps(records, indent=2), encoding='utf-8')
print(dst)
PY

set +e
(cd "$ROOT_DIR" && cargo run -p edgesentry-rs -- verify-chain --records-file "$TAMPERED_RECORDS_FILE")
TAMPER_EXIT_CODE=$?
set -e

if [[ "$TAMPER_EXIT_CODE" -eq 0 ]]; then
  echo "Tamper detection failed: tampered chain was accepted"
  exit 1
fi
echo "Tamper detection: PASSED (verify-chain exited with $TAMPER_EXIT_CODE)"
wait_for_ok "4.5/7"

echo "──────────────────────────────────────────────────────────────────────────"
echo "CLOUD BACKEND: ingesting records through NetworkPolicy + IntegrityPolicyGate"
echo "(in production, records arrive here from the edge gateway over HTTPS/MQTT)"
echo "──────────────────────────────────────────────────────────────────────────"
echo "[5/7] Ingesting records via IngestService (PostgreSQL + MinIO)..."
(
  cd "$ROOT_DIR"
  cargo run -p edgesentry-rs --features s3,postgres -- \
    demo-ingest \
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
    --tampered-records-file "$TAMPERED_RECORDS_FILE"
)
wait_for_ok "5/7"

echo "──────────────────────────────────────────────────────────────────────────"
echo "CLOUD BACKEND: querying persisted audit records and operation log"
echo "──────────────────────────────────────────────────────────────────────────"
echo "[6/7] Querying persisted data..."
docker exec -i edgesentry-rs-postgres psql -U trace -d trace_audit \
  -c "SELECT id, device_id, sequence, object_ref, ingested_at FROM audit_records ORDER BY sequence;"
docker exec -i edgesentry-rs-postgres psql -U trace -d trace_audit \
  -c "SELECT id, decision, device_id, sequence, message, created_at FROM operation_logs ORDER BY id;"
wait_for_ok "6/7"

echo "[7/7] Stopping PostgreSQL + MinIO..."
docker compose -f "$COMPOSE_FILE" down >/dev/null
echo "Backend stopped."
wait_for_ok "7/7"

echo
echo "Done. Demo completed successfully."
echo "PostgreSQL and MinIO have been stopped."
