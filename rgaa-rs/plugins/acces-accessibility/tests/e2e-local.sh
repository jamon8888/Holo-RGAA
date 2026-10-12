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

for bin in rgaa-mcp jq python3; do
  command -v "$bin" >/dev/null || { echo "SKIP: $bin not on PATH"; exit 0; }
done

# One JSON-RPC session per call: initialize, notify, then the request.
#
# stdin must stay OPEN until the response arrives. Piping the three lines in
# with printf closes it immediately; the server logs "input stream terminated",
# starts shutting down, and gives in-flight work only about five seconds to
# drain. Fast calls complete inside that window, so the bug hides — but
# `analyze` takes ~20s and its response never arrives, which this script
# previously reported as a cheerful "findings: 0".
rpc() {
  local method="$1" params="$2"
  METHOD="$method" PARAMS="$params" TIMEOUT="${RPC_TIMEOUT:-300}" python3 - <<'PYEOF'
import json, os, subprocess, sys

method, params = os.environ["METHOD"], os.environ["PARAMS"]
deadline = float(os.environ["TIMEOUT"])

proc = subprocess.Popen(
    ["rgaa-mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
    stderr=subprocess.DEVNULL, text=True, bufsize=1,
)

def send(obj):
    proc.stdin.write(json.dumps(obj) + "\n")
    proc.stdin.flush()

try:
    send({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
        "protocolVersion": "2024-11-05", "capabilities": {},
        "clientInfo": {"name": "e2e", "version": "0"}}})
    proc.stdout.readline()                      # initialize result
    send({"jsonrpc": "2.0", "method": "notifications/initialized"})
    send({"jsonrpc": "2.0", "id": 2, "method": method,
          "params": json.loads(params)})

    # Read until the response carrying our id. stdin stays open throughout.
    import threading
    out = {}
    def reader():
        for line in proc.stdout:
            try:
                msg = json.loads(line)
            except ValueError:
                continue
            if msg.get("id") == 2:
                out["msg"] = line.strip()
                return
    t = threading.Thread(target=reader, daemon=True)
    t.start()
    t.join(deadline)
    if "msg" not in out:
        print(json.dumps({"error": {"message": f"no response within {deadline:.0f}s"}}))
        sys.exit(0)
    print(out["msg"])
finally:
    try:
        proc.stdin.close()
    except Exception:
        pass
    try:
        proc.wait(timeout=10)
    except Exception:
        proc.kill()
PYEOF
}

call_tool() {
  rpc "tools/call" "{\"name\":\"$1\",\"arguments\":$2}"
}

# Name the binary under test. Two install mechanisms write to different
# prefixes (install.sh -> ~/.local/bin from a release tarball, cargo install ->
# ~/.cargo/bin from source) and ~/.local/bin usually wins on PATH, so a
# reinstall can appear to do nothing. A failure below must name the binary.
RESOLVED=$(command -v rgaa-mcp)
echo "— binary under test: $RESOLVED"
for alt in "$HOME/.local/bin/rgaa-mcp" "$HOME/.cargo/bin/rgaa-mcp"; do
  if [[ -f "$alt" && "$alt" != "$RESOLVED" ]]; then
    echo "   note: a second binary exists and is shadowed: $alt"
  fi
done

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
    else fail "MISSING $t — $RESOLVED is stale. Note a plain 'cargo install' writes to ~/.cargo/bin, which ~/.local/bin usually shadows; install into the winning prefix: cargo install --path rgaa-rs/crates/rgaa-mcp --root \"\$HOME/.local\" --force"; fi
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
# Must be a real server rejection, not our own timeout. The rpc helper emits
# {"error":{"message":"no response within Ns"}} when nothing comes back, and
# matching on `.error` alone would score that as a successful rejection test —
# a false pass of exactly the kind this script exists to prevent.
if printf '%s' "$MISSING_REQ" | jq -e '.error.message? // "" | test("no response within")' >/dev/null 2>&1; then
  fail "remediate check inconclusive: the call timed out, nothing was tested"
elif printf '%s' "$MISSING_REQ" | jq -e '(.result.isError == true) or ((.error | type) == "object")' >/dev/null 2>&1; then
  # Note: `.error | objects` would emit an empty stream when .error is null,
  # which annihilates the whole `or` and makes jq -e exit non-zero. Compare the
  # type instead.
  ok "rejects an issue missing summary/remediation"
else
  fail "accepted an issue missing summary/remediation"
fi

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
  # `length` on an object returns its key count, so assert the type before any
  # counting below depends on it.
  if printf '%s' "$AB" | jq -e '[.. | objects | select(has("findings")) | .findings | select(type=="array")] | length > 0' >/dev/null 2>&1; then
    ok "flat response shape (findings is an array at top level)"
  else
    fail "no top-level findings array — the shape may have switched, or the call failed: $(printf '%s' "$A" | head -c 200)"
  fi

  NF=$(printf '%s' "$AB" | jq '[.. | objects | select(has("findings")) | .findings] | first | length' 2>/dev/null)
  # Zero findings on an arbitrary page is far more likely to mean the call did
  # not complete than a flawless page. Never report it as a pass.
  if [[ "${NF:-0}" -gt 0 ]]; then
    ok "findings: $NF"
  else
    fail "findings: 0 — treat as a failed call, not a clean page. Check the raw response: $(printf '%s' "$A" | head -c 200)"
  fi

  # analyze still emits one dom_snapshot hash per scan, shared by every finding.
  # That is expected, and since #320 it is harmless: the fingerprint hashes the
  # finding's identity (rule, criterion_id, url, target, component_path) and no
  # longer touches evidence, so a shared hash cannot move it. Recorded rather
  # than asserted on, because the count legitimately varies with
  # ScreenshotPolicy. What must hold is the uniqueness check below.
  HASHES=$(printf '%s' "$AB" | jq -r '[.. | objects | select(has("evidence")) | .evidence[]?.hash] | unique | length' 2>/dev/null)
  if [[ "${NF:-0}" -gt 1 ]]; then
    ok "evidence hashes: ${HASHES:-0} distinct across ${NF} findings (scan-level is normal; excluded from the fingerprint since #320)"
  fi

  # criterion_id is often null: real axe findings outside the referential.
  UNMAPPED=$(printf '%s' "$AB" | jq '[.. | objects | select(has("criterion_id")) | select(.criterion_id == null)] | length' 2>/dev/null)
  ok "unmapped findings (criterion_id null): ${UNMAPPED:-0} of ${NF:-0}"

  # The loop's key must be unique, or matching silently merges findings. One axe
  # violation often maps to two RGAA criteria, so (rule,target) alone is not
  # enough and html does not disambiguate it.
  K2=$(printf '%s' "$AB" | jq '[.. | objects | select(has("criterion_id")) | [.rule,.target]] | unique | length' 2>/dev/null)
  K3=$(printf '%s' "$AB" | jq '[.. | objects | select(has("criterion_id")) | [.rule,.target,.criterion_id]] | unique | length' 2>/dev/null)
  if [[ "${K3:-0}" == "${NF:-0}" ]]; then
    ok "(rule,target,criterion_id) is unique: ${K3}/${NF} — (rule,target) alone gives ${K2}"
  else
    fail "(rule,target,criterion_id) is NOT unique: ${K3} keys for ${NF} findings — the loop's matching would merge findings"
  fi

  # Stability: re-scan the unedited page. Findings on positional or
  # runtime-generated selectors move between scans; those on stable selectors
  # must not. A difference among the stable ones means the loop cannot tell a
  # fix from noise.
  echo "— re-scanning to measure selector stability"
  A2=$(call_tool analyze "{\"url\":\"${URL}\"}")
  AB2=$(printf '%s' "$A2" | jq -r '.result.content[0].text // empty' 2>/dev/null)
  [[ -z "$AB2" ]] && AB2="$A2"
  stable_keys() {
    printf '%s' "$1" | jq -r '[.. | objects | select(has("criterion_id"))
      | select((.target | test(":nth-child|:nth-of-type|carousel|go[0-9]{6,}")) | not)
      | "\(.rule)|\(.target)|\(.criterion_id)"] | sort | .[]' 2>/dev/null
  }
  S1=$(stable_keys "$AB"); S2=$(stable_keys "$AB2")
  N1=$(printf '%s\n' "$S1" | grep -c . || true); N2=$(printf '%s\n' "$S2" | grep -c . || true)
  # An empty stable set is not agreement. Two empty sets compare equal, which
  # would report "reproduce exactly (0 findings)" when the second scan simply
  # failed. Require something to have been compared.
  if [[ "${N1:-0}" -eq 0 ]]; then
    fail "no stable-selector findings to compare — the re-scan probably failed; nothing was verified"
  elif [[ "$S1" == "$S2" ]]; then
    ok "stable-selector findings reproduce exactly across scans ($N1 findings)"
  else
    fail "stable-selector findings differ across two scans of an unedited page ($N1 vs $N2) — the loop cannot distinguish a fix from scan noise"
  fi
  ok "unstable-selector findings (excluded from convergence): $(( ${NF:-0} - N1 )) of ${NF:-0}"
else
  echo "— analyze skipped (pass a URL to exercise it)"
fi

echo
[[ $FAILURES -eq 0 ]] && echo "e2e OK" || echo "$FAILURES failure(s)"
exit $(( FAILURES > 0 ? 1 : 0 ))
