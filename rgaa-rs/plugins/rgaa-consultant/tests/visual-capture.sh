#!/usr/bin/env bash
# visual-capture.sh — vérifie que scripts/visual/capture.mjs produit les
# captures attendues sur une page fixture locale (Chromium Playwright requis).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TMP="$(mktemp -d)"; trap 'rm -rf "$TMP"' EXIT
(cd "$ROOT/scripts/visual" && { [[ -d node_modules ]] || npm ci --no-audit --no-fund; } && npx playwright install ${CI:+--with-deps} chromium >/dev/null)
sed "s|__PAGE__|file://$ROOT/tests/fixtures/visual/page.html|" "$ROOT/tests/fixtures/visual/input.json" > "$TMP/input.json"
node "$ROOT/scripts/visual/capture.mjs" "$TMP/input.json" "$TMP/out"
fail=0
for f in index.html manifest.json criteres/1.1.png criteres/3.2.png \
         findings/fng_img/existant.png findings/fng_img/proposition-1.png findings/fng_img/proposition-2.png \
         findings/fng_contrast/existant.png findings/fng_contrast/proposition-1.png findings/fng_contrast/proposition-2.png; do
  [[ -s "$TMP/out/$f" ]] && echo "✅ $f" || { echo "❌ manquant : $f"; fail=1; }
done
jq -e '.findings[] | select(.id=="fng_missing") | .error' "$TMP/out/manifest.json" >/dev/null \
  && echo "✅ cible absente signalée" || { echo "❌ cible absente non signalée"; fail=1; }
jq -e '[.findings[].proposals[] | select(.error)] | length == 0' "$TMP/out/manifest.json" >/dev/null \
  && echo "✅ toutes les propositions appliquées" || { echo "❌ proposition en erreur"; fail=1; }
exit $fail
