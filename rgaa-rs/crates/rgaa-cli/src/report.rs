use std::fmt::Write;

use rgaa_core::{AuditBundle, CriterionStatus, Finding};

use crate::format::ReportFormat;
use crate::CliError;

/// Renders an audit bundle into the specified report format.
///
/// Supports JSON, Markdown, SARIF 2.1.0, and JUnit XML output formats.
///
/// # Arguments
///
/// * `bundle` - The audit bundle containing findings and metadata.
/// * `format` - The desired output format.
///
/// # Returns
///
/// A `Result` containing the rendered report string or a `CliError`.
///
/// # Errors
///
/// Returns `CliError::Execution` if JSON serialization fails.
pub fn render(bundle: &AuditBundle, format: ReportFormat) -> Result<String, CliError> {
    match format {
        ReportFormat::Json => serde_json::to_string_pretty(bundle)
            .map_err(|error| CliError::execution(error.to_string())),
        ReportFormat::Markdown => Ok(render_markdown(bundle)),
        ReportFormat::Sarif => Ok(render_sarif(bundle)),
        ReportFormat::Junit => Ok(render_junit(bundle)),
    }
}

fn render_markdown(bundle: &AuditBundle) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# RGAA Audit Report: {}", bundle.audit_id);
    let _ = writeln!(out);
    let _ = writeln!(out, "- URL: {}", bundle.url);
    let _ = writeln!(out, "- Schema: {}", bundle.schema_version);
    let _ = writeln!(
        out,
        "- Pages: {} (completed: {})",
        bundle.summary.total_pages, bundle.summary.completed_pages
    );
    let _ = writeln!(out, "- Findings: {}", bundle.summary.total_findings);
    let _ = writeln!(out);
    let _ = writeln!(out, "## Summary");
    let _ = writeln!(out);
    let _ = writeln!(out, "| Status | Count |");
    let _ = writeln!(out, "| --- | --- |");
    let _ = writeln!(out, "| Pass | {} |", bundle.summary.passed);
    let _ = writeln!(out, "| Fail | {} |", bundle.summary.failed);
    let _ = writeln!(out, "| Needs review | {} |", bundle.summary.needs_review);
    let _ = writeln!(out, "| Errors | {} |", bundle.summary.errors);
    let _ = writeln!(out);

    let findings = all_findings(bundle);
    if findings.is_empty() {
        let _ = writeln!(out, "No findings.");
        return out;
    }
    let _ = writeln!(out, "## Findings");
    let _ = writeln!(out);
    for finding in findings {
        let severity = finding.severity.as_deref().unwrap_or("unknown");
        let status = status_str(&finding.status);
        let _ = writeln!(
            out,
            "- **{}** `{}` — {} ({}, {})",
            finding.id, finding.rule, finding.target, severity, status
        );
        if let Some(description) = &finding.description {
            let _ = writeln!(out, "  - {}", description);
        }
        if let Some(remediation) = &finding.remediation {
            let _ = writeln!(out, "  - Remediation: {}", remediation);
        }
    }
    out
}

fn render_sarif(bundle: &AuditBundle) -> String {
    let findings = all_findings(bundle);
    let mut rules = Vec::new();
    let mut results = Vec::new();
    let mut seen_rules = std::collections::HashSet::new();

    for finding in findings {
        if seen_rules.insert(finding.rule.clone()) {
            rules.push(serde_json::json!({
                "id": finding.rule,
                "name": finding.rule,
                "shortDescription": { "text": finding.description.clone().unwrap_or_default() },
            }));
        }
        let level = match &finding.status {
            CriterionStatus::Fail => "error",
            CriterionStatus::NeedsReview => "warning",
            CriterionStatus::Error => "error",
            _ => "note",
        };
        results.push(serde_json::json!({
            "ruleId": finding.rule,
            "level": level,
            "message": { "text": finding.description.clone().unwrap_or_else(|| finding.id.clone()) },
            "locations": [{
                "physicalLocation": {
                    "artifactLocation": { "uri": finding.target },
                }
            }],
        }));
    }

    let sarif = serde_json::json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": { "driver": { "name": "rgaa", "informationUri": "https://rgaa.test", "rules": rules } },
            "results": results,
        }],
    });
    serde_json::to_string_pretty(&sarif).unwrap_or_else(|_| "{}".into())
}

fn render_junit(bundle: &AuditBundle) -> String {
    let findings = all_findings(bundle);
    let mut failures = 0usize;
    let mut errors = 0usize;
    let mut cases = String::new();

    for finding in &findings {
        match &finding.status {
            CriterionStatus::Fail | CriterionStatus::NeedsReview => {
                failures += 1;
                let message = finding.description.clone().unwrap_or_default();
                let _ = writeln!(
                    cases,
                    "    <testcase name=\"{}\" classname=\"{}\"><failure message=\"{}\"/></testcase>",
                    escape_xml(&finding.id),
                    escape_xml(&finding.rule),
                    escape_xml(&message),
                );
            }
            CriterionStatus::Error => {
                errors += 1;
                let message = finding.description.clone().unwrap_or_default();
                let _ = writeln!(
                    cases,
                    "    <testcase name=\"{}\" classname=\"{}\"><error message=\"{}\"/></testcase>",
                    escape_xml(&finding.id),
                    escape_xml(&finding.rule),
                    escape_xml(&message),
                );
            }
            _ => {
                let _ = writeln!(
                    cases,
                    "    <testcase name=\"{}\" classname=\"{}\"/>",
                    escape_xml(&finding.id),
                    escape_xml(&finding.rule),
                );
            }
        }
    }

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<testsuites>\n  <testsuite name=\"rgaa\" tests=\"{}\" failures=\"{}\" errors=\"{}\">\n{}</testsuite>\n</testsuites>\n",
        findings.len(),
        failures,
        errors,
        cases,
    )
}

fn all_findings(bundle: &AuditBundle) -> Vec<&Finding> {
    bundle
        .findings
        .iter()
        .chain(bundle.pages.iter().flat_map(|page| page.findings.iter()))
        .collect()
}

fn status_str(status: &CriterionStatus) -> &'static str {
    match status {
        CriterionStatus::Pass => "pass",
        CriterionStatus::Fail => "fail",
        CriterionStatus::NotApplicable => "not_applicable",
        CriterionStatus::Error => "error",
        CriterionStatus::NeedsReview => "needs_review",
        CriterionStatus::NotTested => "not_tested",
    }
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgaa_core::AuditConfig;

    fn sample_bundle() -> AuditBundle {
        let mut bundle =
            AuditBundle::new("audit-1", "https://example.test", AuditConfig::default());
        bundle.summary.total_pages = 1;
        bundle.summary.completed_pages = 1;
        bundle.summary.total_findings = 1;
        bundle.summary.failed = 1;
        let mut finding = rgaa_core::Finding::new("finding-1");
        finding.rule = "rgaa-1.1".into();
        finding.url = "https://example.test".into();
        finding.target = "#main".into();
        finding.status = CriterionStatus::Fail;
        finding.severity = Some("critical".into());
        finding.description = Some("missing alternative text".into());
        bundle.findings.push(finding);
        bundle
    }

    #[test]
    fn json_round_trips_the_bundle() {
        let bundle = sample_bundle();
        let output = render(&bundle, ReportFormat::Json).expect("json");
        let decoded: AuditBundle = serde_json::from_str(&output).expect("valid bundle");
        assert_eq!(decoded.audit_id, "audit-1");
    }

    #[test]
    fn markdown_groups_and_lists_findings() {
        let output = render(&sample_bundle(), ReportFormat::Markdown).expect("markdown");
        assert!(output.contains("# RGAA Audit Report: audit-1"));
        assert!(output.contains("finding-1"));
        assert!(output.contains("critical"));
    }

    #[test]
    fn sarif_has_rules_and_results() {
        let output = render(&sample_bundle(), ReportFormat::Sarif).expect("sarif");
        let value: serde_json::Value = serde_json::from_str(&output).expect("valid sarif json");
        assert_eq!(value["version"], "2.1.0");
        assert_eq!(value["runs"][0]["results"][0]["level"], "error");
    }

    #[test]
    fn junit_is_valid_xml_shape() {
        let output = render(&sample_bundle(), ReportFormat::Junit).expect("junit");
        assert!(output.contains("<testsuite name=\"rgaa\""));
        assert!(output.contains("<failure"));
        assert!(output.contains("tests=\"1\""));
    }
}
