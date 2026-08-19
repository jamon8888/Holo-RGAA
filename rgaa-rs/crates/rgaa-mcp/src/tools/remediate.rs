use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RemediationIssueInput {
    pub id: String,
    pub rule: String,
    pub element_html: String,
    pub page_url: String,
    #[serde(default)]
    pub source_locations: Vec<SourceLocationInput>,
    pub summary: String,
    pub remediation: String,
    #[serde(default)]
    pub criteria: Vec<String>,
    #[serde(default)]
    pub framework: Option<FrameworkInput>,
}

impl From<RemediationIssueInput> for rgaa_remediation::RemediationIssue {
    fn from(issue: RemediationIssueInput) -> Self {
        Self {
            id: issue.id,
            rule: issue.rule,
            element_html: issue.element_html,
            page_url: issue.page_url,
            source_locations: issue.source_locations.into_iter().map(Into::into).collect(),
            summary: issue.summary,
            remediation: issue.remediation,
            criteria: issue.criteria,
            framework: issue.framework.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum FrameworkInput {
    React,
    Next,
    Vue,
    Angular,
}

impl From<FrameworkInput> for rgaa_remediation::Framework {
    fn from(framework: FrameworkInput) -> Self {
        match framework {
            FrameworkInput::React => Self::React,
            FrameworkInput::Next => Self::Next,
            FrameworkInput::Vue => Self::Vue,
            FrameworkInput::Angular => Self::Angular,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SourceLocationInput {
    pub file: String,
    pub line: u32,
    #[serde(default)]
    pub column: Option<u32>,
}

impl From<SourceLocationInput> for rgaa_remediation::SourceLocation {
    fn from(location: SourceLocationInput) -> Self {
        Self {
            file: location.file,
            line: location.line,
            column: location.column,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStateDto {
    Required,
    NotRequired,
    Approved { approver: String, token: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct PatchProposalDto {
    pub proposal_id: String,
    pub finding_ids: Vec<String>,
    pub diff: String,
    pub files: Vec<String>,
    pub rationale: String,
    pub risks: Vec<String>,
    pub validation_commands: Vec<String>,
    pub expected_effect: String,
    pub proposal_hash: String,
    pub approval_state: ApprovalStateDto,
    pub approval_token: String,
}

impl From<rgaa_remediation::PatchProposal> for PatchProposalDto {
    fn from(proposal: rgaa_remediation::PatchProposal) -> Self {
        let approval_token = proposal.approval_token();
        let approval_state = match proposal.approval_state() {
            rgaa_remediation::ApprovalState::Required => ApprovalStateDto::Required,
            rgaa_remediation::ApprovalState::NotRequired => ApprovalStateDto::NotRequired,
            rgaa_remediation::ApprovalState::Approved { approver, .. } => {
                ApprovalStateDto::Approved {
                    approver: approver.clone(),
                    token: approval_token.clone(),
                }
            }
        };
        Self {
            proposal_id: proposal.proposal_id,
            finding_ids: proposal.finding_ids,
            diff: proposal.diff,
            files: proposal.files,
            rationale: proposal.rationale,
            risks: proposal.risks,
            validation_commands: proposal.validation_commands,
            expected_effect: proposal.expected_effect,
            proposal_hash: proposal.proposal_hash,
            approval_state,
            approval_token,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "outcome", rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)]
pub enum RemediationOutcomeDto {
    Ok {
        issue_id: String,
        explanation: String,
        steps: Vec<String>,
        confidence: String,
        criteria: Vec<String>,
        proposal: PatchProposalDto,
    },
    Error {
        issue_id: String,
        code: String,
        message: String,
    },
}

impl From<rgaa_remediation::RemediationOutcome> for RemediationOutcomeDto {
    fn from(outcome: rgaa_remediation::RemediationOutcome) -> Self {
        match outcome {
            rgaa_remediation::RemediationOutcome::Ok(guidance) => Self::Ok {
                issue_id: guidance.issue_id,
                explanation: guidance.explanation,
                steps: guidance.steps,
                confidence: guidance.confidence,
                criteria: guidance.criteria,
                proposal: guidance.proposal.into(),
            },
            rgaa_remediation::RemediationOutcome::Error(error) => Self::Error {
                issue_id: error.issue_id,
                code: format!("{:?}", error.code),
                message: error.message,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RemediationResponse {
    pub outcomes: Vec<RemediationOutcomeDto>,
}
