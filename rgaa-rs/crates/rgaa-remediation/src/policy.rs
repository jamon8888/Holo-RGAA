use crate::{Framework, RemediationError, RemediationIssue};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemediationPolicy {
    pub allow_remote_ai: bool,
    pub use_remote_ai: bool,
    pub allowed_frameworks: Vec<Framework>,
    pub max_batch_size: usize,
    pub require_approval: bool,
}

impl Default for RemediationPolicy {
    fn default() -> Self {
        Self {
            allow_remote_ai: false,
            use_remote_ai: false,
            allowed_frameworks: vec![
                Framework::React,
                Framework::Next,
                Framework::Vue,
                Framework::Angular,
            ],
            max_batch_size: 25,
            require_approval: true,
        }
    }
}

impl RemediationPolicy {
    pub fn check(&self, issue: &RemediationIssue) -> Result<(), RemediationError> {
        if self.use_remote_ai && !self.allow_remote_ai {
            return Err(RemediationError::PolicyDenied {
                issue_id: issue.id.clone(),
                reason: "remote remediation is disabled by policy".into(),
            });
        }
        if issue.id.trim().is_empty()
            || issue.rule.trim().is_empty()
            || issue.page_url.trim().is_empty()
        {
            return Err(RemediationError::InvalidIssue {
                issue_id: issue.id.clone(),
                message: "id, rule, and page_url are required".into(),
            });
        }
        if issue.source_locations.is_empty() {
            return Err(RemediationError::MissingSourceLocation {
                issue_id: issue.id.clone(),
            });
        }
        if let Some(framework) = issue.framework {
            if !self.allowed_frameworks.contains(&framework) {
                return Err(RemediationError::PolicyDenied {
                    issue_id: issue.id.clone(),
                    reason: "framework is not allowed".into(),
                });
            }
        }
        Ok(())
    }
}
