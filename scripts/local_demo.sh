#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
COMPOSE_FILE="$ROOT_DIR/docker-compose.local.yml"
RECORDS_FILE="/tmp/lift_inspection_records.json"
INSERT_SQL="/tmp/insert_lift_records.sql"
TAMPERED_RECORDS_FILE="/tmp/lift_inspection_records_tampered.json"

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

echo "[1/8] Starting PostgreSQL + MinIO..."
docker compose -f "$COMPOSE_FILE" up -d postgres minio minio-setup >/dev/null
echo "Backend started."
wait_for_ok "1/8"

echo "[2/8] Waiting for PostgreSQL healthcheck..."
for _ in $(seq 1 30); do
  STATUS=$(docker inspect --format='{{.State.Health.Status}}' immutable-trace-postgres 2>/dev/null || true)
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
wait_for_ok "2/8"

echo "[3/8] Waiting for MinIO bucket setup..."
for _ in $(seq 1 30); do
  MINIO_SETUP_STATE=$(docker inspect --format='{{.State.Status}}' immutable-trace-minio-setup 2>/dev/null || true)
  MINIO_SETUP_EXIT_CODE=$(docker inspect --format='{{.State.ExitCode}}' immutable-trace-minio-setup 2>/dev/null || true)
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
wait_for_ok "3/8"

echo "[4/8] Generating demo lift inspection records..."
(
  cd "$ROOT_DIR"
  cargo run -p immutable-trace-audit-cli -- demo-lift-inspection --device-id lift-01 --out-file "$RECORDS_FILE" >/dev/null
)
echo "CLI check: chain generated -> $RECORDS_FILE"
cargo run -p immutable-trace-audit-cli -- verify-chain --records-file "$RECORDS_FILE"
wait_for_ok "4/8"

echo "[4.5/8] Tampering the chain and verifying detection..."
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
cargo run -p immutable-trace-audit-cli -- verify-chain --records-file "$TAMPERED_RECORDS_FILE"
TAMPER_EXIT_CODE=$?
set -e

if [[ "$TAMPER_EXIT_CODE" -eq 0 ]]; then
  echo "Tamper detection failed: tampered chain was accepted"
  exit 1
fi
echo "Tamper detection: PASSED (verify-chain exited with $TAMPER_EXIT_CODE)"
wait_for_ok "4.5/8"

echo "[5/8] Preparing SQL inserts..."
python3 - <<'PY'
import json
import pathlib

records_file = pathlib.Path('/tmp/lift_inspection_records.json')
insert_sql = pathlib.Path('/tmp/insert_lift_records.sql')
records = json.loads(records_file.read_text(encoding='utf-8'))

def esc(v: str) -> str:
    return v.replace("'", "''")

lines = [
    'BEGIN;',
    "TRUNCATE TABLE operation_logs, audit_records RESTART IDENTITY;",
]

for rec in records:
    device_id = esc(rec['device_id'])
    sequence = int(rec['sequence'])
    timestamp_ms = int(rec['timestamp_ms'])
    payload_hash = esc(json.dumps(rec['payload_hash']))
    signature = esc(json.dumps(rec['signature']))
    prev_hash = esc(json.dumps(rec['prev_record_hash']))
    object_ref = esc(rec['object_ref'])
    lines.append(
        "INSERT INTO audit_records "
        "(device_id, sequence, timestamp_ms, payload_hash, signature, prev_record_hash, object_ref) "
        f"VALUES ('{device_id}', {sequence}, {timestamp_ms}, '{payload_hash}'::jsonb, "
        f"'{signature}'::jsonb, '{prev_hash}'::jsonb, '{object_ref}');"
    )
    lines.append(
        "INSERT INTO operation_logs (decision, device_id, sequence, message) "
        f"VALUES ('Accepted', '{device_id}', {sequence}, 'ingest accepted via demo script');"
    )

lines.append('COMMIT;')
insert_sql.write_text("\n".join(lines) + "\n", encoding='utf-8')
PY
echo "SQL prepared: $INSERT_SQL"
wait_for_ok "5/8"

echo "[6/8] Inserting demo data into PostgreSQL..."
docker exec -i immutable-trace-postgres psql -U trace -d trace_audit < "$INSERT_SQL" >/dev/null
echo "Insert completed."
wait_for_ok "6/8"

echo "[7/8] Querying persisted data..."
docker exec -i immutable-trace-postgres psql -U trace -d trace_audit -c "SELECT id, device_id, sequence, object_ref, ingested_at FROM audit_records ORDER BY sequence;"
docker exec -i immutable-trace-postgres psql -U trace -d trace_audit -c "SELECT id, decision, device_id, sequence, message, created_at FROM operation_logs ORDER BY id;"
wait_for_ok "7/8"

echo "[8/8] Stopping PostgreSQL + MinIO..."
docker compose -f "$COMPOSE_FILE" down >/dev/null
echo "Backend stopped."
wait_for_ok "8/8"

echo
echo "Done. Demo completed successfully."
echo "PostgreSQL and MinIO have been stopped."
