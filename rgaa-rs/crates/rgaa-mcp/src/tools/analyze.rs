use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use rgaa_core::CrawlConfig;
use crate::tools::igt::IgtResultsDto;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AuditUrlInput {
    pub url: String,
    #[serde(default)]
    pub config: Option<CrawlConfigInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CrawlConfigInput {
    #[serde(default = "default_max_pages")]
    pub max_pages: usize,
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,
    #[serde(default = "default_respect_robots")]
    pub respect_robots: bool,
    #[serde(default)]
    pub sample_mode: bool,
}

impl Default for CrawlConfigInput {
    fn default() -> Self {
        Self {
            max_pages: 50,
            max_depth: 5,
            respect_robots: true,
            sample_mode: false,
        }
    }
}

impl From<CrawlConfigInput> for CrawlConfig {
    fn from(input: CrawlConfigInput) -> Self {
        Self {
            max_pages: input.max_pages,
            max_depth: input.max_depth,
            respect_robots: input.respect_robots,
            sample_mode: input.sample_mode,
        }
    }
}

fn default_max_pages() -> usize {
    50
}
fn default_max_depth() -> u32 {
    5
}
fn default_respect_robots() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AuditUrlResult {
    pub audit_id: String,
    pub taux_global: f64,
    pub etat_conformite: String,
}

impl From<rgaa_core::AuditResult> for AuditUrlResult {
    fn from(result: rgaa_core::AuditResult) -> Self {
        Self {
            audit_id: result.audit_id,
            taux_global: result.taux_global,
            etat_conformite: result.etat_conformite,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CriterionDto {
    pub id: String,
    pub title: String,
    pub classification: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListCriteriaResponse {
    pub criteria: Vec<CriterionDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeConfigInput {
    #[serde(default = "default_profile")]
    pub profile: String,
    #[serde(default = "default_width")]
    pub viewport_width: u32,
    #[serde(default = "default_height")]
    pub viewport_height: u32,
    #[serde(default)]
    pub selector: Option<String>,
    #[serde(default)]
    pub pre_scan_actions: Vec<PreScanActionInput>,
    #[serde(default)]
    pub cookies: Vec<CookieInput>,
    #[serde(default)]
    pub screenshot: Option<ScreenshotInput>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub retry_limit: Option<u8>,
    #[serde(default)]
    pub advanced_rules: Option<String>,
    #[serde(default)]
    pub igt_tools: Option<Vec<String>>,
    #[serde(default)]
    pub needs_review_policy: Option<NeedsReviewPolicyInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum NeedsReviewPolicyInput {
    #[default]
    Record,
    Fail,
}

impl From<NeedsReviewPolicyInput> for rgaa_obscura::NeedsReviewPolicy {
    fn from(policy: NeedsReviewPolicyInput) -> Self {
        match policy {
            NeedsReviewPolicyInput::Record => rgaa_obscura::NeedsReviewPolicy::Record,
            NeedsReviewPolicyInput::Fail => rgaa_obscura::NeedsReviewPolicy::Fail,
        }
    }
}

impl Default for AnalyzeConfigInput {
    fn default() -> Self {
        let config = rgaa_obscura::AnalyzeConfig::default();
        Self {
            profile: config.profile,
            viewport_width: config.viewport.width,
            viewport_height: config.viewport.height,
            selector: config.selector,
            pre_scan_actions: Vec::new(),
            cookies: Vec::new(),
            screenshot: None,
            timeout_ms: None,
            retry_limit: None,
            advanced_rules: None,
            igt_tools: None,
            needs_review_policy: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScreenshotFormat {
    #[default]
    Png,
    Jpeg,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ScreenshotInput {
    #[serde(default)]
    pub format: Option<ScreenshotFormat>,
    #[serde(default)]
    pub save_to: Option<String>,
    #[serde(default)]
    pub save: Option<bool>,
    #[serde(default)]
    pub inline: Option<bool>,
}

impl Default for ScreenshotInput {
    fn default() -> Self {
        Self {
            format: None,
            save_to: None,
            save: None,
            inline: None,
        }
    }
}

/// Converts MCP ScreenshotInput to domain ScreenshotConfig, propagating save_to and inline options.
impl From<ScreenshotInput> for rgaa_obscura::ScreenshotConfig {
    fn from(input: ScreenshotInput) -> Self {
        let policy = match input.save {
            Some(false) => rgaa_obscura::ScreenshotPolicy::None,
            Some(true) => rgaa_obscura::ScreenshotPolicy::Always,
            None => rgaa_obscura::ScreenshotPolicy::Always,
        };
        let format = input
            .format
            .map(|f| match f {
                ScreenshotFormat::Png => rgaa_obscura::ScreenshotFormat::Png,
                ScreenshotFormat::Jpeg => rgaa_obscura::ScreenshotFormat::Jpeg,
            })
            .unwrap_or(rgaa_obscura::ScreenshotFormat::Png);
        Self {
            policy,
            format,
            save_to: input.save_to,
            inline: input.inline,
        }
    }
}

fn default_profile() -> String {
    "default".into()
}
fn default_width() -> u32 {
    1000
}
fn default_height() -> u32 {
    1080
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WaitForState {
    Visible,
    Attached,
    Hidden,
    Detached,
}

impl Default for WaitForState {
    fn default() -> Self {
        WaitForState::Visible
    }
}

impl From<rgaa_obscura::WaitForState> for WaitForState {
    fn from(state: rgaa_obscura::WaitForState) -> Self {
        match state {
            rgaa_obscura::WaitForState::Visible => WaitForState::Visible,
            rgaa_obscura::WaitForState::Attached => WaitForState::Attached,
            rgaa_obscura::WaitForState::Hidden => WaitForState::Hidden,
            rgaa_obscura::WaitForState::Detached => WaitForState::Detached,
        }
    }
}

impl From<WaitForState> for rgaa_obscura::WaitForState {
    fn from(state: WaitForState) -> Self {
        match state {
            WaitForState::Visible => rgaa_obscura::WaitForState::Visible,
            WaitForState::Attached => rgaa_obscura::WaitForState::Attached,
            WaitForState::Hidden => rgaa_obscura::WaitForState::Hidden,
            WaitForState::Detached => rgaa_obscura::WaitForState::Detached,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PreScanActionInput {
    Click { selector: String },
    Fill { selector: String, value: String },
    WaitFor {
        selector: String,
        #[serde(default)]
        state: WaitForState,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CookieInput {
    pub name: String,
    #[serde(skip_serializing)]
    pub value: String,
    pub domain: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub same_site: Option<SameSiteInput>,
    #[serde(default)]
    pub r#secure: Option<bool>,
    #[serde(default)]
    pub http_only: Option<bool>,
    #[serde(default)]
    pub expires: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SameSiteInput {
    Strict,
    Lax,
    None,
}

/// Converts MCP CookieInput to domain CookieReference for browser injection.
impl From<CookieInput> for rgaa_obscura::CookieReference {
    fn from(cookie: CookieInput) -> Self {
        Self {
            name: cookie.name,
            value: Some(cookie.value),
            domain: Some(cookie.domain),
            path: cookie.path,
            same_site: cookie.same_site.map(|s| match s {
                SameSiteInput::Strict => rgaa_obscura::CookieSameSite::Strict,
                SameSiteInput::Lax => rgaa_obscura::CookieSameSite::Lax,
                SameSiteInput::None => rgaa_obscura::CookieSameSite::None,
            }),
            r#secure: cookie.r#secure,
            http_only: cookie.http_only,
            expires: cookie.expires,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CriterionStatusDto {
    Pass,
    Fail,
    NotApplicable,
    Error,
    NeedsReview,
    NotTested,
}

impl From<rgaa_core::CriterionStatus> for CriterionStatusDto {
    fn from(status: rgaa_core::CriterionStatus) -> Self {
        match status {
            rgaa_core::CriterionStatus::Pass => Self::Pass,
            rgaa_core::CriterionStatus::Fail => Self::Fail,
            rgaa_core::CriterionStatus::NotApplicable => Self::NotApplicable,
            rgaa_core::CriterionStatus::Error => Self::Error,
            rgaa_core::CriterionStatus::NeedsReview => Self::NeedsReview,
            rgaa_core::CriterionStatus::NotTested => Self::NotTested,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct EvidenceRefDto {
    pub kind: String,
    pub hash: String,
    pub location: Option<String>,
}

impl From<rgaa_core::EvidenceRef> for EvidenceRefDto {
    fn from(evidence: rgaa_core::EvidenceRef) -> Self {
        Self {
            kind: evidence.kind,
            hash: evidence.hash,
            location: evidence.location,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct FindingDto {
    pub id: String,
    pub rule: String,
    pub criterion_id: Option<String>,
    pub url: String,
    pub target: String,
    pub component_path: Option<String>,
    pub evidence: Vec<EvidenceRefDto>,
    pub status: CriterionStatusDto,
    pub severity: Option<String>,
    pub description: Option<String>,
    pub remediation: Option<String>,
    pub html: Option<String>,
    pub details: Option<String>,
    pub source: String,
}

impl From<rgaa_core::Finding> for FindingDto {
    fn from(finding: rgaa_core::Finding) -> Self {
        Self {
            id: finding.id,
            rule: finding.rule,
            criterion_id: finding.criterion_id,
            url: finding.url,
            target: finding.target,
            component_path: finding.component_path,
            evidence: finding.evidence.into_iter().map(Into::into).collect(),
            status: finding.status.into(),
            severity: finding.severity,
            description: finding.description,
            remediation: finding.remediation,
            html: finding.html,
            details: finding.details,
            source: finding.source,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct PageErrorDto {
    pub code: String,
    pub message: String,
}

impl From<rgaa_core::PageError> for PageErrorDto {
    fn from(error: rgaa_core::PageError) -> Self {
        Self {
            code: error.code,
            message: error.message,
        }
    }
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum AnalyzeResponse {
    Nested(NestedAnalyzeResponse),
    Flat(AnalyzeResponseFlat),
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct NestedAnalyzeResponse {
    pub url: String,
    pub data: NestedData,
    pub evidence: Vec<EvidenceRefDto>,
    pub errors: Vec<PageErrorDto>,
    pub completed: bool,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct NestedData {
    pub axe: Vec<FindingDto>,
    pub igt: IgtResultsDto,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq)]
pub struct AnalyzeResponseFlat {
    pub url: String,
    pub findings: Vec<FindingDto>,
    pub evidence: Vec<EvidenceRefDto>,
    pub errors: Vec<PageErrorDto>,
    pub completed: bool,
    pub duration_ms: u64,
}

impl AnalyzeResponse {
    /// Converts domain AnalyzePageResult to MCP response DTO.
    ///
    /// Uses NestedAnalyzeResponse when IGT results are present (to provide both
    /// axe findings and IGT data), otherwise uses flat AnalyzeResponseFlat.
    pub fn from_result(result: rgaa_obscura::AnalyzePageResult) -> Self {
        let flat = AnalyzeResponseFlat {
            url: result.url.clone(),
            findings: result.findings.into_iter().map(Into::into).collect(),
            evidence: result.evidence.into_iter().map(Into::into).collect(),
            errors: result.errors.into_iter().map(Into::into).collect(),
            completed: result.completed,
            duration_ms: result.duration_ms,
        };
        match result.igt {
            Some(igt) => Self::Nested(NestedAnalyzeResponse {
                url: result.url,
                data: NestedData {
                    axe: flat.findings,
                    igt: igt.into(),
                },
                evidence: flat.evidence,
                errors: flat.errors,
                completed: flat.completed,
                duration_ms: flat.duration_ms,
            }),
            None => Self::Flat(flat),
        }
    }
}
