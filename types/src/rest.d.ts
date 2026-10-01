// REST API request/response types.
//
// Hand-written, unlike tools.d.ts: the axum handlers use plain serde
// derives with no schemars schema to generate from. Mirrors
// rgaa-rs/crates/rgaa-api/src/routes.rs — keep the two in step.

/** `POST /audit` — unauthenticated. */
export interface AuditRequest {
  url: string;
}

/** `POST /audit` and `GET /audit/{id}`. */
export interface AuditResponse {
  audit_id: string;
  url: string;
  /** Overall conformance rate. Never read without `coverage_percent`. */
  taux_global: number;
  /**
   * Share of the catalog actually evaluated.
   *
   * A high `taux_global` over a low coverage is not a high score — it is a
   * small sample. The two are published together for that reason.
   */
  coverage_percent: number;
  etat_conformite: string;
  passed: number;
  failed: number;
  na: number;
}

/** `GET /criteria` — unauthenticated. Returns all 106. */
export interface CriteriaResponse {
  id: string;
  title: string;
  /** `Deterministe` | `IaAssiste` | `Manuel` | `PartiellementAutomatable` */
  classification: string;
}

/** `POST /v1/audit-bundles` */
export interface CreateBundleRequest {
  bundle: unknown; // rgaa_core::AuditBundle — schema version "1.0"
}

export interface BundleResponse {
  audit_id: string;
  schema_version: string;
  /** ISO 8601 */
  created_at: string;
}

/** Query string for `GET /v1/audit-bundles` */
export interface ListBundlesQuery {
  limit?: number;
  offset?: number;
}

export interface BundleSummary {
  audit_id: string;
  url: string;
  schema_version: string;
  status: string;
  created_at: string;
}

export interface ListBundlesResponse {
  bundles: BundleSummary[];
}

/** Query string for `GET /v1/findings`. `audit_id` is required. */
export interface ListFindingsQuery {
  audit_id: string;
  status?: string;
  severity?: string;
  rule?: string;
  limit?: number;
  offset?: number;
}

export interface FindingsResponse {
  findings: unknown[]; // rgaa_storage::FindingRow
}

/** `POST /v1/policy/evaluate` */
export interface PolicyEvaluateRequest {
  bundle: unknown; // rgaa_core::AuditBundle
  baseline_audit_id?: string | null;
}

export interface PolicyEvaluateResponse {
  passed: boolean;
  failures: unknown[]; // rgaa_remediation::PolicyFailure
  warnings: unknown[]; // rgaa_remediation::PolicyWarning
  counts: unknown; // rgaa_remediation::PolicyCounts
}

/**
 * `/v1/*` requires `Authorization: Bearer <api-key>`, validated against
 * storage for the `audit:write` scope. Missing or unknown returns 401.
 *
 * The MCP transports have **no** authentication (#87). `POST /audit` has
 * none either, and the REST API sets `Access-Control-Allow-Origin: *` —
 * do not expose `rgaa-api` to a network you do not control.
 */
export type ApiKeyHeader = `Bearer ${string}`;
