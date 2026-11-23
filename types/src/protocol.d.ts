// JSON-RPC envelope and error taxonomy for the MCP HTTP transport.
// Hand-written: the envelope is defined by the transport, not by schemars.

import type { LicenseErrorBody } from "./rest";

/** Stable failure classes. Match on these, never on the message text. */
export type RgaaErrorCode =
  | "INVALID_INPUT"
  | "POLICY_DENIED"
  | "UNSUPPORTED_CONFIGURATION"
  | "EXECUTION_FAILED"
  /**
   * The run completed but the evidence is incomplete.
   *
   * Not a pass. The server rejects an analysis that returns incomplete with
   * no errors rather than reporting it clean — a partial audit read as a
   * passing audit is the failure this code exists to prevent.
   */
  | "INCOMPLETE_RESULT"
  | "EMPTY_RESULT";

export interface JsonRpcRequest<P = unknown> {
  jsonrpc: "2.0";
  id?: number | string | null;
  method: string;
  params?: P;
}

/**
 * JSON-RPC error codes for licence refusals (#87). MCP clients on stdio never
 * see an HTTP status, so the class travels here; `error.data` is then a
 * `LicenseErrorBody` (discriminate on `data.code`).
 *
 * -32002: licence invalid, expired, or feature not in tier (HTTP 401 / 402).
 */
export type JsonRpcLicenseRequiredCode = -32002;
/** Daily quota used up (HTTP 429). */
export type JsonRpcQuotaExceededCode = -32003;

export interface JsonRpcError {
  /**
   * Transport codes: -32700 parse, -32601 method not found, -32602 invalid
   * params. Licence: -32002 licence required, -32003 quota exceeded.
   */
  code: number;
  /** Prefixed with the `RgaaErrorCode`; secret-looking values are redacted. */
  message: string;
  data?: { code?: RgaaErrorCode } & Record<string, unknown>;
}

/** A licence refusal on an MCP transport. */
export interface JsonRpcLicenseError {
  code: JsonRpcLicenseRequiredCode | JsonRpcQuotaExceededCode;
  message: string;
  data: LicenseErrorBody;
}

export interface JsonRpcResponse<R = unknown> {
  jsonrpc: "2.0";
  id: number | string | null;
  result?: R;
  error?: JsonRpcError | JsonRpcLicenseError;
}

/**
 * Result of `tools/call`.
 *
 * Read `structuredContent`. `content` carries the same value serialized as
 * text, for clients that only render text.
 */
export interface ToolCallResult<T = unknown> {
  content: Array<{ type: "text"; text: string }>;
  structuredContent: T;
  isError: boolean;
}

export interface InitializeResult {
  protocolVersion: string;
  capabilities: { tools?: Record<string, unknown> };
  serverInfo: { name: string; version: string };
}

export interface ToolDescriptor {
  name: string;
  description?: string | null;
  inputSchema: Record<string, unknown>;
}

export interface ToolsListResult {
  tools: ToolDescriptor[];
}

/**
 * An event on `GET /mcp/events`.
 *
 * The stream is a broadcast of all activity on the server, not a
 * per-request channel: there is no request id, so concurrent calls cannot
 * be told apart. Subscribe before issuing the call you want to watch.
 */
export interface ProgressEvent {
  event: "tool_started" | "tool_completed";
  tool: string;
}
