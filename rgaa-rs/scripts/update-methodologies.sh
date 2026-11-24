#!/usr/bin/env bash
# scripts/update-methodologies.sh — Re-fetch RGAA/methodologies.json from the
# official DISIC/accessibilite.numerique.gouv.fr repository at the pinned
# commit, verify its SHA-256, and embed it under
# rgaa-core/data/rgaa-4.1.2/methodologies.json.
#
# Usage: ./scripts/update-methodologies.sh
#
# The pinned commit and expected SHA-256 live in
# crates/rgaa-core/src/methodologies.rs (METHODOLOGIES_COMMIT /
# METHODOLOGIES_SHA256). This script re-reads them from there so the script,
# the crate and the embedded file can never drift apart.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$REPO_ROOT/crates/rgaa-core/src/methodologies.rs"
OUT="$REPO_ROOT/crates/rgaa-core/data/rgaa-4.1.2/methodologies.json"

# Read the pinned commit and expected SHA-256 from the Rust source.
COMMIT="$(grep -n 'pub const METHODOLOGIES_COMMIT: &str' "$SRC" | sed -E 's/.*"([0-9a-f]+)".*/\1/')"
EXPECTED_SHA="$(grep -n 'pub const METHODOLOGIES_SHA256: &str' "$SRC" | sed -E 's/.*"([0-9a-f]+)".*/\1/')"

if [[ -z "$COMMIT" || -z "$EXPECTED_SHA" ]]; then
  echo "❌ could not read METHODOLOGIES_COMMIT / METHODOLOGIES_SHA256 from $SRC" >&2
  exit 1
fi

URL="https://raw.githubusercontent.com/DISIC/accessibilite.numerique.gouv.fr/$COMMIT/RGAA/methodologies.json"

echo "Fetching $URL"
TMP="$(mktemp)"
if ! curl -fsSL "$URL" -o "$TMP"; then
  echo "❌ fetch failed" >&2
  rm -f "$TMP"
  exit 1
fi

GOT_SHA="$(sha256sum "$TMP" | awk '{print $1}')"
echo "expected: $EXPECTED_SHA"
echo "got:      $GOT_SHA"

if [[ "$GOT_SHA" != "$EXPECTED_SHA" ]]; then
  echo "❌ SHA-256 mismatch — aborting, $OUT left untouched" >&2
  rm -f "$TMP"
  exit 1
fi

mkdir -p "$(dirname "$OUT")"
mv "$TMP" "$OUT"
echo "✓ wrote $OUT (commit $COMMIT, sha256 $GOT_SHA)"
