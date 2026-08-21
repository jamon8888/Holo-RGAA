use crate::{Framework, FrameworkAdapter, RemediationError, RemediationPolicy};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub struct SourceLocation {
    pub file: String,
    pub line: u32,
    pub column: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemediationIssue {
    pub id: String,
    pub rule: String,
    pub element_html: String,
    pub page_url: String,
    pub source_locations: Vec<SourceLocation>,
    pub summary: String,
    pub remediation: String,
    pub criteria: Vec<String>,
    #[serde(default)]
    pub framework: Option<Framework>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemediationGuidance {
    pub issue_id: String,
    pub explanation: String,
    pub steps: Vec<String>,
    pub confidence: String,
    pub criteria: Vec<String>,
    pub proposal: PatchProposal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemediationErrorInfo {
    pub issue_id: String,
    pub code: RemediationErrorCode,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RemediationErrorCode {
    InvalidIssue,
    PolicyDenied,
    NeedsReview,
    UnsupportedFramework,
    MissingSourceLocation,
    MissingApproval,
    ModelFailure,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum RemediationOutcome {
    Ok(RemediationGuidance),
    Error(RemediationErrorInfo),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PatchProposal {
    pub proposal_id: String,
    pub finding_ids: Vec<String>,
    pub diff: String,
    pub files: Vec<String>,
    pub rationale: String,
    pub risks: Vec<String>,
    pub validation_commands: Vec<String>,
    pub expected_effect: String,
    pub proposal_hash: String,
    #[serde(skip)]
    approval: ApprovalState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ApprovalState {
    #[default]
    Required,
    NotRequired,
    Approved {
        proposal_id: String,
        proposal_hash: String,
        approver: String,
        token: String,
    },
}

impl PatchProposal {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        proposal_id: impl Into<String>,
        finding_ids: Vec<String>,
        diff: impl Into<String>,
        files: Vec<String>,
        rationale: impl Into<String>,
        risks: Vec<String>,
        validation_commands: Vec<String>,
        expected_effect: impl Into<String>,
    ) -> Self {
        let mut proposal = Self {
            proposal_id: proposal_id.into(),
            finding_ids,
            diff: diff.into(),
            files,
            rationale: rationale.into(),
            risks,
            validation_commands,
            expected_effect: expected_effect.into(),
            proposal_hash: String::new(),
            approval: ApprovalState::Required,
        };
        proposal.proposal_hash = proposal.compute_hash();
        proposal
    }

    pub(crate) fn set_approval_required(&mut self, required: bool) {
        self.approval = if required {
            ApprovalState::Required
        } else {
            ApprovalState::NotRequired
        };
    }

    pub fn requires_approval(&self) -> bool {
        matches!(self.approval, ApprovalState::Required)
    }

    pub fn approval_state(&self) -> &ApprovalState {
        &self.approval
    }

    pub fn approve(&mut self, approver: &str, token: &str) -> Result<(), RemediationError> {
        if approver.trim().is_empty() || token != self.approval_token() {
            return Err(RemediationError::InvalidApproval {
                issue_id: self.finding_ids.first().cloned().unwrap_or_default(),
            });
        }
        self.approval = ApprovalState::Approved {
            proposal_id: self.proposal_id.clone(),
            proposal_hash: self.proposal_hash.clone(),
            approver: approver.to_owned(),
            token: token.to_owned(),
        };
        Ok(())
    }

    pub fn approval_token(&self) -> String {
        format!(
            "rgaa-approval-v1-{}-{}",
            self.proposal_id, self.proposal_hash
        )
    }

    pub fn ensure_approved(&self) -> Result<(), RemediationError> {
        match &self.approval {
            ApprovalState::NotRequired => Ok(()),
            ApprovalState::Required => Err(RemediationError::MissingApproval {
                issue_id: self.finding_ids.first().cloned().unwrap_or_default(),
            }),
            ApprovalState::Approved {
                proposal_id,
                proposal_hash,
                token,
                ..
            } if proposal_id == &self.proposal_id
                && proposal_hash == &self.proposal_hash
                && self.compute_hash() == self.proposal_hash
                && token == &self.approval_token() =>
            {
                Ok(())
            }
            ApprovalState::Approved { .. } => Err(RemediationError::MissingApproval {
                issue_id: self.finding_ids.first().cloned().unwrap_or_default(),
            }),
        }
    }

    pub fn compute_hash(&self) -> String {
        let mut hash = 0xcbf29ce484222325_u64;
        for field in [
            &self.proposal_id,
            &self.finding_ids.join("\0"),
            &self.diff,
            &self.files.join("\0"),
            &self.rationale,
            &self.risks.join("\0"),
            &self.validation_commands.join("\0"),
            &self.expected_effect,
        ] {
            for byte in field.as_bytes() {
                hash = (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3);
            }
            hash = (hash ^ 0xff).wrapping_mul(0x100000001b3);
        }
        format!("rgaa-proposal-v1-{hash:016x}")
    }
}

pub fn remediate(
    issues: &[RemediationIssue],
    policy: &RemediationPolicy,
    adapter: &dyn FrameworkAdapter,
) -> Result<Vec<RemediationOutcome>, RemediationError> {
    if !(1..=25).contains(&issues.len()) || issues.len() > policy.max_batch_size {
        return Err(RemediationError::InvalidBatchSize {
            actual: issues.len(),
        });
    }
    Ok(issues
        .iter()
        .map(|issue| match policy.check(issue) {
            Ok(()) => match adapter.propose(issue, &issue.element_html) {
                Ok(mut proposal) => {
                    proposal.set_approval_required(policy.require_approval);
                    RemediationOutcome::Ok(RemediationGuidance {
                        issue_id: issue.id.clone(),
                        explanation: issue.summary.clone(),
                        steps: vec![issue.remediation.clone()],
                        confidence: "high".into(),
                        criteria: issue.criteria.clone(),
                        proposal,
                    })
                }
                Err(error) => error_outcome(issue, error),
            },
            Err(error) => error_outcome(issue, error),
        })
        .collect())
}

fn error_outcome(issue: &RemediationIssue, error: RemediationError) -> RemediationOutcome {
    let (code, message) = match &error {
        RemediationError::InvalidIssue { message, .. } => {
            (RemediationErrorCode::InvalidIssue, message.clone())
        }
        RemediationError::PolicyDenied { reason, .. } => {
            (RemediationErrorCode::PolicyDenied, reason.clone())
        }
        RemediationError::NeedsReview { reason, .. } => {
            (RemediationErrorCode::NeedsReview, reason.clone())
        }
        RemediationError::UnsupportedFramework { .. } => (
            RemediationErrorCode::UnsupportedFramework,
            error.to_string(),
        ),
        RemediationError::MissingSourceLocation { .. } => (
            RemediationErrorCode::MissingSourceLocation,
            error.to_string(),
        ),
        RemediationError::MissingApproval { .. } | RemediationError::InvalidApproval { .. } => {
            (RemediationErrorCode::MissingApproval, error.to_string())
        }
        RemediationError::InvalidTransition { .. } | RemediationError::InvalidBatchSize { .. } => {
            (RemediationErrorCode::ModelFailure, error.to_string())
        }
    };
    RemediationOutcome::Error(RemediationErrorInfo {
        issue_id: issue.id.clone(),
        code,
        message,
    })
}
