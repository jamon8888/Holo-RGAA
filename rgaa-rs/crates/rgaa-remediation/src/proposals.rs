use crate::{Framework, FrameworkAdapter, RemediationError, RemediationPolicy};
use serde::{Deserialize, Serialize};

/// Source code location for a remediation issue.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceLocation {
    /// The file path.
    pub file: String,
    /// The line number (1-indexed).
    pub line: u32,
    /// Optional column number (1-indexed).
    pub column: Option<u32>,
}

/// An accessibility issue requiring remediation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemediationIssue {
    /// Unique identifier for the issue.
    pub id: String,
    /// The RGAA rule that was violated.
    pub rule: String,
    /// HTML snippet of the problematic element.
    pub element_html: String,
    /// URL of the page where the issue was found.
    pub page_url: String,
    /// Source code locations where the fix should be applied.
    pub source_locations: Vec<SourceLocation>,
    /// Human-readable summary of the issue.
    pub summary: String,
    /// Recommended remediation steps.
    pub remediation: String,
    /// RGAA criteria affected by this issue.
    pub criteria: Vec<String>,
    /// The frontend framework this issue belongs to.
    #[serde(default)]
    pub framework: Option<Framework>,
}

/// Guidance for remediating an issue, including a patch proposal.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemediationGuidance {
    /// The issue ID this guidance applies to.
    pub issue_id: String,
    /// Detailed explanation of the issue and fix.
    pub explanation: String,
    /// Step-by-step remediation instructions.
    pub steps: Vec<String>,
    /// Confidence level of the proposed fix.
    pub confidence: String,
    /// RGAA criteria affected.
    pub criteria: Vec<String>,
    /// The proposed patch.
    pub proposal: PatchProposal,
}

/// Information about a remediation error.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemediationErrorInfo {
    /// The issue ID that failed.
    pub issue_id: String,
    /// The error code.
    pub code: RemediationErrorCode,
    /// Human-readable error message.
    pub message: String,
}

/// Error codes for remediation failures.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RemediationErrorCode {
    /// The issue is invalid or missing required fields.
    InvalidIssue,
    /// Policy denied the remediation attempt.
    PolicyDenied,
    /// The issue requires human review.
    NeedsReview,
    /// The framework is not supported.
    UnsupportedFramework,
    /// Source location is missing.
    MissingSourceLocation,
    /// Approval is required but not provided.
    MissingApproval,
    /// The AI model failed to generate a fix.
    ModelFailure,
}

/// Outcome of a remediation attempt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum RemediationOutcome {
    /// Successful remediation with guidance.
    Ok(RemediationGuidance),
    /// Failed remediation with error information.
    Error(RemediationErrorInfo),
}

/// A proposed patch for fixing accessibility issues.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PatchProposal {
    /// Unique identifier for this proposal.
    pub proposal_id: String,
    /// IDs of findings this proposal addresses.
    pub finding_ids: Vec<String>,
    /// The diff to apply.
    pub diff: String,
    /// Files that will be modified.
    pub files: Vec<String>,
    /// Rationale for the proposed changes.
    pub rationale: String,
    /// Potential risks of applying this patch.
    pub risks: Vec<String>,
    /// Commands to validate the fix.
    pub validation_commands: Vec<String>,
    /// Expected effect of applying the patch.
    pub expected_effect: String,
    /// Hash of the proposal for integrity verification.
    pub proposal_hash: String,
    /// Approval state (not serialized).
    #[serde(skip)]
    approval: ApprovalState,
}

/// Approval state for a patch proposal.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ApprovalState {
    /// Approval is required but not yet granted.
    #[default]
    Required,
    /// No approval is required.
    NotRequired,
    /// Proposal has been approved.
    Approved {
        /// The proposal ID.
        proposal_id: String,
        /// Hash of the proposal.
        proposal_hash: String,
        /// Who approved the proposal.
        approver: String,
        /// Approval token for verification.
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
