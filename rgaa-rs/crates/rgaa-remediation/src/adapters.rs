use crate::{PatchProposal, RemediationError, RemediationIssue, SourceLocation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Framework {
    React,
    Next,
    Vue,
    Angular,
}

pub trait FrameworkAdapter: Send + Sync {
    fn framework(&self) -> Framework;
    fn detect(&self, source: &str) -> Framework;
    fn locate(&self, source: &str, issue: &RemediationIssue) -> Vec<SourceLocation>;
    fn propose(
        &self,
        issue: &RemediationIssue,
        source: &str,
    ) -> Result<PatchProposal, RemediationError>;
}

pub struct ReactAdapter;
pub struct NextAdapter;
pub struct VueAdapter;
pub struct AngularAdapter;

pub fn adapter_for(framework: Framework) -> &'static dyn FrameworkAdapter {
    match framework {
        Framework::React => &ReactAdapter,
        Framework::Next => &NextAdapter,
        Framework::Vue => &VueAdapter,
        Framework::Angular => &AngularAdapter,
    }
}

fn propose_for(
    framework: Framework,
    issue: &RemediationIssue,
    source: &str,
) -> Result<PatchProposal, RemediationError> {
    let trimmed = source.trim();
    let diff = if issue.rule.contains("image")
        && trimmed.contains("<img")
        && !trimmed.contains(" alt=")
    {
        let image_tag = trimmed
            .split("<img")
            .nth(1)
            .and_then(|rest| rest.split('>').next())
            .unwrap_or_default();
        if image_tag.contains('{') {
            return Err(RemediationError::NeedsReview {
                issue_id: issue.id.clone(),
                reason: "image source or content is dynamic".into(),
            });
        }
        source.replacen("<img", "<img alt=\"\"", 1)
    } else if issue.rule.contains("label") || issue.rule.contains("input") {
        if trimmed.contains("id=") && trimmed.contains("<label") {
            source.to_owned()
        } else {
            return Err(RemediationError::NeedsReview {
                issue_id: issue.id.clone(),
                reason: "control-label association is ambiguous".into(),
            });
        }
    } else if issue.rule.contains("button") && trimmed.contains("<button") && trimmed.contains("><")
    {
        return Err(RemediationError::NeedsReview {
            issue_id: issue.id.clone(),
            reason: "button name depends on rendered content".into(),
        });
    } else {
        return Err(RemediationError::NeedsReview {
            issue_id: issue.id.clone(),
            reason: "pattern is not high confidence".into(),
        });
    };
    let file = issue
        .source_locations
        .first()
        .ok_or_else(|| RemediationError::MissingSourceLocation {
            issue_id: issue.id.clone(),
        })?
        .file
        .clone();
    Ok(PatchProposal::new(
        format!("{}-proposal", issue.id),
        vec![issue.id.clone()],
        diff,
        vec![file],
        format!(
            "apply the high-confidence {} remediation for {:?}",
            issue.rule, framework
        ),
        vec!["verify rendered accessibility semantics".into()],
        vec!["run the focused accessibility test".into()],
        "removes the reported accessibility violation",
    ))
}

macro_rules! adapter_impl {
    ($type:ty, $framework:expr) => {
        impl FrameworkAdapter for $type {
            fn framework(&self) -> Framework {
                $framework
            }
            fn detect(&self, _source: &str) -> Framework {
                $framework
            }
            fn locate(&self, _source: &str, issue: &RemediationIssue) -> Vec<SourceLocation> {
                issue.source_locations.clone()
            }
            fn propose(
                &self,
                issue: &RemediationIssue,
                source: &str,
            ) -> Result<PatchProposal, RemediationError> {
                propose_for($framework, issue, source)
            }
        }
    };
}

adapter_impl!(ReactAdapter, Framework::React);
adapter_impl!(NextAdapter, Framework::Next);
adapter_impl!(VueAdapter, Framework::Vue);
adapter_impl!(AngularAdapter, Framework::Angular);
