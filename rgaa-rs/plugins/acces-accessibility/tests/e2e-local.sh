#!/usr/bin/env bash
# e2e-local.sh — Exercise the real rgaa-mcp contract over stdio.
#
# This is not a build runner. It speaks JSON-RPC to the installed server and
# asserts the claims the plugin's skills rely on. It exists because four skills
# were written from a source read, and the first real run against the server
# found five documentation errors plus a stale binary missing two tools.
#
#   ./tests/e2e-local.sh                      # offline assertions only
#   ./tests/e2e-local.sh https://example.org  # also exercise analyze

set -uo pipefail

PLUGIN_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
URL="${1:-}"
FAILURES=0

ok()   { echo "✅ $1"; }
fail() { echo "❌ $1"; FAILURES=$((FAILURES + 1)); }

for bin in rgaa-mcp jq; do
  command -v "$bin" >/dev/null || { echo "SKIP: $bin not on PATH"; exit 0; }
done

# One JSON-RPC session per call: initialize, notify, then the request.
rpc() {
  local method="$1" params="$2"
  printf '%s\n' \
    '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"e2e","version":"0"}}}' \
    '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
    "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"${method}\",\"params\":${params}}" \
    | timeout 180 rgaa-mcp 2>/dev/null | tail -1
}

call_tool() {
  rpc "tools/call" "{\"name\":\"$1\",\"arguments\":$2}"
}

echo "— the advertised tool set"
# A short list means a stale binary. This is the check that would have caught
# an installed server predating source_map and lint_static, on which the
# documented analyze -> source_map -> remediate loop cannot run at all.
TOOLS=$(rpc "tools/list" "{}" | jq -r '.result.tools[].name' 2>/dev/null | sort | tr '\n' ' ')
if [[ -z "$TOOLS" ]]; then
  fail "server did not answer tools/list"
else
  ok "tools/list answered: $TOOLS"
  for t in analyze remediate igt audit_url get_audit_result list_criteria lint_static source_map verify_fix; do
    if [[ " $TOOLS " == *" $t "* ]]; then ok "advertises $t"
    else fail "MISSING $t — the installed binary is stale; reinstall with: cargo install --path rgaa-rs/crates/rgaa-mcp --force"; fi
  done
fi

echo "— list_criteria"
CRIT=$(call_tool list_criteria '{}')
BODY=$(printf '%s' "$CRIT" | jq -r '.result.content[0].text // empty' 2>/dev/null)
[[ -z "$BODY" ]] && BODY="$CRIT"
COUNT=$(printf '%s' "$BODY" | jq '[.. | objects | select(has("criteria")) | .criteria] | first | length' 2>/dev/null)
[[ "$COUNT" == "106" ]] && ok "returns 106 criteria" || fail "expected 106 criteria, got: ${COUNT:-none}"

# Capitalised Debug spellings, not snake_case. The skills depend on this.
CLASSES=$(printf '%s' "$BODY" | jq -r '[.. | objects | select(has("classification")) | .classification] | unique | join(",")' 2>/dev/null)
if [[ "$CLASSES" == "Deterministe,IaAssiste,Manuel" ]]; then
  ok "classification values are capitalised: $CLASSES"
else
  fail "unexpected classification values: ${CLASSES:-none}"
fi

echo "— remediate rejects what the docs say it rejects"
# summary and remediation are required. Omitting them must be an error; the
# plugin's field-mapping table once left them out.
MISSING_REQ=$(call_tool remediate '{"issues":[{"id":"a","rule":"image-alt","element_html":"<img>","page_url":"https://example.org"}]}')
printf '%s' "$MISSING_REQ" | jq -e '.error // (.result.isError == true)' >/dev/null 2>&1 \
  && ok "rejects an issue missing summary/remediation" \
  || fail "accepted an issue missing summary/remediation"

# Empty source_locations fails the issue, which is what makes source_map
# mandatory. Per-issue failure, not call failure.
EMPTY_LOC=$(call_tool remediate '{"issues":[{"id":"a","rule":"image-alt","element_html":"<img src=\"x.png\">","page_url":"https://example.org","summary":"s","remediation":"r"}]}')
if printf '%s' "$EMPTY_LOC" | grep -qi "source_location\|MissingSourceLocation\|needs_review\|NeedsReview"; then
  ok "empty source_locations does not silently succeed"
else
  fail "empty source_locations produced no visible objection"
fi

if [[ -n "$URL" ]]; then
  echo "— analyze $URL"
  A=$(call_tool analyze "{\"url\":\"${URL}\"}")
  AB=$(printf '%s' "$A" | jq -r '.result.content[0].text // empty' 2>/dev/null)
  [[ -z "$AB" ]] && AB="$A"

  # Flat shape when igt_tools is unset: findings at top level, not data.axe.
  printf '%s' "$AB" | jq -e '.. | objects | select(has("findings"))' >/dev/null 2>&1 \
    && ok "flat response shape (findings at top level)" \
    || fail "no top-level findings — shape may have switched"

  NF=$(printf '%s' "$AB" | jq '[.. | objects | select(has("findings")) | .findings] | first | length' 2>/dev/null)
  ok "findings: ${NF:-0}"

  # The fingerprint defect: one dom_snapshot hash per scan, shared by every
  # finding. While this holds, verify_fix's diff cannot be trusted after an
  # edit, and the skills must compare on rule + target themselves.
  HASHES=$(printf '%s' "$AB" | jq -r '[.. | objects | select(has("evidence")) | .evidence[]?.hash] | unique | length' 2>/dev/null)
  if [[ "${HASHES:-0}" == "1" && "${NF:-0}" -gt 1 ]]; then
    fail "KNOWN DEFECT still present: all ${NF} findings share one evidence hash, so every fingerprint moves when the page changes (see field-mapping.md)"
  elif [[ "${NF:-0}" -gt 1 ]]; then
    ok "evidence hashes are per-finding (${HASHES} distinct) — the defect may be fixed; drop the caveats from usage/ and audit-loop/"
  fi

  # criterion_id is often null: real axe findings outside the referential.
  UNMAPPED=$(printf '%s' "$AB" | jq '[.. | objects | select(has("criterion_id")) | select(.criterion_id == null)] | length' 2>/dev/null)
  ok "unmapped findings (criterion_id null): ${UNMAPPED:-0} of ${NF:-0}"
else
  echo "— analyze skipped (pass a URL to exercise it)"
fi

echo
[[ $FAILURES -eq 0 ]] && echo "e2e OK" || echo "$FAILURES failure(s)"
exit $(( FAILURES > 0 ? 1 : 0 ))
