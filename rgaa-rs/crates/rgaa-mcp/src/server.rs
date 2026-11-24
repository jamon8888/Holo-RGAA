use crate::tools::*;
use futures_util::StreamExt;
use rmcp::{tool, tool_handler, tool_router, ErrorData, ServerHandler};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use rgaa_core::{AuditType, CrawlConfig};
use rgaa_license::LicenseGate;
use rgaa_orchestrator::Orchestrator;

use crate::license::{denial_error_data, tool_license, LicenseOutcome, ToolLicense};

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
                        "thorough" | "standard" => rgaa_obscura::AdvancedRulePolicy::Enabled,
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
                patch_attach_internals: false,
            },
        };
        domain
            .validate()
            .map_err(|error| McpFailure::invalid(error.to_string()))?;
        Ok(domain)
    }
}

/// Input of `run_analysis_plugin`. There is deliberately no status field.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RunAnalysisPluginInput {
    pub url: String,
    pub plugin: String,
    /// Absent or explicit `null` becomes `{}` so `PluginOptions` keeps its defaults.
    #[serde(
        default = "default_plugin_options",
        deserialize_with = "deserialize_plugin_options"
    )]
    pub options: serde_json::Value,
}

fn default_plugin_options() -> serde_json::Value {
    serde_json::json!({})
}

fn deserialize_plugin_options<'de, D>(deserializer: D) -> Result<serde_json::Value, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(value.unwrap_or_else(default_plugin_options))
}

/// Proofs only: a verdict would violate R-P1.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RunAnalysisPluginResponse {
    pub preuves: Vec<rgaa_analysis::Preuve>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListAnalysisPluginsResponse {
    pub plugins: Vec<rgaa_analysis::PluginInfo>,
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

/// Arguments for `get_test_methodology`: one RGAA test key, e.g. `"1.2.3"`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetTestMethodologyRequest {
    pub test: String,
}

/// `get_test_methodology` response: the official method for the test, taken
/// from `RGAA/methodologies.json` (DISIC/accessibilite.numerique.gouv.fr) at
/// the pinned commit. `methodology` is `None` when the key is not an RGAA test.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetTestMethodologyResponse {
    pub test: String,
    pub methodology: Option<String>,
    pub source: String,
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
    /// Map a domain error from the audit pipeline to its MCP failure class.
    /// `RgaaError` has no policy variant, so no case maps to `PolicyDenied`.
    #[allow(dead_code)]
    fn from_rgaa_error(err: rgaa_core::RgaaError) -> Self {
        use rgaa_core::RgaaError;
        match err {
            RgaaError::CriterionNotFound(_)
            | RgaaError::InvalidCriterion(_)
            | RgaaError::MissingId(_)
            | RgaaError::MissingField(_)
            | RgaaError::DuplicateFindingId(_)
            | RgaaError::InvalidStatus(_)
            | RgaaError::NonConformityNotFound(_)
            | RgaaError::Analysis { .. } => Self::invalid(err.to_string()),
            RgaaError::UnsupportedSchemaVersion(_) => Self::unsupported(err.to_string()),
            RgaaError::IncompleteEvidence(_) => Self::incomplete(err.to_string()),
            RgaaError::License(_) => Self::policy(err.to_string()),
            RgaaError::Llm { .. }
            | RgaaError::RateLimited { .. }
            | RgaaError::Timeout { .. }
            | RgaaError::Crawl(_)
            | RgaaError::Browser(_)
            | RgaaError::AxeCore(_)
            | RgaaError::Holo3(_)
            | RgaaError::Media(_)
            | RgaaError::Storage(_)
            | RgaaError::Http(_) => Self::execution(err.to_string()),
        }
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
        audit_type: AuditType,
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

/// Analysis plugins are exposed through this narrow seam so the tool schema
/// can be tested without a browser (P-00 §7).
pub trait AnalysisService: Send + Sync {
    fn list(&self) -> Vec<rgaa_analysis::PluginInfo>;
    fn run(
        &self,
        url: &str,
        plugin: &str,
        options: &serde_json::Value,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Vec<rgaa_analysis::Preuve>, McpFailure>>
                + Send
                + '_,
        >,
    >;
}

/// Default service: the registry built from the plugins linked into this binary.
pub struct RegistryAnalysisService {
    registry: Result<rgaa_analysis::Registry, String>,
    bridge: Arc<LazyObscuraBridge>,
}

impl RegistryAnalysisService {
    /// Builds the registry once, at service construction.
    #[must_use]
    pub fn new() -> Self {
        Self::with_bridge(Arc::new(LazyObscuraBridge::new(
            rgaa_obscura::ObscuraBridge::from_env(),
        )))
    }

    #[must_use]
    pub fn with_bridge(bridge: Arc<LazyObscuraBridge>) -> Self {
        Self {
            registry: rgaa_plugins::default_registry().map_err(|error| error.to_string()),
            bridge,
        }
    }
}

impl Default for RegistryAnalysisService {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalysisService for RegistryAnalysisService {
    fn list(&self) -> Vec<rgaa_analysis::PluginInfo> {
        match &self.registry {
            Ok(registry) => registry.list(),
            Err(error) => {
                tracing::error!(error = %error, "registre des plugins d'analyse indisponible");
                Vec::new()
            }
        }
    }

    fn run(
        &self,
        url: &str,
        plugin: &str,
        options: &serde_json::Value,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Vec<rgaa_analysis::Preuve>, McpFailure>>
                + Send
                + '_,
        >,
    > {
        let url = url.to_owned();
        let plugin_id = plugin.to_owned();
        let options = options.clone();
        let bridge = Arc::clone(&self.bridge);
        Box::pin(async move {
            let registry = self.registry.as_ref().map_err(|error| {
                McpFailure::execution(format!("registre des plugins indisponible : {error}"))
            })?;
            let plugin = registry.get(&plugin_id).ok_or_else(|| {
                McpFailure::invalid(format!("plugin d'analyse inconnu : {plugin_id}"))
            })?;
            let capture = bridge.capture_analysis_page(&url).await?;
            let mut dom_json: serde_json::Value = serde_json::from_str(&capture.dom_snapshot.json)
                .map_err(|error| {
                    McpFailure::execution(format!("capture DOM invalide : {error}"))
                })?;
            if plugin_id == "images" {
                if let Some(images) = dom_json.get("images_for_plugins").cloned() {
                    dom_json["images"] = images;
                }
            }
            let dom_snapshot = rgaa_analysis::DomSnapshot::new(dom_json.to_string());
            let resource_fetcher = SameOriginResourceFetcher::new(&url)?;
            run_analysis_on_capture(
                plugin,
                &url,
                &options,
                &dom_snapshot,
                &capture.ax_tree,
                &capture.screenshot,
                capture.dpr,
                capture.lang_declared.as_deref(),
                &resource_fetcher,
            )
            .await
            .map_err(|error| McpFailure::execution(error.to_string()))
        })
    }
}

const MAX_PAGE_RESOURCE_BYTES: u64 = 20 * 1024 * 1024;

struct SameOriginResourceFetcher {
    client: reqwest::Client,
    page: reqwest::Url,
}

impl SameOriginResourceFetcher {
    fn new(page: &str) -> Result<Self, McpFailure> {
        let page = reqwest::Url::parse(page)
            .map_err(|error| McpFailure::invalid(format!("invalid page URL: {error}")))?;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| {
                McpFailure::execution(format!("resource client unavailable: {error}"))
            })?;
        Ok(Self { client, page })
    }
}

#[async_trait::async_trait]
impl rgaa_analysis::ResourceFetcher for SameOriginResourceFetcher {
    async fn fetch(&self, url: &str) -> rgaa_analysis::Result<Vec<u8>> {
        let resource = self.page.join(url).map_err(|error| {
            rgaa_analysis::analysis_error("resources", format!("invalid resource URL: {error}"))
        })?;
        if resource.scheme() != self.page.scheme()
            || resource.host_str() != self.page.host_str()
            || resource.port_or_known_default() != self.page.port_or_known_default()
            || !resource.username().is_empty()
            || resource.password().is_some()
        {
            return Err(rgaa_analysis::analysis_error(
                "resources",
                "resource is outside the audited page origin",
            ));
        }
        let response = self.client.get(resource).send().await.map_err(|error| {
            rgaa_analysis::analysis_error("resources", format!("resource request failed: {error}"))
        })?;
        if !response.status().is_success() {
            return Err(rgaa_analysis::analysis_error(
                "resources",
                format!("resource returned {}", response.status()),
            ));
        }
        if response
            .content_length()
            .is_some_and(|size| size > MAX_PAGE_RESOURCE_BYTES)
        {
            return Err(rgaa_analysis::analysis_error(
                "resources",
                "resource exceeds 20 MiB",
            ));
        }
        let mut stream = response.bytes_stream();
        let mut bytes = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|error| {
                rgaa_analysis::analysis_error("resources", format!("resource read failed: {error}"))
            })?;
            if bytes.len() as u64 + chunk.len() as u64 > MAX_PAGE_RESOURCE_BYTES {
                return Err(rgaa_analysis::analysis_error(
                    "resources",
                    "resource exceeds 20 MiB",
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_analysis_on_capture(
    plugin: &dyn rgaa_analysis::AnalysisPlugin,
    url: &str,
    options: &serde_json::Value,
    dom_snapshot: &rgaa_analysis::DomSnapshot,
    ax_tree: &rgaa_analysis::AxTree,
    screenshot: &rgaa_analysis::RgbaImage,
    dpr: f32,
    lang_declared: Option<&str>,
    resources: &dyn rgaa_analysis::ResourceFetcher,
) -> rgaa_analysis::Result<Vec<rgaa_analysis::Preuve>> {
    if !plugin.models_optional() {
        for model in plugin.models() {
            rgaa_analysis::models::ensure_present(plugin.id(), model)?;
        }
    }
    let options: rgaa_analysis::PluginOptions =
        serde_json::from_value(options.clone()).map_err(|error| {
            rgaa_analysis::analysis_error(
                plugin.id(),
                format!("options de plugin invalides : {error}"),
            )
        })?;
    let capture = rgaa_analysis::PageCapture {
        url,
        dom_snapshot,
        ax_tree: Some(ax_tree),
        screenshot: Some(screenshot),
        dpr,
        resources,
        lang_declared,
    };
    plugin.analyze(&capture, &options).await
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

    async fn capture_analysis_page(
        &self,
        url: &str,
    ) -> Result<rgaa_obscura::AnalysisPageCapture, McpFailure> {
        self.ensure_started().await?;
        self.bridge
            .lock()
            .await
            .capture_analysis_page(url)
            .await
            .map_err(classify_obscura_error)
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
        audit_type: AuditType,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<rgaa_core::AuditResult, String>> + Send + '_>,
    > {
        let url = url.to_string();
        let mut config = config.clone();
        config.audit_type = audit_type;
        Box::pin(async move { self.orchestrator.run_crawl_and_audit(&url, &config).await })
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
<<    analysis_service: Arc<dyn AnalysisService>,
    license: Arc<LicenseGate>,
}

impl ToolServer {
    pub const fn tool_names() -> [&'static str; 12] {
        [
            "analyze",
            "remediate",
            "igt",
            "audit_url",
            "get_audit_result",
            "list_criteria",
            "lint_static",
            "source_map",
            "verify_fix",
            "get_test_methodology",
            "list_analysis_plugins",
            "run_analysis_plugin",
        ]
    }

    pub fn new(
        analyze: Arc<dyn AnalyzeService>,
        remediation: Arc<dyn RemediationService>,
        guided: Arc<dyn GuidedService>,
        audit: Arc<dyn AuditOrchestrationService>,
        storage: Arc<dyn AuditStorageService>,
    ) -> Self {
        Self::new_with_analysis(
            analyze,
            remediation,
            guided,
            audit,
            storage,
            Arc::new(RegistryAnalysisService::new()),
        )
    }

    pub fn new_with_analysis(
        analyze: Arc<dyn AnalyzeService>,
        remediation: Arc<dyn RemediationService>,
        guided: Arc<dyn GuidedService>,
        audit: Arc<dyn AuditOrchestrationService>,
        storage: Arc<dyn AuditStorageService>,
        analysis: Arc<dyn AnalysisService>,
    ) -> Self {
        Self {
            analyze_service: analyze,
            remediation_service: remediation,
            guided_service: guided,
            audit_service: audit,
            storage_service: storage,
<<            analysis_service: analysis,
            // Local default: tier gating from the installed licence, no
            // quota (a counter in a user-writable file is not enforceable on
            // the user's own machine). The hosted HTTP server replaces it
            // with a metering gate through `with_license`.
            license: Arc::new(LicenseGate::from_env(
                false,
                crate::license::upgrade_url_from_env(),
            )),
        }
    }

    /// Replaces the licence gate every tool call is checked against.
    pub fn with_license(mut self, gate: Arc<LicenseGate>) -> Self {
        self.license = gate;
        self
    }

    pub fn license(&self) -> &Arc<LicenseGate> {
        &self.license
    }

    /// The licence decision for one call of `tool`. Consumes quota when the
    /// tool is metered and the call is admitted; a refusal consumes nothing.
    ///
    /// The tool methods call this themselves, so call it directly only to
    /// ask, never before calling a tool (that would meter twice).
    pub fn check_tool(&self, tool: &str) -> Option<LicenseOutcome> {
        let (feature, cost) = match tool_license(tool)? {
            ToolLicense::Exempt => return Some(LicenseOutcome::Exempt),
            ToolLicense::Access(feature) => (feature, 0),
            ToolLicense::Metered(feature, units) => (feature, units),
        };
        Some(match self.license.check(feature, cost) {
            Ok(admission) => LicenseOutcome::Admitted(admission),
            Err(denial) => LicenseOutcome::Denied(denial),
        })
    }

    /// The check at the top of every tool method.
    fn admit(&self, tool: &'static str) -> Result<(), ErrorData> {
        let Some(outcome) = self.check_tool(tool) else {
            // A tool without a licence class is a wiring bug: refuse rather
            // than run it unmetered.
            return Err(
                McpFailure::policy(format!("no licence class for tool `{tool}`")).into_error_data(),
            );
        };
        let refused = match &outcome {
            LicenseOutcome::Denied(denial) => {
                tracing::info!(
                    tool,
                    code = denial.error.code(),
                    tier = %denial.tier,
                    "licence refused tool call"
                );
                Some(denial_error_data(denial))
            }
            LicenseOutcome::Exempt | LicenseOutcome::Admitted(_) => None,
        };
        crate::license::record(outcome);
        refused.map_or(Ok(()), Err)
    }
}

/// Why a corrected file's page did not make it into the diff.
enum FileFailure {
    TimedOut,
    Failed(String),
}

impl ToolServer {
    /// Re-analyse one page through the injected [`AnalyzeService`] under a
    /// hard time budget.
    ///
    /// The timeout is applied here, around the service call, rather than
    /// being passed to the analyzer as a config value: an analyzer that
    /// hangs before it reads its own config would ignore the latter, which
    /// is exactly the case the budget exists for.
    async fn verify_one_page(
        &self,
        url: &str,
        budget: std::time::Duration,
    ) -> Result<Vec<rgaa_core::Finding>, FileFailure> {
        let request = AnalyzeRequest {
            url: url.to_string(),
            config: Default::default(),
            viewport_width: None,
            viewport_height: None,
        };
        let domain = match request.to_domain() {
            Ok(domain) => domain,
            Err(failure) => return Err(FileFailure::Failed(failure.message)),
        };
        match tokio::time::timeout(budget, self.analyze_service.analyze(domain)).await {
            Err(_) => Err(FileFailure::TimedOut),
            Ok(Err(failure)) => Err(FileFailure::Failed(failure.message)),
            Ok(Ok(result)) => Ok(result.findings),
        }
    }
}

#[tool_router(vis = "pub")]
impl ToolServer {
    #[tool(
        name = "analyze",
        description = "Analyze a URL for RGAA accessibility findings. Returns detailed per-criterion findings with criterion_id, status (Pass/Fail/NeedsReview/NotTested/NotApplicable/Error), source (axe-core/gap-fix/agent/manual), evidence, and justification. Note: Both Manuel and PartiellementAutomatable criteria map to NeedsReview status — watch this single status for human-review items."
    )]
    pub async fn analyze(
        &self,
        request: rmcp::handler::server::wrapper::Parameters<AnalyzeRequest>,
    ) -> Result<rmcp::handler::server::wrapper::Json<AnalyzeResponse>, ErrorData> {
        self.admit("analyze")?;
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
        self.admit("remediate")?;
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
        description = "[DEPRECATED] Use `analyze` with `config.igt_tools: [\"keyboard\"]` instead. Run a bounded, reproducible intelligent guided accessibility test (keyboard navigation). Reports keyboard-trap after 5 consecutive tabs on the same element. Sets status: incomplete with terminated_reason: ExecutionError on CDP failures."
    )]
    pub async fn igt(
        &self,
        request: rmcp::handler::server::wrapper::Parameters<GuidedTestRequest>,
    ) -> Result<rmcp::handler::server::wrapper::Json<GuidedTestResponse>, ErrorData> {
        self.admit("igt")?;
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
        description = "Run an RGAA audit on a URL using the orchestrator pipeline. `audit_type` selects the scope: rapide (25 criteria), complementaire (50) or complet (106, the default). Only in-scope criteria are evaluated — a rapide audit makes no LLM call for the other 81. Returns a summary (audit_type, taux_global, etat_conformite) and sampled_page_urls. A rapide audit never yields a conformity declaration. IMPORTANT: This returns a summary only — for per-criterion details with evidence, use the `analyze` tool on URLs from sampled_page_urls."
    )]
    pub async fn audit_url(
        &self,
        request: rmcp::handler::server::wrapper::Parameters<AuditUrlInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<AuditUrlResult>, ErrorData> {
        self.admit("audit_url")?;
        let input = request.0;
        let config = input.config.unwrap_or_default().into();
        let result = self
            .audit_service
            .run_audit(&input.url, &config, input.audit_type)
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
        self.admit("get_audit_result")?;
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
        name = "source_map",
        description = "Relocate browser findings to the template that produced them. Takes a source_root directory and findings carrying a CSS selector and/or the element's outerHTML, and returns a source_location (file, line, column, snippet) per mapped finding. Matching is a best-effort literal search — the browser reports the rendered DOM while the repository holds templates, so there is no exact inverse. Supports React JSX, Vue SFC, Angular and vanilla HTML. Findings that stay ambiguous are returned in `unmappable` with a reason instead of a guessed location; always check `confidence` and `matched_on` before editing."
    )]
    pub async fn source_map(
        &self,
        request: rmcp::handler::server::wrapper::Parameters<SourceMapRequest>,
    ) -> Result<rmcp::handler::server::wrapper::Json<SourceMapResponse>, ErrorData> {
        self.admit("source_map")?;
        let request = request.0;
        SourceMapRequest::validate_finding_count(request.findings.len())
            .map_err(McpFailure::into_error_data)?;

        // The scan walks directories and reads files synchronously. Run
        // directly, it occupies a Tokio worker for its whole duration without
        // ever yielding, so a scan on a slow filesystem stalls unrelated
        // requests and the SSE progress stream on the same worker.
        let response = tokio::task::spawn_blocking(move || {
            crate::tools::source_map::map_findings(&request.source_root, &request.findings)
        })
        .await
        .map_err(|error| {
            McpFailure::execution(format!("source_map scan task failed: {error}")).into_error_data()
        })?
        .map_err(McpFailure::into_error_data)?;
        Ok(rmcp::handler::server::wrapper::Json(response))
    }

    #[tool(
        name = "verify_fix",
        description = "Re-verify corrected files against a reference audit bundle. Re-runs the same analysis used by `analyze` on each corrected file's page and diffs the result against the reference with the shared baseline comparison, returning findings as fixed / remaining / new, plus `unverified` for pages that could not be re-analysed. Citations from the reference audit's criterion verdicts travel with each finding. Each page gets at most 30s."
    )]
    pub async fn verify_fix(
        &self,
        request: rmcp::handler::server::wrapper::Parameters<VerifyFixRequest>,
    ) -> Result<rmcp::handler::server::wrapper::Json<VerifyFixResponse>, ErrorData> {
        self.admit("verify_fix")?;
        let input = request.0;
        if input.files.is_empty() || input.files.len() > MAX_FILES {
            return Err(McpFailure::invalid(format!(
                "files must contain between 1 and {MAX_FILES} items"
            ))
            .into_error_data());
        }
        let reference: rgaa_core::AuditBundle = serde_json::from_value(input.reference_audit)
            .map_err(|error| {
                McpFailure::invalid(format!("reference_audit is not an AuditBundle: {error}"))
                    .into_error_data()
            })?;

        // Clamp rather than reject: the ceiling is a safety property of this
        // server, not a caller preference, and a caller asking for longer
        // should still get an answer.
        let budget = std::time::Duration::from_millis(
            input
                .per_file_timeout_ms
                .unwrap_or(MAX_PER_FILE_TIMEOUT_MS)
                .clamp(1, MAX_PER_FILE_TIMEOUT_MS),
        );

        // Several corrected files routinely live behind one page. Analysing
        // per distinct URL keeps one browser run per page and, more
        // importantly, stops the same page's findings entering the diff
        // twice — duplicates would inflate every category.
        let mut outcomes: Vec<(String, Result<Vec<rgaa_core::Finding>, FileFailure>)> = Vec::new();
        for file in &input.files {
            if outcomes.iter().any(|(url, _)| url == &file.url) {
                continue;
            }
            let outcome = self.verify_one_page(&file.url, budget).await;
            outcomes.push((file.url.clone(), outcome));
        }

        let mut scope = VerificationScope {
            verified_urls: std::collections::HashSet::new(),
            unverified_urls: std::collections::HashSet::new(),
        };
        let mut current_findings = Vec::new();
        for (url, outcome) in &outcomes {
            match outcome {
                Ok(findings) => {
                    scope.verified_urls.insert(url.clone());
                    current_findings.extend(findings.iter().cloned());
                }
                Err(_) => {
                    scope.unverified_urls.insert(url.clone());
                }
            }
        }

        let files = input
            .files
            .iter()
            .map(|file| {
                let outcome = outcomes
                    .iter()
                    .find(|(url, _)| url == &file.url)
                    .map(|(_, outcome)| outcome);
                let (status, message) = match outcome {
                    Some(Ok(_)) | None => (FileVerificationStatus::Verified, None),
                    Some(Err(FileFailure::TimedOut)) => (
                        FileVerificationStatus::TimedOut,
                        Some(format!("re-analysis exceeded {} ms", budget.as_millis())),
                    ),
                    Some(Err(FileFailure::Failed(message))) => {
                        (FileVerificationStatus::Failed, Some(redact(message)))
                    }
                };
                FileVerificationDto {
                    path: file.path.clone(),
                    url: file.url.clone(),
                    status,
                    message,
                }
            })
            .collect();

        Ok(rmcp::handler::server::wrapper::Json(categorize(
            &reference,
            &scope,
            current_findings,
            files,
        )))
    }

    #[tool(
        name = "list_criteria",
        description = "List all 106 RGAA criteria with their IDs, titles, and classifications."
    )]
    pub fn list_criteria(
        &self,
    ) -> Result<rmcp::handler::server::wrapper::Json<ListCriteriaResponse>, ErrorData> {
        self.admit("list_criteria")?;
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

    #[tool(
        name = "get_test_methodology",
        description = "Return the official RGAA test method for one test key (e.g. \"1.2.3\"): the numbered verification steps from RGAA/methodologies.json (DISIC/accessibilite.numerique.gouv.fr, pinned commit). Skills cite this instead of describing a method from memory. Returns `methodology: null` when the key is not an RGAA 4.1.2 test."
    )]
    pub fn get_test_methodology(
        &self,
        request: rmcp::handler::server::wrapper::Parameters<GetTestMethodologyRequest>,
    ) -> Result<rmcp::handler::server::wrapper::Json<GetTestMethodologyResponse>, ErrorData> {
        let test = request.0.test;
        let methodology = rgaa_core::RgaaMethodologies::get(&test);
        Ok(rmcp::handler::server::wrapper::Json(
            GetTestMethodologyResponse {
                test,
                methodology: methodology.map(|steps| steps.to_string()),
                source: format!(
                    "RGAA/methodologies.json @ DISIC/accessibilite.numerique.gouv.fr commit {}",
                    rgaa_core::METHODOLOGIES_COMMIT
                ),
            },
        ))
    }

    #[tool(
        name = "list_analysis_plugins",
        description = "List compiled analysis plugins with their versions, RGAA tests, required models and presence. Sidecars show their exact command and executable digest. Plugins produce proofs, never statuses."
    )]
    pub fn list_analysis_plugins(
        &self,
    ) -> Result<rmcp::handler::server::wrapper::Json<ListAnalysisPluginsResponse>, ErrorData> {
        Ok(rmcp::handler::server::wrapper::Json(
            ListAnalysisPluginsResponse {
                plugins: self.analysis_service.list(),
            },
        ))
    }

    #[tool(
        name = "run_analysis_plugin",
        description = "Run one analysis plugin on a URL and return its Preuve values (measurements, artefacts, hashes). No status or verdict is returned: the orchestrator and a human reviewer decide (R-P1)."
    )]
    pub async fn run_analysis_plugin(
        &self,
        request: rmcp::handler::server::wrapper::Parameters<RunAnalysisPluginInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<RunAnalysisPluginResponse>, ErrorData> {
        let input = request.0;
        let preuves = self
            .analysis_service
            .run(&input.url, &input.plugin, &input.options)
            .await
            .map_err(McpFailure::into_error_data)?;
        Ok(rmcp::handler::server::wrapper::Json(
            RunAnalysisPluginResponse { preuves },
        ))
    }

    #[tool(
        name = "lint_static",
        description = "Lint HTML, JSX/TSX or Vue source for accessibility defects without a browser: missing img alt, unlabelled form controls, nameless buttons and links. Returns line/column, rule, severity and a fix hint per finding, against a selectable profile (rgaa-4.1, wcag-2.1-aa, section-508). Sub-millisecond per file, so it suits an edit loop — it is a first pass over source, NOT a conformance verdict: use `analyze` on a rendered page for that. Pass `source_files` to have the files read from disk and findings mapped onto their real paths."
    )]
    pub fn lint_static(
        &self,
        request: rmcp::handler::server::wrapper::Parameters<LintStaticRequest>,
    ) -> Result<rmcp::handler::server::wrapper::Json<LintStaticResponse>, ErrorData> {
        self.admit("lint_static")?;
        let report = request.0.run().map_err(McpFailure::into_error_data)?;
        Ok(rmcp::handler::server::wrapper::Json(report))
    }
}

#[tool_handler]
impl ServerHandler for ToolServer {}

#[cfg(test)]
mod tests {
    use super::*;

    struct CaptureProbe;

    #[async_trait::async_trait]
    impl rgaa_analysis::AnalysisPlugin for CaptureProbe {
        fn id(&self) -> &'static str {
            "capture-probe"
        }
        fn version(&self) -> &'static str {
            "test"
        }
        fn tests(&self) -> &'static [&'static str] {
            &["1.1.1"]
        }

        async fn analyze(
            &self,
            page: &rgaa_analysis::PageCapture<'_>,
            _options: &rgaa_analysis::PluginOptions,
        ) -> rgaa_analysis::Result<Vec<rgaa_analysis::Preuve>> {
            Ok(vec![rgaa_analysis::Preuve {
                plugin: self.id().to_owned(),
                version_plugin: self.version().to_owned(),
                version_modele: None,
                tests: vec!["1.1.1".into()],
                sujet: rgaa_analysis::Subject::Page,
                portee: rgaa_analysis::Portee::Mesuree,
                mesures: serde_json::json!({
                    "dom": page.dom_snapshot.json,
                    "ax_nodes": page.ax_tree.map(|tree| tree.nodes.len()),
                    "screenshot": page.screenshot.map(|image| [image.width, image.height]),
                    "dpr": page.dpr,
                    "lang": page.lang_declared,
                }),
                indice: None,
                artefacts: Vec::new(),
                empreinte_entree: "captured-page".into(),
            }])
        }
    }

    #[tokio::test]
    async fn plugin_receives_the_populated_browser_capture() {
        let dom = rgaa_analysis::DomSnapshot::new(
            r#"{"page":{"contenu_principal":"texte de la page"},"texts":[{"target":"p#real"}]}"#,
        );
        let ax = rgaa_analysis::AxTree {
            nodes: vec![rgaa_analysis::AxNode {
                role: "paragraph".into(),
                name: "texte de la page".into(),
            }],
        };
        let screenshot = rgaa_analysis::RgbaImage {
            width: 2,
            height: 1,
            pixels: vec![1, 2, 3, 255, 4, 5, 6, 255],
        };
        let proofs = run_analysis_on_capture(
            &CaptureProbe,
            "https://example.test/page",
            &serde_json::json!({}),
            &dom,
            &ax,
            &screenshot,
            2.0,
            Some("fr"),
            &rgaa_analysis::NoopResourceFetcher,
        )
        .await
        .expect("plugin runs on the supplied capture");

        assert_eq!(proofs[0].mesures["dom"], dom.json);
        assert_eq!(proofs[0].mesures["ax_nodes"], 1);
        assert_eq!(proofs[0].mesures["screenshot"], serde_json::json!([2, 1]));
        assert_eq!(proofs[0].mesures["dpr"], 2.0);
        assert_eq!(proofs[0].mesures["lang"], "fr");
    }

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
