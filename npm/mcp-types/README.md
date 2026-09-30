# @holo-rgaa/mcp-types

TypeScript types for the RGAA MCP server's JSON-RPC surface
(`rgaa-mcp` over stdio, `rgaa-mcp-http` over `POST /mcp`).

Published automatically by `.github/workflows/release.yml` at the release
version. The version in this directory's `package.json` is a placeholder
(`0.0.0`) and is overwritten at publish time from the release tag — do not
hand-edit it expecting it to be what ships.

The type definitions themselves are currently minimal: only the JSON-RPC
envelopes and the tool-name union. Full per-tool argument and result types are
tracked in issue #169.
