// GENERATED FILE — DO NOT EDIT.
//
// Produced by scripts/generate-ts-types.py from types/schemas.json, which is
// itself produced by:
//
//     cargo run -p rgaa-mcp --bin dump-tool-schemas > types/schemas.json
//
// Edit the Rust types and regenerate. CI fails if this file does not match
// what the server currently registers.

/** Every tool name the server registers. */
export type RgaaToolName =
  | "analyze"
  | "audit_url"
  | "get_audit_result"
  | "igt"
  | "list_criteria"
  | "remediate";

export type AnalyzeConfigInput = {
  "profile"?: string;
  "viewport_width"?: number;
  "viewport_height"?: number;
  "selector"?: string | null;
  "pre_scan_actions"?: Array<PreScanActionInput>;
  "cookies"?: Array<CookieInput>;
  "screenshot"?: ScreenshotInput | null;
  "timeout_ms"?: number | null;
  "retry_limit"?: number | null;
  "advanced_rules"?: string | null;
  "igt_tools"?: Array<string> | null;
  "needs_review_policy"?: NeedsReviewPolicyInput | null;
};

export type PreScanActionInput = {
  "selector": string;
  "action": "click";
} | {
  "selector": string;
  "value": string;
  "action": "fill";
} | {
  "selector": string;
  "state"?: WaitForState;
  "action": "wait_for";
};

export type WaitForState = "visible" | "attached" | "hidden" | "detached";

export type CookieInput = {
  "name": string;
  "value": string;
  "domain": string;
  "path"?: string | null;
  "same_site"?: SameSiteInput | null;
  "secure"?: boolean | null;
  "http_only"?: boolean | null;
  "expires"?: number | null;
};

export type SameSiteInput = "strict" | "lax" | "none";

export type ScreenshotInput = {
  "format"?: ScreenshotFormat | null;
  "save_to"?: string | null;
  "save"?: boolean | null;
  "inline"?: boolean | null;
};

export type ScreenshotFormat = "png" | "jpeg";

export type NeedsReviewPolicyInput = "record" | "fail";

/**
 * Analyze a URL for RGAA accessibility findings. Returns detailed per-criterion findings with criterion_id, status (Pass/Fail/NeedsReview/NotTested/NotApplicable/Error), source (axe-core/gap-fix/holo3/manual), evidence, and justification. Note: Both Manuel and PartiellementAutomatable criteria map to NeedsReview status — watch this single status for human-review items.
 */
export type AnalyzeArguments = {
  "url": string;
  "config"?: AnalyzeConfigInput;
  "viewport_width"?: number | null;
  "viewport_height"?: number | null;
};

export type CrawlConfigInput = {
  "max_pages"?: number;
  "max_depth"?: number;
  "respect_robots"?: boolean;
  "sample_mode"?: boolean;
};

/**
 * Run a full RGAA audit on a URL using the orchestrator pipeline. Returns a summary (taux_global, etat_conformite) and sampled_page_urls. IMPORTANT: This returns a summary only — for per-criterion details with evidence, use the `analyze` tool on URLs from sampled_page_urls.
 */
export type AuditUrlArguments = {
  "url": string;
  "config"?: CrawlConfigInput | null;
};

/**
 * Retrieve a previously run audit by its ID.
 */
export type GetAuditResultArguments = {
  "audit_id": string;
};

export type GuidedTestInput = {
  "id": string;
  "version": number;
  "preconditions"?: Array<string>;
  "steps": Array<GuidedStepDto>;
  "criterion_mapping"?: Array<string>;
  "evidence_requirements"?: Array<string>;
};

export type GuidedStepDto = {
  "url": string;
  "kind": "navigate";
} | {
  "kind": "accessibility_tree";
} | {
  "key": string;
  "kind": "press_key";
} | {
  "reference": string;
  "kind": "click_ref";
} | {
  "reference": string;
  "value": string;
  "kind": "fill_ref";
} | {
  "kind": "screenshot";
} | {
  "expected": unknown;
  "kind": "assert_state";
};

/**
 * [DEPRECATED] Use `analyze` with `config.igt_tools: ["keyboard"]` instead. Run a bounded, reproducible intelligent guided accessibility test (keyboard navigation). Reports keyboard-trap after 5 consecutive tabs on the same element. Sets status: incomplete with terminated_reason: ExecutionError on CDP failures.
 */
export type IgtArguments = {
  "test": GuidedTestInput;
};

/**
 * List all 106 RGAA criteria with their IDs, titles, and classifications.
 */
export type ListCriteriaArguments = Record<string, unknown>;

export type RemediationIssueInput = {
  "id": string;
  "rule": string;
  "element_html": string;
  "page_url": string;
  "source_locations"?: Array<SourceLocationInput>;
  "summary": string;
  "remediation": string;
  "criteria"?: Array<string>;
  "framework"?: FrameworkInput | null;
};

export type SourceLocationInput = {
  "file": string;
  "line": number;
  "column"?: number | null;
};

export type FrameworkInput = "react" | "next" | "vue" | "angular";

/**
 * Create approval-gated remediation guidance for accessibility issues.
 */
export type RemediateArguments = {
  "issues": Array<RemediationIssueInput>;
};

/** Maps a tool name to its argument type. */
export interface RgaaToolArguments {
  "analyze": AnalyzeArguments;
  "audit_url": AuditUrlArguments;
  "get_audit_result": GetAuditResultArguments;
  "igt": IgtArguments;
  "list_criteria": ListCriteriaArguments;
  "remediate": RemediateArguments;
}
