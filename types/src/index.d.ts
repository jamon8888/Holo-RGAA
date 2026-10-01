// Public entry point for @rgaa/types.
//
// `tools.d.ts` is GENERATED from the server's own schemas — see
// scripts/generate-ts-types.py. `rest.d.ts` and `protocol.d.ts` are
// hand-written, because the REST handlers and the JSON-RPC envelope have no
// schemars source to derive from. That split is deliberate: the hand-written
// half is small and stable, the half that changes with every new tool is
// generated.

export * from "./protocol";
export * from "./rest";
export * from "./tools";
