use crate::tools::*;
use rmcp::{tool, tool_handler, tool_router, ErrorData, ServerHandler};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use rgaa_core::CrawlConfig;
use rgaa_orchestrator::Orchestrator;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeRequest {
    pub url: String,
    #[serde(default)]
    pub config: AnalyzeConfigInput,
    #[serde(default)]
    pub viewport_width: Option<u32>,
    #[serde(default)]
    pub viewport_height: Option<u32>,
}

impl AnalyzeRequest {
    /// Validates that the URL is a valid http/https URL and returns the request if valid.
    pub fn malformed(url: &str) -> Result<Self, McpFailure> {
        let request = Self {
            url: url.into(),
            config: Default::default(),
            viewport_width: None,
            viewport_height: None,
        };
        request.to_domain().map(|_| request)
    }

    /// Converts MCP AnalyzeRequest to domain AnalyzeRequest, validating and mapping fields.
    ///
    /// Validates viewport consistency, maps pre_scan_actions, cookies, screenshot config,
    /// advanced_rules, needs_review_policy, and igt_tools to domain types.
    fn to_domain(&self) -> Result<rgaa_obscura::AnalyzeRequest, McpFailure> {
        if self.viewport_height.is_some() && self.viewport_width.is_none() {
            return Err(McpFailure::invalid(
                "viewportHeight requires viewportWidth to be set",
            ));
        }
        let config = &self.config;
        let actions = config
            .pre_scan_actions
            .iter()
            .map(|action| match action {
                PreScanActionInput::Click { selector } => rgaa_obscura::PreScanAction::Click {
                    selector: selector.clone(),
                },
                PreScanActionInput::Fill { selector, value } => rgaa_obscura::PreScanAction::Fill {
                    selector: selector.clone(),
                    value: value.clone(),
                },
                PreScanActionInput::WaitFor { selector, state } => {
                    rgaa_obscura::PreScanAction::WaitFor {
                        selector: selector.clone(),
                        state: match *state {
                            crate::tools::WaitForState::Visible => {
                                rgaa_obscura::WaitForState::Visible
                            }
                            crate::tools::WaitForState::Attached => {
                                rgaa_obscura::WaitForState::Attached
                            }
                            crate::tools::WaitForState::Hidden => {
                                rgaa_obscura::WaitForState::Hidden
                            }
                            crate::tools::WaitForState::Detached => {
                                rgaa_obscura::WaitForState::Detached
                            }
                        },
                    }
                }
            })
            .collect();
        let domain = rgaa_obscura::AnalyzeRequest {
            url: self.url.clone(),
            config: rgaa_obscura::AnalyzeConfig {
                profile: config.profile.clone(),
                viewport: rgaa_obscura::Viewport {
                    width: self.viewport_width.unwrap_or(config.viewport_width),
                    height: self.viewport_height.unwrap_or(config.viewport_height),
                },
                selector: config.selector.clone(),
                pre_scan_actions: actions,
                cookie_references: config
                    .cookies
                    .iter()
                    .map(|cookie| rgaa_obscura::CookieReference::from(cookie.clone()))
                    .collect(),
                screenshot: config
                    .screenshot
                    .clone()
                    .map(Into::into)
                    .unwrap_or_default(),
                advanced_rule_policy: config
                    .advanced_rules
                    .as_ref()
                    .map(|v| match v.as_str() {
                        // FIXME: "thorough" and "standard" should map to Enabled
                        // once advanced rules are supported by the domain runner.
                        // Currently rejected by validate_supported().
                        "thorough" | "standard" => rgaa_obscura::AdvancedRulePolicy::Disabled,
                        _ => rgaa_obscura::AdvancedRulePolicy::Disabled,
                    })
                    .unwrap_or_default(),
                needs_review_policy: config
                    .needs_review_policy
                    .clone()
                    .map(Into::into)
                    .unwrap_or_default(),
                igt_tools: config.igt_tools.clone().unwrap_or_default(),
                timeout_ms: config.timeout_ms.unwrap_or(30_000),
                retry_limit: config.retry_limit.unwrap_or(0),
                concurrency: 1,
            },
        };
        domain
            .validate()
            .map_err(|error| McpFailure::invalid(error.to_string()))?;
        Ok(domain)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RemediationRequest {
    pub issues: Vec<RemediationIssueInput>,
}

impl RemediationRequest {
    pub fn validate_issue_count(count: usize) -> Result<(), McpFailure> {
        (1..=25)
            .contains(&count)
            .then_some(())
            .ok_or_else(|| McpFailure::invalid("issues must contain between 1 and 25 items"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuidedTestRequest {
    pub test: GuidedTestInput,
}

#[derive(Debug, Clone)]
pub struct McpFailure {
    code: ErrorCode,
    message: String,
}

impl McpFailure {
    pub fn invalid(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::InvalidInput,
            message: message.into(),
        }
    }
    pub fn policy(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::PolicyDenied,
            message: message.into(),
        }
    }
    pub fn unsupported(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::UnsupportedConfiguration,
            message: message.into(),
        }
    }
    pub fn execution(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::ExecutionFailed,
            message: message.into(),
        }
    }
    pub fn incomplete(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::IncompleteResult,
            message: message.into(),
        }
    }
    pub fn empty(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::EmptyResult,
            message: message.into(),
        }
    }
    pub fn code(&self) -> &'static str {
        self.code.as_str()
    }
    pub fn into_error_data(self) -> ErrorData {
        let message = format!("{}: {}", self.code.as_str(), redact(&self.message));
        let data = Some(serde_json::json!({ "code": self.code.as_str() }));
        match self.code {
            ErrorCode::InvalidInput
            | ErrorCode::UnsupportedConfiguration
            | ErrorCode::EmptyResult => ErrorData::invalid_params(message, data),
            _ => ErrorData::internal_error(message, data),
        }
    }
}

const SECRET_KEYS: &[&str] = &[
    "password",
    "passwd",
    "pwd",
    "secret",
    "token",
    "cookie",
    "authorization",
    "api_key",
    "apikey",
    "api-key",
    "access_key",
    "access_token",
    "client_secret",
    "session",
];

pub(crate) fn redact(input: &str) -> String {
    let input = redact_url_userinfo(input);
    let mut output = String::with_capacity(input.len());
    let mut cursor = 0;
    while cursor < input.len() {
        match next_secret_value_from(&input, cursor) {
            Some((start, end)) => {
                output.push_str(&input[cursor..start]);
                output.push_str("[REDACTED]");
                cursor = end;
            }
            None => {
                output.push_str(&input[cursor..]);
                break;
            }
        }
    }
    output
}

fn redact_url_userinfo(input: &str) -> String {
    let mut result = input.to_string();
    let Some(scheme_end) = result.find("://") else {
        return result;
    };
    let auth_start = scheme_end + 3;
    let authority_end = result[auth_start..]
        .find(['/', '?', '#'])
        .map(|i| auth_start + i)
        .unwrap_or(result.len());
    if let Some(at) = result[auth_start..authority_end].rfind('@') {
        let userinfo_end = auth_start + at;
        if result[auth_start..userinfo_end].contains(':') {
            result.replace_range(auth_start..userinfo_end, "[REDACTED]");
        }
    }
    result
}

fn next_secret_value_from(input: &str, from_offset: usize) -> Option<(usize, usize)> {
    let lower = input.to_ascii_lowercase();
    let bytes = input.as_bytes();
    let mut best: Option<(usize, usize)> = None;
    for key in SECRET_KEYS {
        let mut search = from_offset;
        while let Some(rel) = lower[search..].find(key) {
            let abs = search + rel;
            let boundary_ok = abs == 0 || !is_ident(bytes[abs - 1]);
            let after = abs + key.len();
            if boundary_ok && after < bytes.len() {
                let mut j = after;
                while j < bytes.len() && matches!(bytes[j], b' ' | b'\t') {
                    j += 1;
                }
                if j < bytes.len() && matches!(bytes[j], b'=' | b':') {
                    let candidate = secret_value_range(input, j + 1);
                    if best.is_none_or(|(start, _)| candidate.0 < start) {
                        best = Some(candidate);
                    }
                }
            }
            search = after.max(abs + 1);
        }
    }
    best
}

fn secret_value_range(input: &str, value_start: usize) -> (usize, usize) {
    let bytes = input.as_bytes();
    let mut value = value_start;
    while value < bytes.len() && matches!(bytes[value], b' ' | b'\t') {
        value += 1;
    }
    for scheme in ["bearer", "basic"] {
        if starts_with_ci(input, value, scheme) {
            let after = value + scheme.len();
            if after >= bytes.len() || !is_ident(bytes[after]) {
                let mut token_start = after;
                while token_start < bytes.len() && matches!(bytes[token_start], b' ' | b'\t') {
                    token_start += 1;
                }
                return (token_start, token_until_delimiter(input, token_start));
            }
        }
    }
    (value_start, value_end(input, value_start))
}

fn token_until_delimiter(input: &str, start: usize) -> usize {
    let bytes = input.as_bytes();
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b',' | b'}' | b']' | b'"' | b'\'' | b'\n' | b'\r' => break,
            _ => i += 1,
        }
    }
    i
}

fn starts_with_ci(input: &str, at: usize, word: &str) -> bool {
    input.len() - at >= word.len() && input[at..at + word.len()].eq_ignore_ascii_case(word)
}

fn value_end(input: &str, start: usize) -> usize {
    let bytes = input.as_bytes();
    let mut start = start;
    while start < bytes.len() && matches!(bytes[start], b' ' | b'\t') {
        start += 1;
    }
    if start >= bytes.len() {
        return start;
    }
    if matches!(bytes[start], b'"' | b'\'') {
        let quote = bytes[start];
        let mut i = start + 1;
        while i < bytes.len() {
            if bytes[i] == quote && bytes[i - 1] != b'\\' {
                return i + 1;
            }
            i += 1;
        }
        return bytes.len();
    }
    token_until_delimiter(input, start)
}

fn is_ident(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'
}

pub trait AnalyzeService: Send + Sync {
    fn analyze(
        &self,
        request: rgaa_obscura::AnalyzeRequest,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<rgaa_obscura::AnalyzePageResult, McpFailure>>
                + Send
                + '_,
        >,
    >;
}

pub trait RemediationService: Send + Sync {
    fn remediate(
        &self,
        issues: Vec<rgaa_remediation::RemediationIssue>,
    ) -> Result<Vec<rgaa_remediation::RemediationOutcome>, McpFailure>;
}

pub trait GuidedService: Send + Sync {
    fn run(
        &self,
        test: rgaa_obscura::GuidedTest,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<rgaa_obscura::GuidedRunResult, McpFailure>>
                + Send
                + '_,
        >,
    >;
}

pub trait AuditOrchestrationService: Send + Sync {
    fn run_audit(
        &self,
        url: &str,
        config: &CrawlConfig,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<rgaa_core::AuditResult, String>> + Send + '_>,
    >;
}

pub trait AuditStorageService: Send + Sync {
    fn get_audit(
        &self,
        audit_id: &str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Option<rgaa_core::AuditResult>, String>>
                + Send
                + '_,
        >,
    >;
}

pub struct LazyObscuraBridge {
    bridge: tokio::sync::Mutex<rgaa_obscura::ObscuraBridge>,
    started: AtomicBool,
}

impl LazyObscuraBridge {
    pub fn new(bridge: rgaa_obscura::ObscuraBridge) -> Self {
        Self {
            bridge: tokio::sync::Mutex::new(bridge),
            started: AtomicBool::new(false),
        }
    }

    async fn ensure_started(&self) -> Result<(), McpFailure> {
        if self.started.load(Ordering::Acquire) {
            return Ok(());
        }
        let mut guard = self.bridge.lock().await;
        if self.started.load(Ordering::Acquire) {
            return Ok(());
        }
        guard.start_server().await.map_err(|error| {
            McpFailure::unsupported(format!("obscura browser service unavailable: {error}"))
        })?;
        self.started.store(true, Ordering::Release);
        Ok(())
    }
}

fn classify_obscura_error(error: rgaa_obscura::ObscuraError) -> McpFailure {
    match &error {
        rgaa_obscura::ObscuraError::Validation(_) => McpFailure::invalid(error.to_string()),
        rgaa_obscura::ObscuraError::UnsupportedConfiguration(_) => {
            McpFailure::unsupported(error.to_string())
        }
        rgaa_obscura::ObscuraError::PolicyDenied(_) => McpFailure::policy(error.to_string()),
        rgaa_obscura::ObscuraError::ProcessStartup(_) => McpFailure::unsupported(error.to_string()),
        _ => McpFailure::execution(error.to_string()),
    }
}

pub struct ObscuraAnalyzeService {
    bridge: Arc<LazyObscuraBridge>,
}

impl ObscuraAnalyzeService {
    pub fn new(bridge: Arc<LazyObscuraBridge>) -> Self {
        Self { bridge }
    }
}

impl AnalyzeService for ObscuraAnalyzeService {
    fn analyze(
        &self,
        request: rgaa_obscura::AnalyzeRequest,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<rgaa_obscura::AnalyzePageResult, McpFailure>>
                + Send
                + '_,
        >,
    > {
        let bridge = Arc::clone(&self.bridge);
        Box::pin(async move {
            bridge.ensure_started().await?;
            let guard = bridge.bridge.lock().await;
            guard
                .analyze(&request)
                .await
                .map_err(classify_obscura_error)
        })
    }
}

#[derive(Default)]
pub struct RemediationServiceImpl {
    policy: rgaa_remediation::RemediationPolicy,
}

impl RemediationService for RemediationServiceImpl {
    fn remediate(
        &self,
        issues: Vec<rgaa_remediation::RemediationIssue>,
    ) -> Result<Vec<rgaa_remediation::RemediationOutcome>, McpFailure> {
        let mut outcomes = Vec::with_capacity(issues.len());
        for issue in issues {
            let framework = match issue.framework {
                Some(framework) => framework,
                None => rgaa_remediation::detect_framework(&issue.element_html)
                    .unwrap_or(rgaa_remediation::Framework::React),
            };
            let batch = rgaa_remediation::remediate(
                &[issue],
                &self.policy,
                rgaa_remediation::adapter_for(framework),
            )
            .map_err(|error| McpFailure::execution(error.to_string()))?;
            outcomes.extend(batch);
        }
        Ok(outcomes)
    }
}

pub struct ObscuraGuidedService {
    bridge: Arc<LazyObscuraBridge>,
}

impl ObscuraGuidedService {
    pub fn new(bridge: Arc<LazyObscuraBridge>) -> Self {
        Self { bridge }
    }
}

impl GuidedService for ObscuraGuidedService {
    fn run(
        &self,
        test: rgaa_obscura::GuidedTest,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<rgaa_obscura::GuidedRunResult, McpFailure>>
                + Send
                + '_,
        >,
    > {
        let bridge = Arc::clone(&self.bridge);
        Box::pin(async move {
            bridge.ensure_started().await?;
            let guard = bridge.bridge.lock().await;
            guard
                .run_guided_test(&test)
                .await
                .map_err(classify_obscura_error)
        })
    }
}

pub struct OrchestrationService {
    orchestrator: Orchestrator,
}

impl OrchestrationService {
    pub fn new() -> Self {
        Self {
            orchestrator: Orchestrator::new(),
        }
    }
}

impl Default for OrchestrationService {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditOrchestrationService for OrchestrationService {
    fn run_audit(
        &self,
        url: &str,
        config: &CrawlConfig,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<rgaa_core::AuditResult, String>> + Send + '_>,
    > {
        let url = url.to_string();
        let config = config.clone();
        Box::pin(async move { self.orchestrator.run(&url, &config).await })
    }
}

pub struct NoOpStorageService;

impl AuditStorageService for NoOpStorageService {
    fn get_audit(
        &self,
        _audit_id: &str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Option<rgaa_core::AuditResult>, String>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async move { Ok(None) })
    }
}

fn outcome_issue_id(outcome: &rgaa_remediation::RemediationOutcome) -> &str {
    match outcome {
        rgaa_remediation::RemediationOutcome::Ok(guidance) => &guidance.issue_id,
        rgaa_remediation::RemediationOutcome::Error(error) => &error.issue_id,
    }
}

fn validate_outcomes(
    input_ids: &[String],
    outcomes: &[rgaa_remediation::RemediationOutcome],
) -> Result<(), McpFailure> {
    if outcomes.len() != input_ids.len() {
        return Err(McpFailure::incomplete(
            "remediation returned a different number of outcomes than inputs",
        ));
    }
    for (index, outcome) in outcomes.iter().enumerate() {
        if outcome_issue_id(outcome) != input_ids[index] {
            return Err(McpFailure::incomplete(
                "remediation outcomes are not correlated with input issues",
            ));
        }
    }
    Ok(())
}

pub struct ToolServer {
    analyze_service: Arc<dyn AnalyzeService>,
    remediation_service: Arc<dyn RemediationService>,
    guided_service: Arc<dyn GuidedService>,
    audit_service: Arc<dyn AuditOrchestrationService>,
    storage_service: Arc<dyn AuditStorageService>,
}

impl ToolServer {
    pub const fn tool_names() -> [&'static str; 5] {
        [
            "analyze",
            "remediate",
            "igt",
            "audit_url",
            "get_audit_result",
        ]
    }

    pub fn new(
        analyze: Arc<dyn AnalyzeService>,
        remediation: Arc<dyn RemediationService>,
        guided: Arc<dyn GuidedService>,
        audit: Arc<dyn AuditOrchestrationService>,
        storage: Arc<dyn AuditStorageService>,
    ) -> Self {
        Self {
            analyze_service: analyze,
            remediation_service: remediation,
            guided_service: guided,
            audit_service: audit,
            storage_service: storage,
        }
    }
}

#[tool_router]
impl ToolServer {
    #[tool(
        name = "analyze",
        description = "Analyze a URL for RGAA accessibility findings."
    )]
    pub async fn analyze(
        &self,
        request: rmcp::handler::server::wrapper::Parameters<AnalyzeRequest>,
    ) -> Result<rmcp::handler::server::wrapper::Json<AnalyzeResponse>, ErrorData> {
        let domain = request.0.to_domain().map_err(McpFailure::into_error_data)?;
        let result = self
            .analyze_service
            .analyze(domain)
            .await
            .map_err(McpFailure::into_error_data)?;
        if !result.completed && result.errors.is_empty() {
            return Err(
                McpFailure::incomplete("analysis returned incomplete empty result")
                    .into_error_data(),
            );
        }
        Ok(rmcp::handler::server::wrapper::Json(
            AnalyzeResponse::from_result(result),
        ))
    }

    #[tool(
        name = "remediate",
        description = "Create approval-gated remediation guidance for accessibility issues."
    )]
    pub fn remediate(
        &self,
        request: rmcp::handler::server::wrapper::Parameters<RemediationRequest>,
    ) -> Result<rmcp::handler::server::wrapper::Json<RemediationResponse>, ErrorData> {
        let inputs = request.0.issues;
        RemediationRequest::validate_issue_count(inputs.len())
            .map_err(McpFailure::into_error_data)?;
        let input_ids: Vec<String> = inputs.iter().map(|issue| issue.id.clone()).collect();
        let issues: Vec<rgaa_remediation::RemediationIssue> =
            inputs.into_iter().map(Into::into).collect();
        let outcomes = self
            .remediation_service
            .remediate(issues)
            .map_err(McpFailure::into_error_data)?;
        validate_outcomes(&input_ids, &outcomes).map_err(McpFailure::into_error_data)?;
        Ok(rmcp::handler::server::wrapper::Json(RemediationResponse {
            outcomes: outcomes.into_iter().map(Into::into).collect(),
        }))
    }

    #[tool(
        name = "igt",
        description = "Run a bounded, reproducible intelligent guided accessibility test."
    )]
    pub async fn igt(
        &self,
        request: rmcp::handler::server::wrapper::Parameters<GuidedTestRequest>,
    ) -> Result<rmcp::handler::server::wrapper::Json<GuidedTestResponse>, ErrorData> {
        let test: rgaa_obscura::GuidedTest = request.0.test.into();
        let result = self
            .guided_service
            .run(test)
            .await
            .map_err(McpFailure::into_error_data)?;
        Ok(rmcp::handler::server::wrapper::Json(
            GuidedTestResponse::from(result),
        ))
    }

    #[tool(
        name = "audit_url",
        description = "Run a full RGAA audit on a URL using the orchestrator pipeline."
    )]
    pub async fn audit_url(
        &self,
        request: rmcp::handler::server::wrapper::Parameters<AuditUrlInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<AuditUrlResult>, ErrorData> {
        let input = request.0;
        let config = input.config.unwrap_or_default().into();
        let result = self
            .audit_service
            .run_audit(&input.url, &config)
            .await
            .map_err(|e| McpFailure::execution(e).into_error_data())?;
        Ok(rmcp::handler::server::wrapper::Json(AuditUrlResult::from(
            result,
        )))
    }

    #[tool(
        name = "get_audit_result",
        description = "Retrieve a previously run audit by its ID."
    )]
    pub async fn get_audit_result(
        &self,
        request: rmcp::handler::server::wrapper::Parameters<GetAuditInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<Option<AuditResultDto>>, ErrorData> {
        let result = self
            .storage_service
            .get_audit(&request.0.audit_id)
            .await
            .map_err(|e| McpFailure::execution(e).into_error_data())?;
        Ok(rmcp::handler::server::wrapper::Json(
            result.map(AuditResultDto::from),
        ))
    }

    #[tool(
        name = "list_criteria",
        description = "List all 106 RGAA criteria with their IDs, titles, and classifications."
    )]
    pub fn list_criteria(
        &self,
    ) -> Result<rmcp::handler::server::wrapper::Json<ListCriteriaResponse>, ErrorData> {
        let criteria = rgaa_core::RgaaCriteria::all()
            .iter()
            .map(|c| CriterionDto {
                id: c.id.to_string(),
                title: c.title.clone(),
                classification: format!("{:?}", c.classification),
            })
            .collect();
        Ok(rmcp::handler::server::wrapper::Json(ListCriteriaResponse {
            criteria,
        }))
    }
}

#[tool_handler]
impl ServerHandler for ToolServer {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redaction_covers_multiple_occurrences() {
        let input = "token=abc123 and cookie=xyz789 and password=secret";
        let output = redact(input);
        assert!(!output.contains("abc123"));
        assert!(!output.contains("xyz789"));
        assert!(!output.contains("secret"));
        assert_eq!(output.matches("[REDACTED]").count(), 3);
    }

    #[test]
    fn redaction_covers_url_userinfo_and_quoted_values() {
        assert!(!redact("https://user:pass@example.test/").contains("pass"));
        assert!(!redact("Authorization: Bearer tok_123").contains("tok_123"));
        assert!(!redact("api_key=\"sk-live-999\"").contains("sk-live-999"));
    }

    #[test]
    fn redaction_does_not_touch_plain_text() {
        let input = "the selector did not match any element";
        assert_eq!(redact(input), input);
    }
}
