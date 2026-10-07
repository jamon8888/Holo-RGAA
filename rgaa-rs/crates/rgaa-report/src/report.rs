use std::fmt::Write;

use rgaa_core::{AuditBundle, CriterionStatus, Finding};

pub mod html;

use crate::format::ReportFormat;
use crate::ReportError;

pub fn render(bundle: &AuditBundle, format: ReportFormat) -> Result<String, ReportError> {
    match format {
        ReportFormat::Json => serde_json::to_string_pretty(bundle)
            .map_err(|error| ReportError::execution(error.to_string())),
        ReportFormat::Markdown => Ok(render_markdown(bundle)),
        ReportFormat::Sarif => Ok(render_sarif(bundle)),
        ReportFormat::Junit => Ok(render_junit(bundle)),
        ReportFormat::Html => Ok(html::generate_html_report(bundle)),
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

    write_markdown_sources(&mut out, bundle);

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

/// Per-criterion RAG evidence.
///
/// Emitted only for criteria that actually carry citations. A
/// deterministic pass, a manual review and a "not tested" legitimately have
/// none, so listing all 106 here would bury the sourced handful in 100 rows
/// of "—" and make the absence of evidence look like a rendering gap rather
/// than a property of the verdict.
fn write_markdown_sources(out: &mut String, bundle: &AuditBundle) {
    let sourced = crate::sources::sourced_criteria(bundle);
    if sourced.is_empty() {
        return;
    }
    let _ = writeln!(out, "## Sources");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "Criteria whose verdict relied on retrieved documents, and what was retrieved."
    );
    let _ = writeln!(out);
    let mut current_page: Option<&str> = None;
    for (page_url, criterion) in sourced {
        if current_page != Some(page_url) {
            let _ = writeln!(out, "### {page_url}");
            let _ = writeln!(out);
            current_page = Some(page_url);
        }
        let title = if criterion.title.is_empty() {
            "—"
        } else {
            criterion.title.as_str()
        };
        let _ = writeln!(
            out,
            "- **{}** {} — {}",
            criterion.criterion_id,
            title,
            status_str(&criterion.status)
        );
        if let Some(line) = crate::sources::sources_line(criterion) {
            let _ = writeln!(out, "  - Sources: {line}");
        }
    }
    let _ = writeln!(out);
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

    /// A bundle with one page carrying a sourced criterion (RAG verdict)
    /// and an unsourced one (deterministic verdict).
    fn bundle_with_citations() -> AuditBundle {
        use rgaa_core::{Citation, Classification, PageAudit};

        let sourced = rgaa_core::CriterionResult {
            criterion_id: "1.1".into(),
            title: "Image porteuse d'information".into(),
            classification: Classification::IaAssiste,
            status: CriterionStatus::Fail,
            violations: Vec::new(),
            confidence: None,
            raw_confidence: Some(0.87),
            justification: Some("alternative absente".into()),
            source: "holo3".into(),
            citations: vec![
                Citation::referentiel("1.1.1", "2024.1"),
                Citation::crawl(
                    "https://example.test/accueil",
                    "2025-01-01T00:00:00Z",
                    "sha256:abc",
                ),
            ],
            considered_sources: Vec::new(),
            tests: Vec::new(),
            automated_verdict: None,
            verdict_basis: Vec::new(),
            evidence: Vec::new(),
            confidence_calibration_version: None,
            review_required: false,
            review_reason: None,
            verified_status: None,
            review_events: Vec::new(),
        };
        let unsourced = rgaa_core::CriterionResult {
            criterion_id: "8.1".into(),
            title: "Document valide".into(),
            classification: Classification::Deterministe,
            status: CriterionStatus::Pass,
            violations: Vec::new(),
            confidence: None,
            raw_confidence: None,
            justification: None,
            source: "axe-core".into(),
            citations: Vec::new(),
            considered_sources: Vec::new(),
            tests: Vec::new(),
            automated_verdict: None,
            verdict_basis: Vec::new(),
            evidence: Vec::new(),
            confidence_calibration_version: None,
            review_required: false,
            review_reason: None,
            verified_status: None,
            review_events: Vec::new(),
        };

        let mut bundle =
            AuditBundle::new("audit-cit", "https://example.test", AuditConfig::default());
        bundle.pages.push(PageAudit {
            page_id: "page-1".into(),
            url: "https://example.test/accueil".into(),
            title: Some("Accueil".into()),
            criteria: vec![sourced, unsourced],
            findings: Vec::new(),
            errors: Vec::new(),
            completed: true,
            duration_ms: 10,
        });
        bundle
    }

    /// The citations field has existed since the dual-router work and the
    /// evaluator populates it, but no renderer read it — a sourced verdict
    /// reached the reader looking exactly like an unsourced one.
    #[test]
    fn markdown_renders_the_sources_behind_a_rag_verdict() {
        let output = render(&bundle_with_citations(), ReportFormat::Markdown).expect("markdown");
        assert!(output.contains("## Sources"), "{output}");
        assert!(
            output.contains("1.1.1"),
            "référentiel test id missing: {output}"
        );
        assert!(
            output.contains("2024.1"),
            "référentiel version missing — a verdict is only valid against the version it was checked against: {output}"
        );
        assert!(
            output.contains("sha256:abc"),
            "evidence hash missing — the crawl index is purged, the hash is what survives: {output}"
        );
    }

    /// An unsourced verdict must not be padded with a fabricated source.
    #[test]
    fn markdown_sources_section_omits_unsourced_criteria() {
        let output = render(&bundle_with_citations(), ReportFormat::Markdown).expect("markdown");
        let sources = output
            .split("## Sources")
            .nth(1)
            .expect("sources section")
            .split("## Findings")
            .next()
            .unwrap_or("");
        assert!(sources.contains("1.1"), "{sources}");
        assert!(
            !sources.contains("Document valide"),
            "deterministic pass leaked into Sources: {sources}"
        );
    }

    /// No citations anywhere means no section at all, rather than an empty
    /// heading that reads as missing evidence.
    #[test]
    fn markdown_omits_the_sources_section_when_nothing_is_sourced() {
        let output = render(&sample_bundle(), ReportFormat::Markdown).expect("markdown");
        assert!(!output.contains("## Sources"), "{output}");
    }

    #[test]
    fn html_criteria_table_carries_a_sources_column() {
        let output = render(&bundle_with_citations(), ReportFormat::Html).expect("html");
        assert!(output.contains("<th>Sources</th>"), "{output}");
        assert!(output.contains("2024.1"), "{output}");
        assert!(output.contains("sha256:abc"), "{output}");
    }

    /// Every criteria row must have as many cells as that table's header has
    /// columns. Adding a column to the header and not the rows (or the
    /// reverse) shifts every value one cell left and is invisible in a diff:
    /// the table still renders, it just attributes each criterion's detail
    /// to the wrong column.
    #[test]
    fn html_criteria_rows_match_the_header_width() {
        let output = render(&bundle_with_citations(), ReportFormat::Html).expect("html");

        // Anchor on the criteria table specifically — the report also
        // renders a findings table with a different width.
        let table = output
            .split("Détail complet des critères")
            .nth(1)
            .expect("criteria table");
        let header = table.split("<tbody>").next().expect("criteria thead");
        let columns = header.matches("<th>").count();
        assert_eq!(columns, 6, "criteria header is {columns} columns wide");

        let body = table
            .split("<tbody>")
            .nth(1)
            .expect("criteria tbody")
            .split("</tbody>")
            .next()
            .expect("tbody close");
        let rows = body.matches("<tr>").count();
        let cells = body.matches("<td>").count();
        assert_eq!(rows, 106, "the table promises all 106 criteria, got {rows}");
        assert_eq!(
            cells,
            rows * columns,
            "{cells} cells across {rows} rows is not {columns} per row"
        );
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
