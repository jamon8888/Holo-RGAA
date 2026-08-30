use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use rgaa_core::CrawlConfig;

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
    pub cookie_references: Vec<CookieReferenceInput>,
    #[serde(default)]
    pub screenshot_policy: ScreenshotPolicyInput,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub retry_limit: Option<u8>,
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
            cookie_references: Vec::new(),
            screenshot_policy: ScreenshotPolicyInput::None,
            timeout_ms: None,
            retry_limit: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScreenshotPolicyInput {
    #[default]
    None,
    OnFailure,
    Always,
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
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PreScanActionInput {
    Click { selector: String },
    Fill { selector: String, value: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CookieReferenceInput {
    pub name: String,
    #[serde(default)]
    pub domain: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct AnalyzeResponse {
    pub url: String,
    pub findings: Vec<FindingDto>,
    pub evidence: Vec<EvidenceRefDto>,
    pub errors: Vec<PageErrorDto>,
    pub completed: bool,
    pub duration_ms: u64,
}

impl From<rgaa_obscura::AnalyzePageResult> for AnalyzeResponse {
    fn from(result: rgaa_obscura::AnalyzePageResult) -> Self {
        Self {
            url: result.url,
            findings: result.findings.into_iter().map(Into::into).collect(),
            evidence: result.evidence.into_iter().map(Into::into).collect(),
            errors: result.errors.into_iter().map(Into::into).collect(),
            completed: result.completed,
            duration_ms: result.duration_ms,
        }
    }
}
