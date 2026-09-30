/**
 * TypeScript types for the RGAA MCP server's JSON-RPC surface.
 *
 * SCOPE NOTE: this file is deliberately thin. Issue #170 set up the release
 * plumbing — publishing this package at the release version and checking that
 * the MCP server and the CLI report the same version. Writing out the full
 * per-tool argument and result types is issue #169, and it lands here.
 *
 * Until then, tool arguments and results are typed as `unknown` rather than
 * `any`: a wrong-but-convenient type is worse than no type, because it makes
 * callers stop checking.
 */

/** Tool names registered by the MCP server. Source of truth: the
 *  `#[tool(name = "...")]` attributes in `rgaa-rs/crates/rgaa-mcp/src/server.rs`.
 *  The release smoke test derives the same list from that file and asserts the
 *  running server exposes exactly it. */
export type RgaaToolName =
  | "analyze"
  | "audit_url"
  | "get_audit_result"
  | "igt"
  | "list_criteria"
  | "remediate";

export interface JsonRpcRequest<TParams = unknown> {
  jsonrpc: "2.0";
  id: number | string;
  method: string;
  params?: TParams;
}

export interface JsonRpcError {
  code: number;
  message: string;
  data?: unknown;
}

export interface JsonRpcResponse<TResult = unknown> {
  jsonrpc: "2.0";
  id: number | string | null;
  result?: TResult;
  error?: JsonRpcError;
}

export interface ToolsCallParams {
  name: RgaaToolName;
  arguments?: Record<string, unknown>;
}

export interface ToolDescriptor {
  name: RgaaToolName;
  description?: string;
  inputSchema: Record<string, unknown>;
}

export interface ToolsListResult {
  tools: ToolDescriptor[];
}

/** Result of `tools/call`. `structuredContent` carries the typed payload;
 *  per-tool shapes arrive with issue #169. */
export interface ToolsCallResult {
  isError: boolean;
  content?: unknown[];
  structuredContent?: unknown;
}
