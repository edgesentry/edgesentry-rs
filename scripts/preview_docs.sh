#!/usr/bin/env bash
# Preview the full GitHub Pages site locally.
# Serves at http://localhost:8080/edgesentry-rs/ — same paths as production.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BOOK="$REPO_ROOT/docs/book"
SITE="$REPO_ROOT/_site"
PORT="${1:-8080}"

cd "$REPO_ROOT"

echo "==> Building docs..."
mdbook build docs/audit/en/
mdbook build docs/audit/ja/
mdbook build docs/inspect/en/
mdbook build docs/inspect/ja/

echo "==> Copying hub index pages..."
cp -r "$REPO_ROOT/docs/hub/." "$BOOK/"

echo "==> Assembling site under _site/edgesentry-rs/..."
rm -rf "$SITE"
mkdir -p "$SITE/edgesentry-rs"
cp -r "$BOOK"/. "$SITE/edgesentry-rs/"

echo ""
echo "==> Serving at http://localhost:${PORT}/edgesentry-rs/"
echo "    Press Ctrl+C to stop."
python3 -m http.server "$PORT" --directory "$SITE"
