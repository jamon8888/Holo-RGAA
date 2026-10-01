#!/usr/bin/env bash
#
# Release smoke test for the MCP HTTP transport (issue #170).
#
# WHY THIS EXISTS
# ---------------
# Until now the release gate only ran `--version` / `--help` on each shipped
# binary. That proves the artifact is a runnable executable for the target
# triple and nothing else: a server that panics on startup, binds nothing, or
# registers zero tools passes `--help` happily. Several releases could have
# shipped an MCP transport that no client could actually talk to.
#
# So this spawns the real server and speaks real JSON-RPC to it over HTTP.
#
# PASS/FAIL SEMANTICS — read this before "fixing" a red run
# ---------------------------------------------------------
# Most tools need a browser (obscura) or the network to *succeed*. A release
# runner has neither, and wiring them in would make this a slow, flaky
# integration test instead of a smoke test. So the question asked here is
# deliberately narrower:
#
#     "is every registered tool reachable and does it answer?"
#
#   PASS  a JSON-RPC `result`                          (tool ran)
#   PASS  a JSON-RPC `error` envelope from the tool    (tool was dispatched
#         — e.g. -32602 for the missing arguments we deliberately send)
#   FAIL  no HTTP response at all                      (server died / not bound)
#   FAIL  -32601 method not found                      (transport lost tools/call)
#   FAIL  "tool not found"                             (tool vanished from the router)
#
# That classifier is the whole value of the script, so it is itself checked:
# a negative control calls a tool name that cannot exist and the run aborts
# unless the classifier rejects it. Without that control a bug that made
# everything "pass" would be invisible.
#
# The expected tool list is DERIVED FROM SOURCE (`#[tool(name = "...")]`),
# never hardcoded. Hardcoding rots the moment someone adds a seventh tool —
# it would ship untested and this script would still be green.
#
# Arguments are intentionally empty (`{}`) for every tool: no tool currently
# does browser or network work without required arguments, so empty arguments
# exercise dispatch without starting a real audit. Every call is also time
# boxed, so a future tool that *does* start long work fails loudly here rather
# than hanging the release job for six hours.
#
# CORS is not touched. It fails closed by design (issue #198 / CWE-942) and
# this script talks to the server directly rather than from a browser origin,
# so it never needs an allowlist. Do not add one to make a test pass.
#
# Usage:
#   scripts/smoke-mcp-http.sh --bin <path/to/rgaa-mcp-http[.exe]> \
#                             [--source <path/to/rgaa-mcp/src/server.rs>] \
#                             [--port N]

set -uo pipefail

BIN=""
SOURCE=""
PORT="${RGAA_SMOKE_PORT:-34117}"
# Generous: a cold binary on a loaded macOS runner can take a few seconds to
# bind. Cheap to wait, expensive to flake.
READY_TIMEOUT="${RGAA_SMOKE_READY_TIMEOUT:-30}"
CALL_TIMEOUT="${RGAA_SMOKE_CALL_TIMEOUT:-30}"

while [ $# -gt 0 ]; do
    case "$1" in
        --bin) BIN="$2"; shift 2 ;;
        --source) SOURCE="$2"; shift 2 ;;
        --port) PORT="$2"; shift 2 ;;
        -h|--help) sed -n '2,60p' "$0"; exit 0 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

[ -n "$BIN" ] || { echo "FATAL: --bin is required" >&2; exit 2; }
[ -x "$BIN" ] || { echo "FATAL: $BIN is not executable (lost the exec bit in an artifact round-trip?)" >&2; exit 2; }

if [ -z "$SOURCE" ]; then
    repo_root="$(cd "$(dirname "$0")/.." && pwd)"
    SOURCE="${repo_root}/rgaa-rs/crates/rgaa-mcp/src/server.rs"
fi
[ -f "$SOURCE" ] || { echo "FATAL: tool source of truth not found at $SOURCE" >&2; exit 2; }

command -v jq >/dev/null 2>&1 || { echo "FATAL: jq is required" >&2; exit 2; }
command -v curl >/dev/null 2>&1 || { echo "FATAL: curl is required" >&2; exit 2; }

ENDPOINT="http://127.0.0.1:${PORT}/mcp"
SERVER_LOG="$(mktemp)"
SERVER_PID=""

cleanup() {
    if [ -n "$SERVER_PID" ]; then
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
    rm -f "$SERVER_LOG"
}
trap cleanup EXIT INT TERM

# ── Expected tools, derived from the #[tool(...)] attributes ────────────────
# Handles both the single-line and the multi-line attribute spelling; the
# crate currently uses the multi-line one because the descriptions are long.
expected_tools="$(
    awk '
        /#\[tool\(/ { in_tool = 1 }
        in_tool && /name[[:space:]]*=[[:space:]]*"/ {
            line = $0
            sub(/^.*name[[:space:]]*=[[:space:]]*"/, "", line)
            sub(/".*$/, "", line)
            print line
            in_tool = 0
        }
    ' "$SOURCE" | sort
)"

if [ -z "$expected_tools" ]; then
    echo "FATAL: derived zero tools from $SOURCE — the attribute spelling changed" >&2
    echo "       and this smoke test would silently test nothing. Fix the parser." >&2
    exit 1
fi

echo "== expected tools (from $(basename "$SOURCE")) =="
echo "$expected_tools" | sed 's/^/   /'

# ── Spawn ───────────────────────────────────────────────────────────────────
echo
echo "== spawning $BIN on 127.0.0.1:${PORT} =="
HOST=127.0.0.1 PORT="$PORT" "$BIN" --host 127.0.0.1 --port "$PORT" >"$SERVER_LOG" 2>&1 &
SERVER_PID=$!

rpc() {
    # $1 = JSON body. Prints the response body; non-zero exit means no answer.
    curl -sS --max-time "$CALL_TIMEOUT" \
        -H 'content-type: application/json' \
        -d "$1" "$ENDPOINT"
}

# ── Readiness ───────────────────────────────────────────────────────────────
# tools/list is the probe on purpose: there is no /health endpoint today
# (one is being added separately under issue #160), and "the router answers
# with its tools" is a stronger readiness signal than "a socket accepted".
ready=""
deadline=$(( $(date +%s) + READY_TIMEOUT ))
while [ "$(date +%s)" -lt "$deadline" ]; do
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        echo "FAIL: server exited before becoming ready. Log:" >&2
        cat "$SERVER_LOG" >&2
        exit 1
    fi
    body="$(rpc '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' 2>/dev/null)"
    if [ -n "$body" ] && printf '%s' "$body" | jq -e '.result.tools' >/dev/null 2>&1; then
        ready="$body"
        break
    fi
    sleep 1
done

if [ -z "$ready" ]; then
    echo "FAIL: server never answered tools/list within ${READY_TIMEOUT}s. Log:" >&2
    cat "$SERVER_LOG" >&2
    exit 1
fi

live_tools="$(printf '%s' "$ready" | jq -r '.result.tools[].name' | sort)"
echo
echo "== tools/list over HTTP =="
echo "$live_tools" | sed 's/^/   /'

if [ "$live_tools" != "$expected_tools" ]; then
    echo
    echo "FAIL: the tools the running server exposes differ from the ones declared in source." >&2
    echo "      This is how a tool ships registered-but-unreachable, or reachable-but-undeclared." >&2
    diff <(echo "$expected_tools") <(echo "$live_tools") >&2 || true
    exit 1
fi

# ── Classifier ──────────────────────────────────────────────────────────────
# Returns 0 when the response proves the tool was reached.
classify() {
    local name="$1" body="$2"
    if [ -z "$body" ]; then
        echo "   FAIL ${name}: no HTTP response (server dead or connection refused)"
        return 1
    fi
    if printf '%s' "$body" | jq -e '.result' >/dev/null 2>&1; then
        echo "   PASS ${name}: result"
        return 0
    fi
    local code msg
    code="$(printf '%s' "$body" | jq -r '.error.code // empty')"
    msg="$(printf '%s' "$body" | jq -r '.error.message // empty')"
    if [ -z "$code" ]; then
        echo "   FAIL ${name}: neither result nor error envelope: ${body}"
        return 1
    fi
    if [ "$code" = "-32601" ]; then
        echo "   FAIL ${name}: -32601 method not found — tools/call is not wired up"
        return 1
    fi
    # rmcp answers an unknown tool name with invalid_params("tool not found"),
    # which is why the message and not just the code decides this.
    case "$msg" in
        *"tool not found"*|*"Tool not found"*|*"unknown tool"*|*"Unknown tool"*)
            echo "   FAIL ${name}: server does not know this tool (${msg})"
            return 1 ;;
    esac
    echo "   PASS ${name}: reachable, answered error ${code} (${msg})"
    return 0
}

# ── Negative control ────────────────────────────────────────────────────────
# If this is classified as a PASS the classifier is broken and every result
# below is meaningless, so abort rather than report a green run.
echo
echo "== negative control (a tool that cannot exist) =="
ctl_body="$(rpc '{"jsonrpc":"2.0","id":99,"method":"tools/call","params":{"name":"__rgaa_smoke_nonexistent__","arguments":{}}}')"
if classify "__rgaa_smoke_nonexistent__" "$ctl_body" >/dev/null 2>&1; then
    echo "FATAL: the pass/fail classifier accepted a tool that does not exist." >&2
    echo "       Everything this script reports would be a false pass. Response was:" >&2
    echo "       ${ctl_body}" >&2
    exit 1
fi
echo "   OK: classifier correctly rejects an unknown tool"

# ── Every tool over HTTP ────────────────────────────────────────────────────
echo
echo "== tools/call for every registered tool =="
failures=0
id=100
while IFS= read -r tool; do
    [ -n "$tool" ] || continue
    id=$((id + 1))
    req="$(jq -cn --arg n "$tool" --argjson i "$id" \
        '{jsonrpc:"2.0",id:$i,method:"tools/call",params:{name:$n,arguments:{}}}')"
    body="$(rpc "$req")"
    classify "$tool" "$body" || failures=$((failures + 1))
done <<EOF
$live_tools
EOF

echo
if [ "$failures" -ne 0 ]; then
    echo "FAIL: ${failures} tool(s) unreachable over HTTP. Server log:" >&2
    cat "$SERVER_LOG" >&2
    exit 1
fi

echo "OK: all $(echo "$live_tools" | wc -l | tr -d ' ') registered tools reachable over HTTP"
