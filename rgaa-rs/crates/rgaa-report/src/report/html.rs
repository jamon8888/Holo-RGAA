use std::collections::HashMap;
use std::fmt::Write;

use rgaa_core::{
    AuditBundle, AuditSummary, AutomatedVerdict, CriterionResult, CriterionStatus, Finding,
    PageResult, RgaaCriteria, VerdictBasis,
};

pub fn generate_html_report(bundle: &AuditBundle) -> String {
    let mut html = String::new();
    write_html_header(&mut html, &bundle.audit_id, &bundle.url);
    write_html_summary(&mut html, bundle);
    write_html_stats(&mut html, &bundle.summary);
    write_html_findings(&mut html, bundle);
    write_html_all_criteria(&mut html, bundle);
    write_html_footer(&mut html);
    html
}

fn write_html_header(html: &mut String, audit_id: &str, url: &str) {
    let _ = writeln!(
        html,
        r#"<!DOCTYPE html>
<html lang="fr">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>RGAA Audit Report - {}</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; line-height: 1.6; color: #333; background: #f5f5f5; }}
        .container {{ max-width: 1200px; margin: 0 auto; padding: 2rem; }}
        header {{ background: linear-gradient(135deg, #1a5276, #2980b9); color: white; padding: 2rem; border-radius: 8px; margin-bottom: 2rem; }}
        h1 {{ font-size: 1.8rem; margin-bottom: 0.5rem; }}
        .meta {{ opacity: 0.9; font-size: 0.95rem; }}
        .summary {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 1rem; margin-bottom: 2rem; }}
        .card {{ background: white; padding: 1.5rem; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
        .card h3 {{ font-size: 0.85rem; text-transform: uppercase; color: #666; margin-bottom: 0.5rem; }}
        .card .value {{ font-size: 2rem; font-weight: bold; }}
        .card .value.pass {{ color: #27ae60; }}
        .card .value.fail {{ color: #e74c3c; }}
        .card .value.review {{ color: #f39c12; }}
        .card .value.neutral {{ color: #3498db; }}
        .status-badge {{ display: inline-block; padding: 0.25rem 0.75rem; border-radius: 20px; font-size: 0.85rem; font-weight: 500; }}
        .status-badge.pass {{ background: #d4edda; color: #155724; }}
        .status-badge.fail {{ background: #f8d7da; color: #721c24; }}
        .status-badge.review {{ background: #fff3cd; color: #856404; }}
        .status-badge.na {{ background: #e2e3e5; color: #383d41; }}
        table {{ width: 100%; border-collapse: collapse; background: white; border-radius: 8px; overflow: hidden; box-shadow: 0 2px 4px rgba(0,0,0,0.1); margin-bottom: 2rem; }}
        th {{ background: #34495e; color: white; padding: 1rem; text-align: left; font-weight: 500; }}
        td {{ padding: 1rem; border-bottom: 1px solid #eee; }}
        tr:last-child td {{ border-bottom: none; }}
        tr:hover {{ background: #f8f9fa; }}
        .finding-id {{ font-family: monospace; background: #ecf0f1; padding: 0.2rem 0.5rem; border-radius: 4px; font-size: 0.9rem; }}
        .severity {{ padding: 0.2rem 0.5rem; border-radius: 4px; font-size: 0.8rem; text-transform: uppercase; }}
        .severity.critical {{ background: #e74c3c; color: white; }}
        .severity.serious {{ background: #f39c12; color: white; }}
        .severity.moderate {{ background: #3498db; color: white; }}
        .severity.minor {{ background: #95a5a6; color: white; }}
        .no-findings {{ text-align: center; padding: 3rem; color: #27ae60; font-size: 1.2rem; }}
        .no-findings::before {{ content: "✓ "; font-size: 1.5rem; }}
        footer {{ text-align: center; color: #666; font-size: 0.85rem; margin-top: 2rem; padding-top: 1rem; border-top: 1px solid #ddd; }}
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1>RGAA Audit Report</h1>
            <p class="meta">ID: {} | URL: {}</p>
        </header>"#,
        audit_id, audit_id, url
    );
}

fn write_html_summary(html: &mut String, bundle: &AuditBundle) {
    // An audit with technical errors, or a page that never got to every
    // catalog criterion (padded to NotTested by complete_criteria below),
    // is incomplete — neither should be able to show "Conforme" any more
    // than an outright failed criterion can.
    let catalog_size = RgaaCriteria::all().len();
    let metrics = bundle_metrics(bundle);
    let expected_ids: std::collections::HashSet<&str> = RgaaCriteria::all()
        .iter()
        .map(|criterion| criterion.id)
        .collect();
    let is_complete = !bundle.pages.is_empty()
        && expected_ids.len() == catalog_size
        && bundle.pages.iter().all(|page| {
            page.criteria
                .iter()
                .map(|criterion| criterion.criterion_id.as_str())
                .collect::<std::collections::HashSet<_>>()
                .is_superset(&expected_ids)
        });
    let is_conforme =
        metrics.verified_compliance_percent >= 100.0 && bundle.summary.errors == 0 && is_complete;
    let conformity_badge_class = if is_conforme { "pass" } else { "fail" };
    let conformity_text = if is_conforme {
        "Conforme"
    } else {
        "Non Conforme"
    };

    let _ = writeln!(
        html,
        r#"        <div class="summary">
            <div class="card">
                <h3>Conformité vérifiée</h3>
                <div class="value {}">{:.1}%</div>
            </div>
            <div class="card">
                <h3>État de Conformité</h3>
                <div class="value"><span class="status-badge {}">{}</span></div>
            </div>
            <div class="card">
                <h3>Couverture des verdicts automatiques</h3>
                <div class="value neutral">{:.1}%</div>
            </div>
            <div class="card">
                <h3>Couverture des tests avec preuve</h3>
                <div class="value neutral">{:.1}%</div>
            </div>
            <div class="card">
                <h3>Pages Auditées</h3>
                <div class="value neutral">{}/{}</div>
            </div>
        </div>"#,
        conformity_badge_class,
        metrics.verified_compliance_percent,
        conformity_badge_class,
        conformity_text,
        metrics.automatic_verdict_coverage_percent,
        metrics.test_evidence_coverage_percent,
        bundle.summary.completed_pages,
        bundle.summary.total_pages
    );
}

fn write_html_stats(html: &mut String, summary: &AuditSummary) {
    let _ = writeln!(
        html,
        r#"        <table>
            <thead>
                <tr>
                    <th>Statut brut</th>
                    <th>Nombre</th>
                </tr>
            </thead>
            <tbody>
                <tr>
                    <td><span class="status-badge pass">Pass</span></td>
                    <td><strong class="value pass">{}</strong></td>
                </tr>
                <tr>
                    <td><span class="status-badge fail">Fail</span></td>
                    <td><strong class="value fail">{}</strong></td>
                </tr>
                <tr>
                    <td><span class="status-badge review">Needs Review</span></td>
                    <td><strong class="value review">{}</strong></td>
                </tr>
                <tr>
                    <td><span class="status-badge na">Not Applicable</span></td>
                    <td><strong class="value neutral">{}</strong></td>
                </tr>
                <tr>
                    <td><span class="status-badge na">Errors</span></td>
                    <td><strong class="value fail">{}</strong></td>
                </tr>
            </tbody>
        </table>"#,
        summary.passed, summary.failed, summary.needs_review, summary.na, summary.errors
    );
}

fn write_html_findings(html: &mut String, bundle: &AuditBundle) {
    let findings = all_findings(bundle);

    if findings.is_empty() {
        let _ = writeln!(
            html,
            r#"        <div class="no-findings">Aucun problème détecté</div>"#
        );
        return;
    }

    let _ = writeln!(
        html,
        r#"        <h2 style="margin-bottom: 1rem; color: #2c3e50;">Problèmes Détectés ({})</h2>
        <table>
            <thead>
                <tr>
                    <th>Critère</th>
                    <th>Règle</th>
                    <th>Cible</th>
                    <th>Sévérité</th>
                    <th>Description</th>
                </tr>
            </thead>
            <tbody>"#,
        findings.len()
    );

    for finding in findings {
        let severity_class = match finding.severity.as_deref() {
            Some("critical") => "critical",
            Some("serious") => "serious",
            Some("moderate") => "moderate",
            _ => "minor",
        };
        let severity_text = finding.severity.as_deref().unwrap_or("unknown");
        let description = finding.description.as_deref().unwrap_or("N/A");

        let _ = writeln!(
            html,
            r#"                <tr>
                    <td><span class="finding-id">{}</span></td>
                    <td>{}</td>
                    <td>{}</td>
                    <td><span class="severity {}">{}</span></td>
                    <td>{}</td>
                </tr>"#,
            finding.criterion_id.as_deref().unwrap_or("N/A"),
            finding.rule,
            escape_html(&finding.target),
            severity_class,
            severity_text,
            escape_html(description)
        );
    }

    let _ = writeln!(
        html,
        r#"            </tbody>
        </table>"#
    );
}

/// Lists every one of the 106 RGAA criteria for every audited page — pass,
/// fail, needs human review, not applicable, not tested, error — so the
/// report is a complete record, not just aggregate scores.
fn write_html_all_criteria(html: &mut String, bundle: &AuditBundle) {
    for page in &bundle.pages {
        let _ = writeln!(
            html,
            r#"        <h2 style="margin: 2rem 0 1rem; color: #2c3e50;">Détail complet des critères — {}</h2>
        <table>
            <thead>
                <tr>
                    <th>Critère</th>
                    <th>Titre</th>
                    <th>Classification</th>
                    <th>Statut brut</th>
                    <th>Verdict automatique</th>
                    <th>Statut vérifié</th>
                    <th>Détail</th>
                    <th>Évaluation et preuves</th>
                </tr>
            </thead>
            <tbody>"#,
            escape_html(&page.url)
        );

        let mut criteria = complete_criteria(&page.criteria);
        criteria.sort_by(|a, b| {
            status_rank(&a.status)
                .cmp(&status_rank(&b.status))
                .then_with(|| a.criterion_id.cmp(&b.criterion_id))
        });

        for criterion in &criteria {
            let (badge_class, status_label) = status_badge(&criterion.status);
            let title = if criterion.title.is_empty() {
                "—"
            } else {
                criterion.title.as_str()
            };

            let _ = writeln!(
                html,
                r#"                <tr>
                    <td><span class="finding-id">{}</span></td>
                    <td>{}</td>
                    <td>{}</td>
                    <td><span class="status-badge {}">{}</span></td>
                    <td>{}</td>
                    <td>{}</td>
                    <td>{}</td>
                    <td>{}</td>
                </tr>"#,
                escape_html(&criterion.criterion_id),
                escape_html(title),
                classification_label(criterion),
                badge_class,
                status_label,
                automated_verdict_cell(criterion),
                crate::verified_status_for(criterion)
                    .as_ref()
                    .map(status_text)
                    .unwrap_or("Non vérifié"),
                escape_html(&criterion_detail(criterion)),
                assessment_cell(criterion)
            );
        }

        let _ = writeln!(html, "            </tbody>\n        </table>");
    }
}

fn bundle_metrics(bundle: &AuditBundle) -> crate::AuditMetrics {
    let pages: Vec<PageResult> = bundle
        .pages
        .iter()
        .map(|page| PageResult {
            url: page.url.clone(),
            title: page.title.clone(),
            criteria: page.criteria.clone(),
            compliance_rate: 0.0,
            crawl_depth: 0,
        })
        .collect();
    crate::compute_audit_metrics(&pages)
}

fn automated_verdict_cell(criterion: &CriterionResult) -> String {
    let Some(verdict) = criterion.automated_verdict else {
        return "—".into();
    };
    let label = match verdict {
        AutomatedVerdict::Pass => "Conforme",
        AutomatedVerdict::Fail => "Non conforme",
        AutomatedVerdict::NotApplicable => "Non applicable",
    };
    let is_estimate = criterion
        .verdict_basis
        .contains(&VerdictBasis::ModelEstimate)
        || crate::is_model_source(&criterion.source);
    let has_non_model_evidence = criterion.tests.iter().any(|test| {
        !crate::is_model_source(&test.source)
            && test
                .evidence
                .as_deref()
                .is_some_and(|evidence| !evidence.trim().is_empty())
    });
    let is_estimate = is_estimate && !has_non_model_evidence;
    if is_estimate {
        format!("{} <small>(estimation)</small>", label)
    } else {
        label.into()
    }
}

fn status_text(status: &CriterionStatus) -> &'static str {
    match status {
        CriterionStatus::Pass => "Conforme",
        CriterionStatus::Fail => "Non conforme",
        CriterionStatus::NotApplicable => "Non applicable",
        CriterionStatus::Error => "Erreur",
        CriterionStatus::NeedsReview => "À vérifier",
        CriterionStatus::NotTested => "Non testé",
    }
}

fn assessment_cell(criterion: &CriterionResult) -> String {
    let mut parts = Vec::new();
    if let Some(sources) = crate::sources::sources_line(criterion) {
        parts.push(format!("Sources documentaires : {sources}"));
    }
    if !criterion.verdict_basis.is_empty() {
        let basis = criterion
            .verdict_basis
            .iter()
            .map(|basis| match basis {
                VerdictBasis::Axe => "axe",
                VerdictBasis::Deterministic => "déterministe",
                VerdictBasis::Browser => "navigateur",
                VerdictBasis::ModelEstimate => "estimation IA",
            })
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(format!("Fondement : {basis}"));
    }
    if let Some(confidence) = criterion.raw_confidence {
        parts.push(format!("Confiance brute : {:.0}%", confidence * 100.0));
    }
    if let Some(confidence) = criterion.confidence {
        parts.push(format!("Confiance calibrée : {:.0}%", confidence * 100.0));
    }
    parts.push(format!(
        "Revue humaine requise : {}{}",
        if criterion.review_required {
            "oui"
        } else {
            "non"
        },
        criterion
            .review_reason
            .as_deref()
            .filter(|_| criterion.review_required)
            .map(|reason| format!(" — {reason}"))
            .unwrap_or_default()
    ));
    for evidence in &criterion.evidence {
        let location = evidence.location.as_deref().unwrap_or(&evidence.hash);
        parts.push(format!("Preuve {} : {location}", evidence.kind));
    }
    for outcome in &criterion.tests {
        if let Some(evidence) = outcome
            .evidence
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            parts.push(format!(
                "Test {} — {} : {evidence}",
                outcome.test_key, outcome.source
            ));
        }
    }
    for event in &criterion.review_events {
        parts.push(format!(
            "Revue {} par {} le {} : {}",
            status_text(&event.status),
            event.author,
            event.reviewed_at,
            event.reason
        ));
    }
    if parts.is_empty() {
        parts.push("Aucune preuve ou métadonnée d’évaluation".into());
    }
    escape_html(&parts.join(" ; "))
}

/// Pads `results` against the full RGAA catalog so every one of the 106
/// criteria appears — a page whose audit only returned a subset (a partial
/// or failed run) would otherwise silently hide the criteria it never got
/// to, which is the opposite of what a "Détail complet" table promises.
fn complete_criteria(results: &[CriterionResult]) -> Vec<CriterionResult> {
    let mut by_id: HashMap<&str, &CriterionResult> = results
        .iter()
        .map(|c| (c.criterion_id.as_str(), c))
        .collect();

    let mut completed: Vec<CriterionResult> = RgaaCriteria::all()
        .iter()
        .map(|catalog_entry| match by_id.remove(catalog_entry.id) {
            Some(existing) => existing.clone(),
            None => CriterionResult {
                criterion_id: catalog_entry.id.to_string(),
                title: catalog_entry.title.clone(),
                classification: catalog_entry.classification,
                status: CriterionStatus::NotTested,
                violations: vec![],
                confidence: None,
                raw_confidence: None,
                justification: Some("Not tested — missing from audit result".into()),
                source: "missing".into(),
                citations: vec![],
                considered_sources: vec![],
                tests: vec![],
                automated_verdict: None,
                verdict_basis: Vec::new(),
                evidence: Vec::new(),
                confidence_calibration_version: None,
                review_required: false,
                review_reason: None,
                verified_status: None,
                review_events: Vec::new(),
            },
        })
        .collect();

    // Anything left in `by_id` has a criterion_id the catalog doesn't
    // recognize — append it rather than silently dropping it, so a stray or
    // legacy id (including one that failed) still shows up in the "Détail
    // complet" table instead of vanishing.
    completed.extend(by_id.into_values().cloned());
    completed
}

/// Sort order within a page's table: problems first, then what still needs a
/// human, then the rest — so a reviewer sees what needs attention first
/// without having to scroll past 100 passing rows.
fn status_rank(status: &CriterionStatus) -> u8 {
    match status {
        CriterionStatus::Fail => 0,
        CriterionStatus::Error => 1,
        CriterionStatus::NeedsReview => 2,
        CriterionStatus::NotTested => 3,
        CriterionStatus::Pass => 4,
        CriterionStatus::NotApplicable => 5,
    }
}

fn status_badge(status: &CriterionStatus) -> (&'static str, &'static str) {
    match status {
        CriterionStatus::Pass => ("pass", "Valide"),
        CriterionStatus::Fail => ("fail", "À corriger"),
        CriterionStatus::NeedsReview => ("review", "Intervention humaine requise"),
        CriterionStatus::NotApplicable => ("na", "Non applicable"),
        CriterionStatus::NotTested => ("na", "Non testé"),
        CriterionStatus::Error => ("fail", "Erreur"),
    }
}

fn classification_label(criterion: &CriterionResult) -> &'static str {
    match criterion.classification {
        rgaa_core::Classification::Deterministe => "Déterministe",
        rgaa_core::Classification::IaAssiste => "IA assistée",
        rgaa_core::Classification::Manuel => "Manuel",
    }
}

/// Best available explanation for a criterion's verdict: the evaluator's own
/// justification when present, else a summary of the violations that were
/// found, else a placeholder for criteria with neither (e.g. a clean pass).
fn criterion_detail(criterion: &CriterionResult) -> String {
    if let Some(justification) = criterion.justification.as_deref().filter(|j| !j.is_empty()) {
        return justification.to_string();
    }
    if !criterion.violations.is_empty() {
        return criterion
            .violations
            .iter()
            .map(|v| {
                format!(
                    "{} ({}, {} élément(s))",
                    v.description, v.impact, v.nodes_affected
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
    }
    "—".to_string()
}

fn write_html_footer(html: &mut String) {
    let _ = writeln!(
        html,
        r#"        <footer>
            <p>Généré par Holo-RGAA le {}</p>
        </footer>
    </div>
</body>
</html>"#,
        chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")
    );
}

fn all_findings(bundle: &AuditBundle) -> Vec<&Finding> {
    bundle
        .findings
        .iter()
        .chain(bundle.pages.iter().flat_map(|page| page.findings.iter()))
        .filter(|f| f.status == CriterionStatus::Fail || f.status == CriterionStatus::NeedsReview)
        .collect()
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgaa_core::{
        AuditConfig, Classification, EvidenceRef, PageAudit, ReviewEvent, VerdictBasis,
    };

    fn criterion(id: &str, status: CriterionStatus) -> CriterionResult {
        CriterionResult {
            criterion_id: id.to_string(),
            title: String::new(),
            classification: Classification::Deterministe,
            status,
            violations: vec![],
            confidence: None,
            raw_confidence: None,
            justification: None,
            source: "test".into(),
            citations: vec![],
            considered_sources: vec![],
            tests: vec![],
            automated_verdict: None,
            verdict_basis: Vec::new(),
            evidence: Vec::new(),
            confidence_calibration_version: None,
            review_required: false,
            review_reason: None,
            verified_status: None,
            review_events: Vec::new(),
        }
    }

    fn sample_bundle() -> AuditBundle {
        let mut bundle =
            AuditBundle::new("audit-1", "https://example.test", AuditConfig::default());
        bundle.summary.total_pages = 1;
        bundle.summary.completed_pages = 1;
        bundle.summary.total_findings = 2;
        bundle.summary.passed = 10;
        bundle.summary.failed = 2;
        bundle.summary.needs_review = 1;
        bundle.summary.errors = 0;

        let mut finding = Finding::new("finding-1");
        finding.rule = "image-alt".into();
        finding.criterion_id = Some("1.1".into());
        finding.url = "https://example.test".into();
        finding.target = "#main img".into();
        finding.status = CriterionStatus::Fail;
        finding.severity = Some("critical".into());
        finding.description = Some("Missing alt text".into());
        bundle.findings.push(finding);

        let mut finding2 = Finding::new("finding-2");
        finding2.rule = "color-contrast".into();
        finding2.criterion_id = Some("1.3".into());
        finding2.url = "https://example.test".into();
        finding2.target = "#content".into();
        finding2.status = CriterionStatus::NeedsReview;
        finding2.severity = Some("serious".into());
        finding2.description = Some("Low contrast detected".into());
        bundle.findings.push(finding2);

        bundle
    }

    /// A probe `review` carries its reason in `justification` (#262); the per-page
    /// criteria table must show it so the reader learns why the criterion is open.
    #[test]
    fn needs_review_reason_reaches_the_criteria_table() {
        let mut bundle =
            AuditBundle::new("audit-3", "https://example.test", AuditConfig::default());
        let mut review = criterion("12.9", CriterionStatus::NeedsReview);
        review.justification = Some("gap-fix review: moteur absent: 3 focusable".into());
        bundle.pages.push(PageAudit {
            page_id: "p1".into(),
            url: "https://example.test".into(),
            title: None,
            criteria: vec![review],
            findings: vec![],
            errors: vec![],
            completed: true,
            duration_ms: 0,
        });
        let html = generate_html_report(&bundle);
        assert!(html.contains("moteur absent"));
    }

    #[test]
    fn html_report_contains_audit_id() {
        let bundle = sample_bundle();
        let html = generate_html_report(&bundle);
        assert!(html.contains("audit-1"));
    }

    #[test]
    fn html_report_contains_findings_count() {
        let bundle = sample_bundle();
        let html = generate_html_report(&bundle);
        assert!(html.contains("Problèmes Détectés (2)"));
    }

    #[test]
    fn html_report_escapes_html_in_description() {
        let mut bundle = sample_bundle();
        bundle.findings[0].description = Some("<script>alert('xss')</script>".into());
        let html = generate_html_report(&bundle);
        assert!(html.contains("&lt;script&gt;"));
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn incomplete_page_is_not_conforme_even_without_failures() {
        let mut bundle =
            AuditBundle::new("audit-2", "https://example.test", AuditConfig::default());
        bundle.summary.failed = 0;
        bundle.summary.errors = 0;
        bundle.pages.push(PageAudit {
            page_id: "page-0".into(),
            url: "https://example.test".into(),
            title: None,
            // Only one of the 106 catalog criteria — an incomplete run.
            criteria: vec![criterion("1.1", CriterionStatus::Pass)],
            findings: vec![],
            errors: vec![],
            completed: true,
            duration_ms: 0,
        });

        let html = generate_html_report(&bundle);
        assert!(html.contains("Non Conforme"));
    }

    #[test]
    fn complete_criteria_preserves_results_outside_the_catalog() {
        let results = vec![criterion("not-a-real-id", CriterionStatus::Fail)];
        let completed = complete_criteria(&results);

        assert_eq!(completed.len(), RgaaCriteria::all().len() + 1);
        assert!(completed
            .iter()
            .any(|c| c.criterion_id == "not-a-real-id" && c.status == CriterionStatus::Fail));
    }

    #[test]
    fn report_separates_automatic_estimate_from_verified_review_and_shows_metrics() {
        let mut bundle = AuditBundle::new(
            "audit-assessment",
            "https://example.test",
            AuditConfig::default(),
        );
        let mut item = criterion("1.1", CriterionStatus::NeedsReview);
        item.automated_verdict = Some(AutomatedVerdict::Pass);
        item.verdict_basis = vec![VerdictBasis::ModelEstimate];
        item.raw_confidence = Some(0.82);
        item.confidence = Some(0.75);
        item.review_required = true;
        item.review_reason = Some("Vérifier l’équivalence".into());
        item.evidence.push(EvidenceRef::new("dom", "sha256:test"));
        item.verified_status = Some(CriterionStatus::Fail);
        item.review_events.push(ReviewEvent {
            status: CriterionStatus::Fail,
            author: "auditrice".into(),
            reviewed_at: "2026-10-07T12:00:00Z".into(),
            reason: "Alternative incomplète".into(),
        });
        bundle.pages.push(PageAudit {
            page_id: "p1".into(),
            url: "https://example.test".into(),
            title: None,
            criteria: vec![item],
            findings: vec![],
            errors: vec![],
            completed: true,
            duration_ms: 0,
        });

        let html = generate_html_report(&bundle);
        for label in [
            "Couverture des verdicts automatiques",
            "Couverture des tests avec preuve",
            "Conformité vérifiée",
            "Verdict automatique",
            "Statut vérifié",
            "Confiance brute",
            "Confiance calibrée",
            "Revue humaine requise",
            "estimation",
            "Alternative incomplète",
            "sha256:test",
            "auditrice",
        ] {
            assert!(html.contains(label), "missing {label}");
        }
        assert!(!html.contains("<h3>Couverture</h3>"));
    }
}
