use serde::{Deserialize, Serialize};
use thiserror::Error;

/// State of a finding in the remediation lifecycle.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FindingState {
    /// Newly discovered finding.
    Open,
    /// Finding has been triaged and prioritized.
    Triaged,
    /// A fix has been proposed for the finding.
    FixProposed,
    /// Fix is awaiting human approval.
    AwaitingApproval,
    /// Fix has been applied to the codebase.
    Applied,
    /// Fix is being verified.
    Verifying,
    /// Finding has been resolved and verified.
    Resolved,
    /// Finding requires human review.
    NeedsReview,
    /// Finding is not applicable to this context.
    NotApplicable,
    /// Finding has been marked as a false positive.
    FalsePositive,
    /// Finding has been deferred to a later date.
    Deferred,
}

/// Record of a state transition in the finding lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LifecycleEntry {
    /// The state before the transition.
    pub from: FindingState,
    /// The state after the transition.
    pub to: FindingState,
    /// The actor who performed the transition.
    pub actor: String,
    /// The reason for the transition.
    pub reason: String,
}

/// Errors that can occur during remediation operations.
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

/// Tracks the lifecycle of a finding through state transitions.
///
/// Maintains a history of all state changes with actor and reason information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FindingLifecycle {
    /// The unique identifier of the finding.
    pub finding_id: String,
    /// The current state of the finding.
    pub state: FindingState,
    /// History of all state transitions.
    history: Vec<LifecycleEntry>,
}

impl FindingLifecycle {
    /// Creates a new finding lifecycle starting in the `Open` state.
    ///
    /// # Arguments
    ///
    /// * `finding_id` - The unique identifier of the finding.
    pub fn new(finding_id: impl Into<String>) -> Self {
        Self {
            finding_id: finding_id.into(),
            state: FindingState::Open,
            history: Vec::new(),
        }
    }

    /// Transitions the finding to a new state.
    ///
    /// # Arguments
    ///
    /// * `next` - The target state to transition to.
    /// * `actor` - The actor performing the transition.
    /// * `reason` - The reason for the transition.
    ///
    /// # Errors
    ///
    /// Returns `RemediationError::InvalidTransition` if the transition is not allowed
    /// or if actor/reason are empty.
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
