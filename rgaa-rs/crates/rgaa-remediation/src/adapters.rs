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
    fn detect(&self, source: &str) -> Option<Framework>;
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
    if issue.framework != Some(framework) {
        return Err(RemediationError::UnsupportedFramework {
            issue_id: issue.id.clone(),
        });
    }
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err(RemediationError::NeedsReview {
            issue_id: issue.id.clone(),
            reason: "source is empty".into(),
        });
    }

    let diff = if issue.rule.contains("image") && trimmed.contains("<img") {
        let tag = opening_tag(trimmed, "img").ok_or_else(|| RemediationError::NeedsReview {
            issue_id: issue.id.clone(),
            reason: "image source is incomplete".into(),
        })?;
        if tag.contains("[src")
            || tag.contains(":src")
            || tag.contains("v-bind:src")
            || tag.contains("src={")
            || tag.contains("{{")
        {
            return Err(RemediationError::NeedsReview {
                issue_id: issue.id.clone(),
                reason: "image source uses a dynamic binding".into(),
            });
        }
        if tag.contains(" alt=")
            || tag.contains(" aria-label=")
            || tag.contains(" aria-labelledby=")
        {
            return Err(RemediationError::NeedsReview {
                issue_id: issue.id.clone(),
                reason: "image already has an accessible name".into(),
            });
        }
        insert_attribute(source, "img", " alt=\"\"")
    } else if issue.rule.contains("label")
        || issue.rule.contains("input")
        || issue.rule.contains("control")
    {
        let tag_name = ["input", "select", "textarea"]
            .iter()
            .find(|name| trimmed.contains(&format!("<{name}")))
            .copied()
            .ok_or_else(|| RemediationError::NeedsReview {
                issue_id: issue.id.clone(),
                reason: "control element is ambiguous".into(),
            })?;
        let tag = opening_tag(trimmed, tag_name).unwrap_or_default();
        if tag.contains('[') || tag.contains(':') || tag.contains("v-") || tag.contains("{{") {
            return Err(RemediationError::NeedsReview {
                issue_id: issue.id.clone(),
                reason: "control uses a dynamic binding and its label is ambiguous".into(),
            });
        }
        if tag.contains("aria-label=")
            || tag.contains("aria-labelledby=")
            || (trimmed.contains("<label") && tag.contains("id="))
        {
            return Err(RemediationError::NeedsReview {
                issue_id: issue.id.clone(),
                reason: "control label association is ambiguous".into(),
            });
        }
        let name = attribute_value(tag, "id")
            .or_else(|| attribute_value(tag, "name"))
            .ok_or_else(|| RemediationError::NeedsReview {
                issue_id: issue.id.clone(),
                reason: "control has no stable name for an accessible label".into(),
            })?;
        insert_attribute(source, tag_name, &format!(" aria-label=\"{name}\""))
    } else if issue.rule.contains("button") && trimmed.contains("<button") {
        let tag = opening_tag(trimmed, "button").unwrap_or_default();
        let body = trimmed
            .split_once('>')
            .and_then(|(_, rest)| rest.split_once("</button>").map(|(body, _)| body))
            .unwrap_or_default();
        if tag.contains("aria-label=")
            || tag.contains("aria-labelledby=")
            || tag.contains("title=")
            || body.contains('{')
            || body.contains("{{")
            || body.contains("v-")
        {
            return Err(RemediationError::NeedsReview {
                issue_id: issue.id.clone(),
                reason: "button name depends on dynamic or existing content".into(),
            });
        }
        if !body.trim().is_empty() {
            return Err(RemediationError::NeedsReview {
                issue_id: issue.id.clone(),
                reason: "button already has rendered content".into(),
            });
        }
        insert_attribute(source, "button", " aria-label=\"Submit\"")
    } else {
        return Err(RemediationError::NeedsReview {
            issue_id: issue.id.clone(),
            reason: "pattern is not high confidence".into(),
        });
    };

    if diff == source || diff.trim().is_empty() {
        return Err(RemediationError::NeedsReview {
            issue_id: issue.id.clone(),
            reason: "remediation would not change the source".into(),
        });
    }
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

fn opening_tag<'a>(source: &'a str, name: &str) -> Option<&'a str> {
    source.split(&format!("<{name}")).nth(1)?.split('>').next()
}

fn insert_attribute(source: &str, name: &str, attribute: &str) -> String {
    let marker = format!("<{name}");
    let start = source.find(&marker).expect("opening tag was checked");
    let end = source[start..].find('>').expect("opening tag was checked") + start;
    format!(
        "{}{}{}{}",
        &source[..end],
        attribute,
        &source[end..end + 1],
        &source[end + 1..]
    )
}

fn attribute_value<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    tag.split_whitespace().find_map(|part| {
        part.strip_prefix(&format!("{name}=\""))
            .and_then(|value| value.strip_suffix('"'))
    })
}

fn detects(framework: Framework, source: &str) -> Option<Framework> {
    let found = match framework {
        Framework::React => {
            source.contains("from \"react\"")
                || source.contains("from 'react'")
                || source.contains("className=")
                || source.contains("import React")
        }
        Framework::Next => {
            source.contains("\"use client\"")
                || source.contains("'use client'")
                || source.contains("from \"next/")
                || source.contains("from 'next/")
        }
        Framework::Vue => {
            source.contains("<template")
                || source.contains("v-model")
                || source.contains("<script setup")
        }
        Framework::Angular => {
            source.contains("@Component")
                || source.contains("[ngModel]")
                || source.contains("[(ngModel)]")
                || source.contains("*ngIf")
        }
    };
    found.then_some(framework)
}

macro_rules! adapter_impl {
    ($type:ty, $framework:expr) => {
        impl FrameworkAdapter for $type {
            fn framework(&self) -> Framework {
                $framework
            }
            fn detect(&self, source: &str) -> Option<Framework> {
                detects($framework, source)
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
