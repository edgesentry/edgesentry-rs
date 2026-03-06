#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo command not found"
  echo "hint: source \"$HOME/.cargo/env\""
  exit 1
fi

if [[ ! -f "deny.toml" ]]; then
  echo "error: deny.toml not found in repository root"
  exit 1
fi

if ! cargo deny --version >/dev/null 2>&1; then
  echo "error: cargo-deny is not installed"
  echo "install with: cargo install cargo-deny"
  exit 1
fi

echo "[1/3] Running unit tests (workspace)..."
cargo test --workspace

echo "[2/3] Running ingest tests with s3 feature..."
cargo test -p ingest --features s3

echo "[3/3] Checking OSS licenses for commercial-use policy (deny.toml)..."
cargo deny check licenses

echo
echo "All checks passed."
echo "- Unit tests: OK"
echo "- OSS license policy (commercial-use allowlist): OK"
