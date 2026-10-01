// Public entry point for @rgaa/types.
//
// `tools.d.ts` is GENERATED from the server's own schemas and is NOT
// committed — run `python3 scripts/generate-ts-types.py` after
// `(cd rgaa-rs && cargo run -p rgaa-mcp --bin dump-tool-schemas) > types/schemas.json`.
// `release.yml` does both before publishing, so the published package always
// matches the released binary. `rest.d.ts` and `protocol.d.ts` are
// hand-written, because the REST handlers and the JSON-RPC envelope have no
// schemars source to derive from. That split is deliberate: the hand-written
// half is small and stable, the half that changes with every new tool is
// generated.

export * from "./protocol";
export * from "./rest";
export * from "./tools";
