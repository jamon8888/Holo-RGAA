use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use rgaa_core::AuditResult;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetAuditInput {
    pub audit_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AuditResultDto {
    pub audit_id: String,
    pub url: String,
    pub taux_global: f64,
    pub etat_conformite: String,
    pub passed: usize,
    pub failed: usize,
    pub na: usize,
    pub duration_ms: u64,
}

impl From<AuditResult> for AuditResultDto {
    fn from(result: AuditResult) -> Self {
        Self {
            audit_id: result.audit_id,
            url: result.url,
            taux_global: result.taux_global,
            etat_conformite: result.etat_conformite,
            passed: result.passed,
            failed: result.failed,
            na: result.na,
            duration_ms: result.duration_ms,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GuidedStepDto {
    Navigate { url: String },
    AccessibilityTree,
    PressKey { key: String },
    ClickRef { reference: String },
    FillRef { reference: String, value: String },
    Screenshot,
    AssertState { expected: serde_json::Value },
}

impl From<GuidedStepDto> for rgaa_obscura::GuidedStep {
    fn from(step: GuidedStepDto) -> Self {
        match step {
            GuidedStepDto::Navigate { url } => Self::Navigate { url },
            GuidedStepDto::AccessibilityTree => Self::AccessibilityTree,
            GuidedStepDto::PressKey { key } => Self::PressKey { key },
            GuidedStepDto::ClickRef { reference } => Self::ClickRef { reference },
            GuidedStepDto::FillRef { reference, value } => Self::FillRef { reference, value },
            GuidedStepDto::Screenshot => Self::Screenshot,
            GuidedStepDto::AssertState { expected } => Self::AssertState { expected },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GuidedTestInput {
    pub id: String,
    pub version: u32,
    #[serde(default)]
    pub preconditions: Vec<String>,
    pub steps: Vec<GuidedStepDto>,
    #[serde(default)]
    pub criterion_mapping: Vec<String>,
    #[serde(default)]
    pub evidence_requirements: Vec<String>,
}

impl From<GuidedTestInput> for rgaa_obscura::GuidedTest {
    fn from(test: GuidedTestInput) -> Self {
        Self {
            id: test.id,
            version: test.version,
            preconditions: test.preconditions,
            steps: test.steps.into_iter().map(Into::into).collect(),
            criterion_mapping: test.criterion_mapping,
            evidence_requirements: test.evidence_requirements,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TerminationReasonDto {
    Completed,
    MissingReference,
    AssertionFailed,
    KeyboardTrap,
    Timeout,
    NavigationError,
    ExecutionError,
    InvalidOrdering,
}

impl From<rgaa_obscura::TerminationReason> for TerminationReasonDto {
    fn from(reason: rgaa_obscura::TerminationReason) -> Self {
        match reason {
            rgaa_obscura::TerminationReason::Completed => Self::Completed,
            rgaa_obscura::TerminationReason::MissingReference => Self::MissingReference,
            rgaa_obscura::TerminationReason::AssertionFailed => Self::AssertionFailed,
            rgaa_obscura::TerminationReason::KeyboardTrap => Self::KeyboardTrap,
            rgaa_obscura::TerminationReason::Timeout => Self::Timeout,
            rgaa_obscura::TerminationReason::NavigationError => Self::NavigationError,
            rgaa_obscura::TerminationReason::ExecutionError => Self::ExecutionError,
            rgaa_obscura::TerminationReason::InvalidOrdering => Self::InvalidOrdering,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct GuidedEvidenceDto {
    pub kind: String,
    pub path: String,
    pub sha256: String,
}

impl From<rgaa_obscura::EvidenceRef> for GuidedEvidenceDto {
    fn from(evidence: rgaa_obscura::EvidenceRef) -> Self {
        Self {
            kind: evidence.kind,
            path: evidence.path,
            sha256: evidence.sha256,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct GuidedTestResponse {
    pub issues: Vec<String>,
    pub unanalyzed_elements: Vec<String>,
    pub terminated_reason: TerminationReasonDto,
    pub completed_steps: usize,
    pub evidence: Vec<GuidedEvidenceDto>,
    pub manual_review_required: bool,
}

impl From<rgaa_obscura::GuidedRunResult> for GuidedTestResponse {
    fn from(result: rgaa_obscura::GuidedRunResult) -> Self {
        Self {
            issues: result.issues,
            unanalyzed_elements: result.unanalyzed_elements,
            terminated_reason: result.terminated_reason.into(),
            completed_steps: result.completed_steps,
            evidence: result.evidence.into_iter().map(Into::into).collect(),
            manual_review_required: result.manual_review_required,
        }
    }
}
