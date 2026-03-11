#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
COMPOSE_FILE="$ROOT_DIR/docker-compose.local.yml"

# ════════════════════════════════════════════════════════════════════════════
# Three-role demo
#
# EDGE DEVICE   — examples/edge_device.rs
#                 Signs lift inspection records and writes them to /tmp/.
#
# EDGE GATEWAY  — examples/edge_gateway.rs
#                 Reads device output, logs each record, forwards unchanged.
#                 No cryptographic verification — routing only.
#
# CLOUD BACKEND — examples/cloud_backend.rs (--features s3,postgres)
#                 Applies NetworkPolicy (CLS-06), runs IntegrityPolicyGate,
#                 persists accepted records to PostgreSQL + MinIO.
# ════════════════════════════════════════════════════════════════════════════

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

# ── CLOUD BACKEND: start storage services ───────────────────────────────────

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

# ── EDGE DEVICE: sign records ────────────────────────────────────────────────

echo "[4/7] EDGE DEVICE: signing lift inspection records..."
(cd "$ROOT_DIR" && cargo run -p edgesentry-rs --example edge_device)
wait_for_ok "4/7"

# ── EDGE GATEWAY: forward records ───────────────────────────────────────────

echo "[5/7] EDGE GATEWAY: forwarding records to cloud backend..."
(cd "$ROOT_DIR" && cargo run -p edgesentry-rs --example edge_gateway)
wait_for_ok "5/7"

# ── CLOUD BACKEND: ingest via NetworkPolicy + IntegrityPolicyGate ───────────

echo "[6/7] CLOUD BACKEND: ingesting records (NetworkPolicy + IngestService → PostgreSQL + MinIO)..."
(cd "$ROOT_DIR" && cargo run -p edgesentry-rs --features s3,postgres --example cloud_backend)
wait_for_ok "6/7"

# ── CLOUD BACKEND: query persisted data ─────────────────────────────────────

echo "[7/7] CLOUD BACKEND: querying persisted audit records and operation log..."
docker exec -i edgesentry-rs-postgres psql -U trace -d trace_audit \
  -c "SELECT id, device_id, sequence, object_ref, ingested_at FROM audit_records ORDER BY sequence;"
docker exec -i edgesentry-rs-postgres psql -U trace -d trace_audit \
  -c "SELECT id, decision, device_id, sequence, message, created_at FROM operation_logs ORDER BY id;"
wait_for_ok "7/7"

echo "[8/8] Stopping PostgreSQL + MinIO..."
docker compose -f "$COMPOSE_FILE" down >/dev/null
echo "Backend stopped."

echo
echo "Done. Demo completed successfully."
