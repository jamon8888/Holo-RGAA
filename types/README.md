# @holo-rgaa/mcp-types

TypeScript types for the RGAA backend:

- **MCP tools** — `rgaa-mcp` over stdio, `rgaa-mcp-http` over `POST /mcp`
- **REST API** — `rgaa-api`

## Layout

| File | Origin |
|---|---|
| `src/tools.d.ts` | **Generated** from the server's own `schemars` schemas |
| `src/protocol.d.ts` | Hand-written — the JSON-RPC envelope is defined by the transport |
| `src/rest.d.ts` | Hand-written — the axum handlers have no schemars source |

`tools.d.ts` is generated on purpose. The tool arguments are declared once,
in Rust; a hand-written copy is a second declaration that nothing keeps in
step, and a stale *type* is worse than a stale doc because it compiles. This
repo has already shipped that failure — plugin docs describing three tools
for a six-tool server, three of them under names no server has ever
registered (#161).

## Regenerating

```bash
(cd rgaa-rs && cargo run -p rgaa-mcp --bin dump-tool-schemas) > types/schemas.json
python3 scripts/generate-ts-types.py
```

CI regenerates and fails if the committed output differs, so the types
cannot drift from the server without someone noticing.

## Versioning

`version` here is a committed placeholder (`0.0.0`), overwritten at publish
time from the release tag by `.github/workflows/release.yml`. Do not
hand-edit it expecting it to be what ships — a hand-maintained version is a
version that is wrong on the release where it matters.
