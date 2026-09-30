# Codex client example

A dependency-free Node client for the RGAA MCP HTTP transport. It shows the
whole path a third-party integrator needs: `initialize`, `tools/list`,
`tools/call`, and how to tell a declined call apart from an unreachable
server.

## Run it

Terminal 1 — the server:

```bash
cargo run -p rgaa-mcp-http -- --host 127.0.0.1 --port 3000
```

Terminal 2 — the client:

```bash
node examples/codex-client/client.mjs
```

Point it elsewhere with `RGAA_MCP_URL`:

```bash
RGAA_MCP_URL=http://127.0.0.1:8080 node examples/codex-client/client.mjs
```

## Expected output

```
→ http://127.0.0.1:3000
  server: rgaa-mcp-http 0.1.0
  protocol: 2025-03-26
  tools (6): analyze, remediate, igt, audit_url, get_audit_result, list_criteria
  list_criteria: 106 criteria
    e.g. 1.1 — … (Deterministe)

✓ end-to-end tool call succeeded
```

Exit code is `0` on success, `1` on any failure, so it works as a CI smoke
check.

## Why it calls `list_criteria`

It is the only tool that needs neither a browser nor network access, so it
proves the transport, the JSON-RPC envelope and the tool dispatch without
dragging in obscura or a live site. A smoke test that needs a working
browser is testing the browser.

## Two failure classes, kept apart

| What happened | How it looks | What it means |
|---|---|---|
| Connection refused, non-2xx | `transport: …` | The server is not there |
| JSON-RPC `error` envelope | `<method>: <message>` plus a `code` | The server is there and declined |

For a tool that needs a browser or the network, a declined call is an
ordinary outcome, not a broken integration. Treating the two the same is
how a smoke test ends up either useless or permanently red.

Match on `error.data.code` (the stable identifier — `INVALID_INPUT`,
`EXECUTION_FAILED`, `INCOMPLETE_RESULT`, …), never on the message text. See
the error reference in [`docs/plugin-integration.md`](../../docs/plugin-integration.md).

## CORS

The client talks to the server directly, so CORS does not apply and none is
needed. CORS fails closed by design: a **browser** page needs its origin
named in `RGAA_CORS_ORIGINS` or `--cors-origin`. Do not open it to `*` —
the endpoint is unauthenticated and `analyze` / `audit_url` fetch
caller-supplied URLs (CWE-942).
