// REST API request/response types.
//
// Hand-written, unlike tools.d.ts: the axum handlers use plain serde
// derives with no schemars schema to generate from. Mirrors
// rgaa-rs/crates/rgaa-api/src/routes.rs — keep the two in step.

/**
 * `POST /audit` — unauthenticated, not licence-checked.
 * @deprecated Use `/v1/*`; the legacy audit routes bypass the licence gate.
 */
export interface AuditRequest {
  url: string;
}

/**
 * `POST /audit` and `GET /audit/{id}`.
 * @deprecated Legacy routes, not licence-checked; use `/v1/*`.
 */
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
 * Licensing (#87) is separate from that key: the *server* holds a licence
 * (`RGAA_LICENSE_KEY` or `rgaa licence activate`), and gated routes answer
 * with {@link LicenseHeaders} and {@link RateLimitHeaders}, or refuse with
 * 401 / 402 / 429 and a {@link LicenseErrorBody}. Clients send nothing extra.
 *
 * The legacy `POST /audit` and `GET /audit/{id}` routes are **not**
 * licence-checked and are deprecated; use `/v1/*`. They also have no
 * authentication, and the REST API sets `Access-Control-Allow-Origin: *` —
 * do not expose `rgaa-api` to a network you do not control.
 */
export type ApiKeyHeader = `Bearer ${string}`;

// ─── Licensing (#87) ────────────────────────────────────────────────────────
// Mirrors rgaa-rs/crates/rgaa-license (tiers.rs, gate.rs, error.rs).

/** Commercial tier. An installation without a licence runs as `"free"`. */
export type LicenseTier = "free" | "professional" | "team";

/** `X-License-Status`. `grace_exceeded` and `expired` run as `"free"`. */
export type LicenseStatus =
  | "unlicensed"
  | "valid"
  | "grace"
  | "expired"
  | "grace_exceeded"
  | "invalid";

/** A gated capability. */
export type LicenseFeature =
  | "static_lint"
  | "full_audit"
  | "remediation"
  | "igt"
  | "batch";

/**
 * Sent on every gated response, admitted or refused. Header names are
 * case-insensitive; they are listed in CORS `Access-Control-Expose-Headers`.
 */
export interface LicenseHeaders {
  "x-license-tier": LicenseTier;
  "x-license-status": LicenseStatus;
  /** Seconds of offline grace left. Only when status is `grace`. */
  "x-license-grace-remaining"?: string;
}

/**
 * Daily quota position. Present when the server enforces quotas (HTTP
 * servers do; local CLI and stdio do not). Values are decimal strings.
 */
export interface RateLimitHeaders {
  "x-ratelimit-limit": string;
  "x-ratelimit-remaining": string;
  /** Unix seconds of the next reset (00:00 UTC). */
  "x-ratelimit-reset": string;
  /** Seconds to wait. Only on 429 `quota_exceeded`. */
  "retry-after"?: string;
}

interface LicenseErrorBase {
  /** Human-readable, English. Old clients reading only `error` keep working. */
  error: string;
  /** Effective tier at the time of the refusal. */
  tier: LicenseTier;
  feature: LicenseFeature;
}

/** 401: a licence is installed but unusable (bad signature, malformed, not yet valid). */
export interface LicenseInvalidError extends LicenseErrorBase {
  code: "license_invalid";
}

/** 402: the licence expired, or the offline grace period ran out. */
export interface LicenseExpiredError extends LicenseErrorBase {
  code: "license_expired";
  upgrade_url?: string;
}

/** 402: the feature needs a higher tier. Waiting does not help: no `Retry-After`. */
export interface FeatureNotInTierError extends LicenseErrorBase {
  code: "feature_not_in_tier";
  required_tier: LicenseTier;
  upgrade_url?: string;
}

/** 429: the daily quota is used up. */
export interface QuotaExceededError extends LicenseErrorBase {
  code: "quota_exceeded";
  limit: number;
  remaining: number;
  /** ISO 8601, next 00:00 UTC. */
  reset_at: string;
  /** Seconds; same value as the `Retry-After` header. */
  retry_after: number;
}

/**
 * Body of a licence refusal (401 / 402 / 429). Discriminate on `code`.
 * The same object is the JSON-RPC `error.data` on the MCP transports.
 */
export type LicenseErrorBody =
  | LicenseInvalidError
  | LicenseExpiredError
  | FeatureNotInTierError
  | QuotaExceededError;

/** HTTP status for each licence error code. */
export interface LicenseErrorStatus {
  license_invalid: 401;
  license_expired: 402;
  feature_not_in_tier: 402;
  quota_exceeded: 429;
}
