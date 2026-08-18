use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FindingState {
    Open,
    Triaged,
    FixProposed,
    AwaitingApproval,
    Applied,
    Verifying,
    Resolved,
    NeedsReview,
    NotApplicable,
    FalsePositive,
    Deferred,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LifecycleEntry {
    pub from: FindingState,
    pub to: FindingState,
    pub actor: String,
    pub reason: String,
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum RemediationError {
    #[error("invalid finding transition from {from:?} to {to:?}")]
    InvalidTransition {
        from: FindingState,
        to: FindingState,
    },
    #[error("batch must contain between 1 and 25 issues")]
    InvalidBatchSize { actual: usize },
    #[error("issue {issue_id} is invalid: {message}")]
    InvalidIssue { issue_id: String, message: String },
    #[error("remediation policy denied issue {issue_id}: {reason}")]
    PolicyDenied { issue_id: String, reason: String },
    #[error("issue {issue_id} needs human review: {reason}")]
    NeedsReview { issue_id: String, reason: String },
    #[error("unsupported framework for issue {issue_id}")]
    UnsupportedFramework { issue_id: String },
    #[error("source location not available for issue {issue_id}")]
    MissingSourceLocation { issue_id: String },
    #[error("proposal for issue {issue_id} has not been approved")]
    MissingApproval { issue_id: String },
    #[error("approval for issue {issue_id} is invalid")]
    InvalidApproval { issue_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FindingLifecycle {
    pub finding_id: String,
    pub state: FindingState,
    history: Vec<LifecycleEntry>,
}

impl FindingLifecycle {
    pub fn new(finding_id: impl Into<String>) -> Self {
        Self {
            finding_id: finding_id.into(),
            state: FindingState::Open,
            history: Vec::new(),
        }
    }

    pub fn transition(
        &mut self,
        next: FindingState,
        actor: &str,
        reason: &str,
    ) -> Result<(), RemediationError> {
        if actor.trim().is_empty()
            || reason.trim().is_empty()
            || !valid_transition(self.state, next)
        {
            return Err(RemediationError::InvalidTransition {
                from: self.state,
                to: next,
            });
        }
        self.history.push(LifecycleEntry {
            from: self.state,
            to: next,
            actor: actor.to_owned(),
            reason: reason.to_owned(),
        });
        self.state = next;
        Ok(())
    }

    pub fn history(&self) -> &[LifecycleEntry] {
        &self.history
    }
}

fn valid_transition(from: FindingState, to: FindingState) -> bool {
    use FindingState::*;
    matches!(
        (from, to),
        (
            Open,
            Triaged | NeedsReview | NotApplicable | FalsePositive | Deferred
        ) | (
            Triaged,
            FixProposed | NeedsReview | NotApplicable | FalsePositive | Deferred
        ) | (FixProposed, AwaitingApproval | NeedsReview | Deferred)
            | (AwaitingApproval, Applied | NeedsReview | Deferred)
            | (Applied, Verifying)
            | (Verifying, Resolved | NeedsReview)
            | (
                NeedsReview,
                Triaged | Deferred | NotApplicable | FalsePositive
            )
            | (Deferred, Triaged)
            | (NotApplicable, Triaged)
            | (FalsePositive, Triaged)
    )
}
