# Plugin integration guide

How a third-party client (Codex, an IDE extension, a CI job) drives the RGAA
backend. Everything here is checked against the source; where a capability
does not exist yet, this guide says so rather than describing it.

- **MCP tools** — the audit surface, over stdio or HTTP JSON-RPC.
- **REST API** — bundle storage, findings, policy evaluation.

Those are two different servers with two different auth stories. Read
[Authentication](#2-authentication) before wiring anything.

---

## 1. Setup

Build the binaries:

```bash
cd rgaa-rs
cargo build --release
```

| Binary | Crate | What it is |
|---|---|---|
| `rgaa-mcp` | `rgaa-mcp` | MCP server, **stdio** transport |
| `rgaa-mcp-http` | `rgaa-mcp-http` | MCP server, **HTTP JSON-RPC + SSE** |
| `rgaa-api` | `rgaa-api` | REST API |
| `rgaa-cli` | `rgaa-cli` | Local audit CLI |
| `rgaa` | `rgaa-tui` | Terminal UI |

### stdio transport

For a client that spawns the server as a child process (the Claude Code
plugin does this):

```json
{
  "mcpServers": {
    "rgaa-mcp": { "command": "rgaa-mcp", "args": [] }
  }
}
```

### HTTP transport

```bash
rgaa-mcp-http --host 127.0.0.1 --port 3000 --cors-origin https://your-plugin.example
```

| Flag | Env | Default |
|---|---|---|
| `--host` | `HOST` | `127.0.0.1` |
| `--port` | `PORT` | `3000` |
| `--cors-origin` | `RGAA_CORS_ORIGINS` (comma-separated) | *(none — see below)* |

**CORS fails closed.** With no allowlist, no cross-origin request is
granted. This is deliberate and must not be "fixed" by allowing any origin:
the endpoint is unauthenticated, `POST /mcp` reads the body as a string
without requiring a JSON content type, and a `text/plain` POST triggers no
preflight — so a page open in the user's browser could reach `tools/call`,
and `analyze` / `audit_url` would then fetch attacker-chosen URLs and return
the results (CWE-942). Name your origins explicitly.

CORS is not authentication. Anything that can reach the port directly,
rather than through a browser, is unaffected by it. Bind to loopback unless
you have put real authentication in front.

---

## 2. Authentication

**The MCP transports have no authentication today.** Not a token, not a
licence check — `rgaa-mcp` and `rgaa-mcp-http` accept any caller that can
reach them. Treat the port as privileged: bind to loopback, or put a
reverse proxy that authenticates in front of it.

A licence/auth middleware is tracked in #87 and is not implemented.

The **REST API** is split:

| Routes | Auth |
|---|---|
| `GET /health`, `GET /criteria` | none |
| `POST /audit`, `GET /audit/{id}` | **none** |
| `/v1/*` | `Authorization: Bearer <api-key>` |

`/v1/*` keys are validated against storage for the `audit:write` scope; a
missing or unknown key returns `401`.

Note `POST /audit` is unauthenticated *and* the REST API sets
`Access-Control-Allow-Origin: *`. Do not expose `rgaa-api` to a network you
do not control.

---

## 3. MCP tools

Six tools are registered. These names are what the server answers to —
verify against `#[tool(name = ...)]` in
`rgaa-rs/crates/rgaa-mcp/src/server.rs`, or just call `tools/list`.

| Tool | Input | Returns |
|---|---|---|
| `analyze` | `AnalyzeRequest` | Per-criterion findings for one page: `criterion_id`, `status`, `source`, evidence, justification |
| `audit_url` | `AuditUrlInput` | **Summary only**: `taux_global`, `etat_conformite`, `sampled_page_urls` |
| `get_audit_result` | `GetAuditInput` | A previously run audit, by `audit_id` |
| `list_criteria` | *(none)* | The 106 RGAA criteria: id, title, classification |
| `remediate` | `RemediationRequest` | Approval-gated fix proposals |
| `igt` | `GuidedTestRequest` | Guided keyboard test — **deprecated**, use `analyze` with `config.igt_tools: ["keyboard"]` |

Two things clients get wrong:

1. **`audit_url` does not return per-criterion detail.** It returns a
   site-level summary plus the URLs it sampled. To answer "which criterion
   failed and why", call `analyze` on each URL in `sampled_page_urls`.
2. **`Manuel` and `PartiellementAutomatable` criteria both surface as
   `NeedsReview`.** There is no separate status for "needs a human" —
   `NeedsReview` *is* that queue.

### Calling a tool over HTTP

`POST /mcp`, JSON-RPC 2.0:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": { "name": "list_criteria", "arguments": {} }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [{ "type": "text", "text": "…" }],
    "structuredContent": { "criteria": [ … ] },
    "isError": false
  }
}
```

Read `structuredContent`. The `content` array carries the same value
serialized as text, for clients that only render text.

Other methods: `initialize`, `tools/list`, and the notifications
`notifications/initialized` / `notifications/cancelled` (which answer `204`
with no body).

### Progress events

`GET /mcp/events` is a Server-Sent Events stream. Each tool call emits
`tool_started` and `tool_completed`, both carrying `{ "event": …, "tool": … }`.
Keep-alive every 15s.

The stream is a broadcast of *all* activity on the server, not a
per-request channel: it carries no request id, so a client running
concurrent calls cannot attribute an event to one of them. Subscribe before
issuing the call you want to watch.

---

## 4. REST API

| Method | Path | Auth | Notes |
|---|---|---|---|
| `GET` | `/health` | — | Returns the string `OK` (not JSON) |
| `GET` | `/criteria` | — | The 106 criteria |
| `POST` | `/audit` | — | Run an audit |
| `GET` | `/audit/{id}` | — | Fetch one; `404` if unknown |
| `POST` | `/v1/audit-bundles` | Bearer | Upload a bundle |
| `GET` | `/v1/audit-bundles` | Bearer | List |
| `GET` | `/v1/audit-bundles/{id}` | Bearer | Fetch one; `404` if unknown |
| `DELETE` | `/v1/audit-bundles/{id}` | Bearer | `204`; `400` if the id is not a UUID |
| `GET` | `/v1/findings` | Bearer | List findings |
| `POST` | `/v1/policy/evaluate` | Bearer | Evaluate policy |

`/audit` and `/audit/{id}` sit behind a concurrency limit with load
shedding and a timeout: under load they return `503` rather than queueing.
`/health` and `/criteria` deliberately sit outside that, so liveness stays
answerable while audits are being shed.

Batch endpoints are tracked in #167 / #168 and are not implemented.

---

## 5. Error reference

### MCP tool errors

A tool failure comes back as a JSON-RPC `error`. The message is prefixed
with a stable code, and the same code is repeated in `error.data.code` —
**match on `data.code`, never on the message text.**

| `data.code` | JSON-RPC code | Meaning |
|---|---|---|
| `INVALID_INPUT` | `-32602` | Arguments rejected (bad URL, bad shape) |
| `UNSUPPORTED_CONFIGURATION` | `-32602` | Config the server will not run (e.g. unsupported schema version) |
| `EMPTY_RESULT` | `-32602` | The run produced nothing usable |
| `POLICY_DENIED` | `-32603` | Refused by policy |
| `EXECUTION_FAILED` | `-32603` | Browser, network, LLM, storage, rate limit, timeout |
| `INCOMPLETE_RESULT` | `-32603` | Ran, but the evidence is incomplete — **not a pass** |

`INCOMPLETE_RESULT` is the one to handle deliberately. An analysis that
returns incomplete with no errors is rejected rather than reported clean: a
partial audit read as a passing audit is the failure mode that matters here.

Error messages are redacted before leaving the server — values under keys
that look secret (`password`, `secret`, `token`, …) are stripped.

### Transport-level JSON-RPC errors

| Code | When |
|---|---|
| `-32700` | Body is not valid JSON |
| `-32601` | Unknown `method` |
| `-32602` | `params` missing or undeserializable; unknown tool name |

### REST status codes

| Status | When |
|---|---|
| `200` / `204` | Success |
| `400` | Malformed path parameter (e.g. a non-UUID id) |
| `401` | Missing or unrecognised bearer key on `/v1/*` |
| `404` | Unknown id |
| `500` | Storage or internal failure |
| `503` | Shed by the concurrency limiter, or timed out |

---

## 6. TypeScript types

`types/` holds the published package. The MCP tool types are **generated
from the server**, not written by hand:

```bash
cargo run -p rgaa-mcp --bin dump-tool-schemas > types/schemas.json
python3 scripts/generate-ts-types.py
```

Hand-written client types rot silently, and a wrong type is worse than a
wrong doc because it compiles. This repo has already had that failure: the
plugin docs described three tools for a six-tool server and named three
that no server has ever registered (#161).

CI regenerates on every PR and fails if the **tool names** disagree with the
server's `#[tool(name = ...)]` registrations — that is the drift that would
lie to a consumer. The generated files are not committed: `release.yml`
regenerates from the binary it publishes, so the package always matches the
released server.

---

## 7. Worked example

`examples/codex-client/` is a runnable Node client that starts from nothing
and performs a real end-to-end tool call over HTTP. See its README.
