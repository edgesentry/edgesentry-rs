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

echo "==> Creating hub index pages..."
mkdir -p "$BOOK/en" "$BOOK/ja"

cat > "$BOOK/index.html" <<'EOF'
<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8">
    <meta http-equiv="refresh" content="0; url=/edgesentry-rs/en/">
    <link rel="canonical" href="/edgesentry-rs/en/">
  </head>
  <body>
    <p>Redirecting to <a href="/edgesentry-rs/en/">documentation</a>...</p>
  </body>
</html>
EOF

cat > "$BOOK/en/index.html" <<'EOF'
<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8">
    <title>EdgeSentry — Documentation</title>
    <style>
      body { font-family: sans-serif; max-width: 640px; margin: 4rem auto; padding: 0 1rem; color: #222; }
      h1 { font-size: 1.6rem; margin-bottom: 0.25rem; }
      p.tagline { color: #555; margin-top: 0; }
      ul { list-style: none; padding: 0; margin: 2rem 0; }
      li { margin: 1rem 0; }
      a.card { display: block; padding: 1rem 1.25rem; border: 1px solid #d0d7de; border-radius: 6px; text-decoration: none; color: inherit; }
      a.card:hover { border-color: #3a7bd5; background: #f6f9ff; }
      a.card h2 { margin: 0 0 0.25rem; font-size: 1.1rem; color: #3a7bd5; }
      a.card p { margin: 0; font-size: 0.9rem; color: #555; }
      .lang { margin-top: 2rem; font-size: 0.85rem; color: #888; }
      .lang a { color: #3a7bd5; text-decoration: none; }
    </style>
  </head>
  <body>
    <h1>EdgeSentry</h1>
    <p class="tagline">Trust and verification for edge infrastructure.</p>
    <ul>
      <li>
        <a class="card" href="/edgesentry-rs/audit/en/">
          <h2>EdgeSentry-Audit</h2>
          <p>Ed25519 + BLAKE3 cryptographic audit trail for IoT devices and infrastructure.</p>
        </a>
      </li>
      <li>
        <a class="card" href="/edgesentry-rs/inspect/en/">
          <h2>EdgeSentry-Inspect</h2>
          <p>Edge-first 3D scan vs. reference deviation detection for construction and maritime inspection.</p>
        </a>
      </li>
    </ul>
    <p class="lang">🌐 <a href="/edgesentry-rs/ja/">日本語版</a></p>
  </body>
</html>
EOF

cat > "$BOOK/ja/index.html" <<'EOF'
<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8">
    <title>EdgeSentry — ドキュメント</title>
    <style>
      body { font-family: sans-serif; max-width: 640px; margin: 4rem auto; padding: 0 1rem; color: #222; }
      h1 { font-size: 1.6rem; margin-bottom: 0.25rem; }
      p.tagline { color: #555; margin-top: 0; }
      ul { list-style: none; padding: 0; margin: 2rem 0; }
      li { margin: 1rem 0; }
      a.card { display: block; padding: 1rem 1.25rem; border: 1px solid #d0d7de; border-radius: 6px; text-decoration: none; color: inherit; }
      a.card:hover { border-color: #3a7bd5; background: #f6f9ff; }
      a.card h2 { margin: 0 0 0.25rem; font-size: 1.1rem; color: #3a7bd5; }
      a.card p { margin: 0; font-size: 0.9rem; color: #555; }
      .lang { margin-top: 2rem; font-size: 0.85rem; color: #888; }
      .lang a { color: #3a7bd5; text-decoration: none; }
    </style>
  </head>
  <body>
    <h1>EdgeSentry</h1>
    <p class="tagline">エッジインフラへの信頼と検証</p>
    <ul>
      <li>
        <a class="card" href="/edgesentry-rs/audit/ja/">
          <h2>EdgeSentry-Audit</h2>
          <p>IoT デバイスおよびインフラ向け Ed25519 + BLAKE3 暗号監査証跡。</p>
        </a>
      </li>
      <li>
        <a class="card" href="/edgesentry-rs/inspect/ja/">
          <h2>EdgeSentry-Inspect</h2>
          <p>建設・海事点検向けエッジファースト 3D スキャン vs 参照逸脱検出。</p>
        </a>
      </li>
    </ul>
    <p class="lang">🌐 <a href="/edgesentry-rs/en/">English</a></p>
  </body>
</html>
EOF

echo "==> Assembling site under _site/edgesentry-rs/..."
rm -rf "$SITE"
mkdir -p "$SITE/edgesentry-rs"
cp -r "$BOOK"/. "$SITE/edgesentry-rs/"

echo ""
echo "==> Serving at http://localhost:${PORT}/edgesentry-rs/"
echo "    Press Ctrl+C to stop."
python3 -m http.server "$PORT" --directory "$SITE"
