use std::path::Path;

use rgaa_remediation::{RemediationIssue, RemediationOutcome};

use crate::commands::write_output;
use crate::commands::CommonArgs;
use crate::CliError;

#[derive(Debug, clap::Args)]
pub struct VerifyArgs {
    #[clap(flatten)]
    pub common: CommonArgs,
    #[clap(long, value_name = "ISSUES")]
    pub issues: std::path::PathBuf,
}

pub fn run(args: VerifyArgs) -> Result<i32, CliError> {
    let issues = load_issues(&args.issues)?;
    if issues.is_empty() {
        return Err(CliError::invalid_input(
            "issues file must contain at least one issue",
        ));
    }
    if issues.len() > 25 {
        return Err(CliError::invalid_input(
            "issues file must contain at most 25 issues",
        ));
    }

    let policy = rgaa_remediation::RemediationPolicy::default();
    let outcomes = remediate_all(&issues, &policy)?;

    let rendered = render_outcomes(&outcomes);
    write_output(&args.common.output, &rendered)?;

    let all_ok = outcomes
        .iter()
        .all(|outcome| matches!(outcome, RemediationOutcome::Ok(_)));
    Ok(if all_ok { 0 } else { 1 })
}

fn load_issues(path: &Path) -> Result<Vec<RemediationIssue>, CliError> {
    let raw = std::fs::read_to_string(path)
        .map_err(|error| CliError::execution(format!("failed to read issues: {error}")))?;
    serde_json::from_str(&raw)
        .map_err(|error| CliError::invalid_input(format!("invalid issues: {error}")))
}

fn remediate_all(
    issues: &[RemediationIssue],
    policy: &rgaa_remediation::RemediationPolicy,
) -> Result<Vec<RemediationOutcome>, CliError> {
    let mut outcomes = Vec::with_capacity(issues.len());
    for issue in issues {
        let framework = match issue.framework {
            Some(framework) => framework,
            None => rgaa_remediation::detect_framework(&issue.element_html)
                .unwrap_or(rgaa_remediation::Framework::React),
        };
        let batch = rgaa_remediation::remediate(
            std::slice::from_ref(issue),
            policy,
            rgaa_remediation::adapter_for(framework),
        )
        .map_err(|error| CliError::execution(error.to_string()))?;
        outcomes.extend(batch);
    }
    Ok(outcomes)
}

fn render_outcomes(outcomes: &[RemediationOutcome]) -> String {
    let mut lines = String::new();
    for outcome in outcomes {
        match outcome {
            RemediationOutcome::Ok(guidance) => {
                let approval = format!("{:?}", guidance.proposal.approval_state());
                lines.push_str(&format!(
                    "OK {}\n  proposal={}\n  hash={}\n  approval={}\n  token={}\n",
                    guidance.issue_id,
                    guidance.proposal.proposal_id,
                    guidance.proposal.proposal_hash,
                    approval,
                    guidance.proposal.approval_token(),
                ));
            }
            RemediationOutcome::Error(error) => {
                lines.push_str(&format!(
                    "ERROR {} [{:?}] {}\n",
                    error.issue_id, error.code, error.message
                ));
            }
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn issue(id: &str) -> RemediationIssue {
        RemediationIssue {
            id: id.into(),
            rule: "image-alt".into(),
            element_html: "import React from \"react\"; <img src=\"hero.png\">".into(),
            page_url: "https://example.test".into(),
            source_locations: vec![rgaa_remediation::SourceLocation {
                file: "src/App.tsx".into(),
                line: 1,
                column: None,
            }],
            summary: "missing alternative text".into(),
            remediation: "add alt".into(),
            criteria: vec!["RGAA-1.1".into()],
            framework: Some(rgaa_remediation::Framework::React),
        }
    }

    #[test]
    fn all_ok_batch_reports_success() {
        let outcomes = remediate_all(
            &[issue("a"), issue("b")],
            &rgaa_remediation::RemediationPolicy::default(),
        )
        .unwrap();
        assert_eq!(outcomes.len(), 2);
        assert!(outcomes
            .iter()
            .all(|o| matches!(o, RemediationOutcome::Ok(_))));
        let rendered = render_outcomes(&outcomes);
        assert!(rendered.contains("rgaa-approval-v1-"));
        assert!(rendered.contains("Required"));
    }
}
