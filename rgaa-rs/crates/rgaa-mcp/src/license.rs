//! Per-tool licence check (#87).
//!
//! Every tool method of [`crate::ToolServer`] starts with the same check, so
//! one decision covers both transports: stdio (the `#[tool_router]` dispatch
//! calls the methods) and HTTP (`rgaa-mcp-http` calls the same methods from
//! its hand-written dispatch). A refusal becomes a JSON-RPC error carrying
//! [`Denial::jsonrpc_error`]'s code and data, never a generic execution error.
//!
//! The HTTP transport also needs the refusal's status and headers, which a
//! JSON-RPC error cannot carry. Instead of checking a second time (and
//! metering twice), it wraps the call in [`capture_license_outcome`] and reads
//! back what the tool's own check decided.

use std::cell::RefCell;
use std::future::Future;

use rgaa_license::{Admission, Denial, Feature};
use rmcp::model::ErrorCode;
use rmcp::ErrorData;

/// Upgrade link put in tier refusals (`upgrade_url` in the error data).
pub const UPGRADE_URL_ENV: &str = "RGAA_UPGRADE_URL";

/// `RGAA_UPGRADE_URL`, ignored when blank.
pub fn upgrade_url_from_env() -> Option<String> {
    std::env::var(UPGRADE_URL_ENV)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// What a tool costs under the licence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolLicense {
    /// No check at all (catalogue reads).
    Exempt,
    /// The feature must be in the tier; nothing is metered.
    Access(Feature),
    /// The feature must be in the tier, and `units` are consumed.
    Metered(Feature, u32),
}

/// The licence class of each tool `ToolServer` registers. `None` for a name
/// it does not know: the caller refuses rather than guessing.
pub fn tool_license(tool: &str) -> Option<ToolLicense> {
    use ToolLicense::*;
    Some(match tool {
        "lint_static" => Metered(Feature::StaticLint, 1),
        "analyze" | "audit_url" | "run_analysis_plugin" => Metered(Feature::FullAudit, 1),
        "igt" => Metered(Feature::Igt, 1),
        "remediate" | "source_map" | "verify_fix" => Metered(Feature::Remediation, 1),
        "get_audit_result" => Access(Feature::FullAudit),
        // MCP protocol discovery is always available. These names are not
        // ToolServer methods, but keeping them classified documents the
        // transport policy and prevents future dispatch code from metering
        // the handshake or tool catalogue.
        "list_criteria"
        | "initialize"
        | "tools/list"
        | "get_test_methodology"
        | "list_analysis_plugins" => Exempt,
        _ => return None,
    })
}

/// What the licence check decided for one tool call.
#[derive(Debug, Clone)]
pub enum LicenseOutcome {
    Exempt,
    Admitted(Admission),
    Denied(Box<Denial>),
}

/// The JSON-RPC error for a refusal: the same `code`, `message` and `data`
/// as [`Denial::jsonrpc_error`].
pub fn denial_error_data(denial: &Denial) -> ErrorData {
    ErrorData::new(
        ErrorCode(denial.jsonrpc_code()),
        denial.error.to_string(),
        Some(denial.body()),
    )
}

tokio::task_local! {
    static OUTCOME: RefCell<Option<LicenseOutcome>>;
}

/// Runs `fut` and returns, with its output, the licence decision taken by the
/// tool call inside it (`None` when no tool check ran, e.g. the arguments
/// failed to parse first).
pub async fn capture_license_outcome<F: Future>(fut: F) -> (F::Output, Option<LicenseOutcome>) {
    OUTCOME
        .scope(RefCell::new(None), async move {
            let output = fut.await;
            let outcome = OUTCOME
                .try_with(|cell| cell.borrow_mut().take())
                .ok()
                .flatten();
            (output, outcome)
        })
        .await
}

/// Records `outcome` for an enclosing [`capture_license_outcome`]; a no-op
/// outside one (stdio).
pub(crate) fn record(outcome: LicenseOutcome) {
    let _ = OUTCOME.try_with(|cell| *cell.borrow_mut() = Some(outcome));
}
