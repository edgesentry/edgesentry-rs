CREATE TABLE IF NOT EXISTS audit_records (
  id BIGSERIAL PRIMARY KEY,
  device_id TEXT NOT NULL,
  sequence BIGINT NOT NULL,
  timestamp_ms BIGINT NOT NULL,
  payload_hash JSONB NOT NULL,
  signature JSONB NOT NULL,
  prev_record_hash JSONB NOT NULL,
  object_ref TEXT NOT NULL,
  ingested_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (device_id, sequence)
);

CREATE TABLE IF NOT EXISTS operation_logs (
  id BIGSERIAL PRIMARY KEY,
  decision TEXT NOT NULL,
  device_id TEXT NOT NULL,
  sequence BIGINT NOT NULL,
  message TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
