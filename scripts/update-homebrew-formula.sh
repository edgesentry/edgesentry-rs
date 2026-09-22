#!/usr/bin/env bash
# update-homebrew-formula.sh — regenerate Formula/eds.rb for a given release tag
# and open a PR against edgesentry/homebrew-tap (direct pushes to main are blocked).
#
# Usage:
#   HOMEBREW_TAP_TOKEN=<pat> ./scripts/update-homebrew-formula.sh v1.2.3
#
# Requirements:
#   - curl, sha256sum (or shasum on macOS), git, gh
#   - HOMEBREW_TAP_TOKEN env var with write access to edgesentry/homebrew-tap
set -euo pipefail

TAG_NAME="${1:?Usage: $0 <tag> (e.g. v1.2.3)}"
GH_TOKEN="${HOMEBREW_TAP_TOKEN:?HOMEBREW_TAP_TOKEN must be set}"
export GH_TOKEN

BASE_URL="https://github.com/edgesentry/edgesentry-rs/releases/download/${TAG_NAME}"
MACOS_ASSET="eds-${TAG_NAME}-aarch64-apple-darwin.tar.gz"
LINUX_ASSET="eds-${TAG_NAME}-x86_64-unknown-linux-gnu.tar.gz"
BRANCH="eds-${TAG_NAME}"

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "${WORK_DIR}"' EXIT
cd "${WORK_DIR}"

echo "Downloading release assets for ${TAG_NAME}..."
curl -fsSL -o "${MACOS_ASSET}" "${BASE_URL}/${MACOS_ASSET}"
curl -fsSL -o "${LINUX_ASSET}" "${BASE_URL}/${LINUX_ASSET}"

# sha256sum on Linux; shasum -a 256 on macOS
if command -v sha256sum &>/dev/null; then
  SHA256_MACOS=$(sha256sum "${MACOS_ASSET}" | awk '{print $1}')
  SHA256_LINUX=$(sha256sum "${LINUX_ASSET}" | awk '{print $1}')
else
  SHA256_MACOS=$(shasum -a 256 "${MACOS_ASSET}" | awk '{print $1}')
  SHA256_LINUX=$(shasum -a 256 "${LINUX_ASSET}" | awk '{print $1}')
fi

echo "macOS SHA256 : ${SHA256_MACOS}"
echo "Linux  SHA256: ${SHA256_LINUX}"

cat > eds.rb <<FORMULA
class Eds < Formula
  desc "EdgeSentry unified CLI for tamper-evident audit and IFC scan inspection"
  homepage "https://github.com/edgesentry/edgesentry-rs"
  version "${TAG_NAME#v}"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
      url "${BASE_URL}/${MACOS_ASSET}"
      sha256 "${SHA256_MACOS}"
    end
  end

  on_linux do
    on_intel do
      url "${BASE_URL}/${LINUX_ASSET}"
      sha256 "${SHA256_LINUX}"
    end
  end

  def install
    bin.install "eds"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/eds --version")
  end
end
FORMULA

echo "Cloning edgesentry/homebrew-tap..."
git clone "https://x-access-token:${GH_TOKEN}@github.com/edgesentry/homebrew-tap.git" tap
mkdir -p tap/Formula
cp eds.rb tap/Formula/eds.rb

cd tap
git config user.email "github-actions[bot]@users.noreply.github.com"
git config user.name "github-actions[bot]"

# Idempotent: reuse branch if a previous attempt left one.
if git ls-remote --exit-code --heads origin "${BRANCH}" >/dev/null 2>&1; then
  git fetch origin "${BRANCH}"
  git checkout -B "${BRANCH}" "origin/${BRANCH}"
else
  git checkout -B "${BRANCH}"
fi

git add Formula/eds.rb
if git diff --cached --quiet; then
  echo "Formula/eds.rb already up to date for ${TAG_NAME}."
else
  git commit -m "eds ${TAG_NAME}"
  git push -u origin "${BRANCH}"
fi

# Open or reuse PR.
if gh pr list --repo edgesentry/homebrew-tap --head "${BRANCH}" --state open --json number --jq 'length' | grep -qx '0'; then
  gh pr create --repo edgesentry/homebrew-tap \
    --base main \
    --head "${BRANCH}" \
    --title "eds ${TAG_NAME}" \
    --body "Update \`Formula/eds.rb\` for edgesentry-rs ${TAG_NAME}."
else
  echo "PR for ${BRANCH} already open."
  gh pr list --repo edgesentry/homebrew-tap --head "${BRANCH}" --state open
fi
