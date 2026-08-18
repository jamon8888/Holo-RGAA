use crate::tools::*;
use rmcp::{tool, tool_handler, tool_router, ErrorData, ServerHandler};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeRequest {
    pub url: String,
    #[serde(default)]
    pub config: AnalyzeConfigInput,
}

impl AnalyzeRequest {
    pub fn malformed(url: &str) -> Result<Self, McpFailure> {
        let request = Self {
            url: url.into(),
            config: Default::default(),
        };
        request.to_domain().map(|_| request)
    }

    fn to_domain(&self) -> Result<rgaa_obscura::AnalyzeRequest, McpFailure> {
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
            })
            .collect();
        let domain = rgaa_obscura::AnalyzeRequest {
            url: self.url.clone(),
            config: rgaa_obscura::AnalyzeConfig {
                profile: config.profile.clone(),
                viewport: rgaa_obscura::Viewport {
                    width: config.viewport_width,
                    height: config.viewport_height,
                },
                selector: config.selector.clone(),
                pre_scan_actions: actions,
                cookie_references: config
                    .cookie_references
                    .iter()
                    .map(|cookie| rgaa_obscura::CookieReference {
                        name: cookie.name.clone(),
                        domain: cookie.domain.clone(),
                    })
                    .collect(),
                screenshot_policy: match config.screenshot_policy {
                    ScreenshotPolicyInput::None => rgaa_obscura::ScreenshotPolicy::None,
                    ScreenshotPolicyInput::OnFailure => rgaa_obscura::ScreenshotPolicy::OnFailure,
                    ScreenshotPolicyInput::Always => rgaa_obscura::ScreenshotPolicy::Always,
                },
                advanced_rule_policy: Default::default(),
                needs_review_policy: Default::default(),
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
            message: format!(
                "{}: {}",
                ErrorCode::InvalidInput.as_str(),
                redact(&message.into())
            ),
        }
    }
    pub fn code(&self) -> &'static str {
        self.code.as_str()
    }
}

fn redact(value: &str) -> String {
    let mut result = value.to_owned();
    for marker in ["password", "token", "secret", "cookie"] {
        if let Some(index) = result.to_ascii_lowercase().find(marker) {
            result.replace_range(index.., "[REDACTED]");
        }
    }
    result
}

pub trait AnalyzeService: Send + Sync {
    fn analyze(
        &self,
        request: rgaa_obscura::AnalyzeRequest,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<rgaa_obscura::AnalyzePageResult, String>>
                + Send
                + '_,
        >,
    >;
}

pub trait RemediationService: Send + Sync {
    fn remediate(
        &self,
        issues: Vec<rgaa_remediation::RemediationIssue>,
    ) -> Result<Vec<rgaa_remediation::RemediationOutcome>, String>;
}

pub trait GuidedService: Send + Sync {
    fn run(
        &self,
        test: rgaa_obscura::GuidedTest,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<rgaa_obscura::GuidedRunResult, String>>
                + Send
                + '_,
        >,
    >;
}

pub struct ObscuraAnalyzeService {
    bridge: Arc<rgaa_obscura::ObscuraBridge>,
}
impl ObscuraAnalyzeService {
    pub fn new(bridge: Arc<rgaa_obscura::ObscuraBridge>) -> Self {
        Self { bridge }
    }
}
impl AnalyzeService for ObscuraAnalyzeService {
    fn analyze(
        &self,
        request: rgaa_obscura::AnalyzeRequest,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<rgaa_obscura::AnalyzePageResult, String>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async move {
            self.bridge
                .analyze(&request)
                .await
                .map_err(|error| error.to_string())
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
    ) -> Result<Vec<rgaa_remediation::RemediationOutcome>, String> {
        let mut outcomes = Vec::with_capacity(issues.len());
        for issue in issues {
            let framework = issue.framework.unwrap_or_else(|| {
                rgaa_remediation::detect_framework(&issue.element_html)
                    .unwrap_or(rgaa_remediation::Framework::React)
            });
            outcomes.extend(
                rgaa_remediation::remediate(
                    &[issue],
                    &self.policy,
                    rgaa_remediation::adapter_for(framework),
                )
                .map_err(|error| error.to_string())?,
            );
        }
        Ok(outcomes)
    }
}

pub struct ObscuraGuidedService {
    bridge: Arc<rgaa_obscura::ObscuraBridge>,
}
impl ObscuraGuidedService {
    pub fn new(bridge: Arc<rgaa_obscura::ObscuraBridge>) -> Self {
        Self { bridge }
    }
}
impl GuidedService for ObscuraGuidedService {
    fn run(
        &self,
        test: rgaa_obscura::GuidedTest,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<rgaa_obscura::GuidedRunResult, String>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async move {
            self.bridge
                .run_guided_test(&test)
                .await
                .map_err(|error| error.to_string())
        })
    }
}

pub struct ToolServer {
    pub(crate) analyze_service: Arc<dyn AnalyzeService>,
    pub(crate) remediation_service: Arc<dyn RemediationService>,
    pub(crate) guided_service: Arc<dyn GuidedService>,
}

impl ToolServer {
    pub const fn tool_names() -> [&'static str; 3] {
        ["analyze", "remediate", "igt"]
    }

    pub fn new(
        analyze: Arc<dyn AnalyzeService>,
        remediation: Arc<dyn RemediationService>,
        guided: Arc<dyn GuidedService>,
    ) -> Self {
        Self {
            analyze_service: analyze,
            remediation_service: remediation,
            guided_service: guided,
        }
    }

    fn error(message: impl Into<String>) -> ErrorData {
        ErrorData::invalid_params(
            format!(
                "{}: {}",
                ErrorCode::ExecutionFailed.as_str(),
                redact(&message.into())
            ),
            None,
        )
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
        let domain = request
            .0
            .to_domain()
            .map_err(|error| Self::error(error.message))?;
        let result = self
            .analyze_service
            .analyze(domain)
            .await
            .map_err(Self::error)?;
        if !result.completed && result.errors.is_empty() {
            return Err(Self::error("analysis returned incomplete empty result"));
        }
        Ok(rmcp::handler::server::wrapper::Json(AnalyzeResponse {
            url: result.url,
            findings: result
                .findings
                .into_iter()
                .map(|v| serde_json::to_value(v).unwrap_or_default())
                .collect(),
            evidence: result
                .evidence
                .into_iter()
                .map(|v| serde_json::to_value(v).unwrap_or_default())
                .collect(),
            errors: result
                .errors
                .into_iter()
                .map(|v| serde_json::to_value(v).unwrap_or_default())
                .collect(),
            completed: result.completed,
            duration_ms: result.duration_ms,
        }))
    }

    #[tool(
        name = "remediate",
        description = "Create approval-gated remediation guidance for accessibility issues."
    )]
    pub fn remediate(
        &self,
        request: rmcp::handler::server::wrapper::Parameters<RemediationRequest>,
    ) -> Result<rmcp::handler::server::wrapper::Json<RemediationResponse>, ErrorData> {
        RemediationRequest::validate_issue_count(request.0.issues.len())
            .map_err(|error| Self::error(error.message))?;
        let issues = request
            .0
            .issues
            .into_iter()
            .map(|issue| rgaa_remediation::RemediationIssue {
                id: issue.id,
                rule: issue.rule,
                element_html: issue.element_html,
                page_url: issue.page_url,
                source_locations: issue
                    .source_locations
                    .into_iter()
                    .map(|location| rgaa_remediation::SourceLocation {
                        file: location.file,
                        line: location.line,
                        column: location.column,
                    })
                    .collect(),
                summary: issue.summary,
                remediation: issue.remediation,
                criteria: issue.criteria,
                framework: issue.framework.map(|framework| match framework {
                    FrameworkInput::React => rgaa_remediation::Framework::React,
                    FrameworkInput::Next => rgaa_remediation::Framework::Next,
                    FrameworkInput::Vue => rgaa_remediation::Framework::Vue,
                    FrameworkInput::Angular => rgaa_remediation::Framework::Angular,
                }),
            })
            .collect();
        let outcomes = self
            .remediation_service
            .remediate(issues)
            .map_err(Self::error)?;
        if outcomes.is_empty() {
            return Err(Self::error("remediation returned empty result"));
        }
        Ok(rmcp::handler::server::wrapper::Json(RemediationResponse {
            outcomes: outcomes
                .into_iter()
                .map(|v| serde_json::to_value(v).unwrap_or_default())
                .collect(),
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
        let input = request.0.test;
        let test = rgaa_obscura::GuidedTest {
            id: input.id,
            version: input.version,
            preconditions: input.preconditions,
            steps: input
                .steps
                .into_iter()
                .map(serde_json::from_value)
                .collect::<Result<_, _>>()
                .map_err(|_| Self::error("invalid guided test step"))?,
            criterion_mapping: input.criterion_mapping,
            evidence_requirements: input.evidence_requirements,
        };
        let result = self.guided_service.run(test).await.map_err(Self::error)?;
        Ok(rmcp::handler::server::wrapper::Json(GuidedTestResponse {
            issues: result.issues,
            unanalyzed_elements: result.unanalyzed_elements,
            terminated_reason: serde_json::to_value(result.terminated_reason)
                .map_err(|_| Self::error("invalid guided result"))?
                .as_str()
                .unwrap_or("execution_error")
                .into(),
            completed_steps: result.completed_steps,
            evidence: result
                .evidence
                .into_iter()
                .map(|v| serde_json::to_value(v).unwrap_or_default())
                .collect(),
            manual_review_required: result.manual_review_required,
        }))
    }
}

#[tool_handler]
impl ServerHandler for ToolServer {}
