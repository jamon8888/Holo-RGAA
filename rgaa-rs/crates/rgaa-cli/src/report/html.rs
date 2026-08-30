use std::fmt::Write;

use rgaa_core::{AuditBundle, AuditSummary, CriterionStatus, Finding};

pub fn generate_html_report(bundle: &AuditBundle) -> String {
    let mut html = String::new();
    write_html_header(&mut html, &bundle.audit_id, &bundle.url);
    write_html_summary(&mut html, bundle);
    write_html_stats(&mut html, &bundle.summary);
    write_html_findings(&mut html, bundle);
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
    let conformity_badge_class = match bundle.summary.failed {
        0 => "pass",
        _ => "fail",
    };
    let conformity_text = match bundle.summary.failed {
        0 => "Conforme",
        _ => "Non Conforme",
    };

    let _ = writeln!(
        html,
        r#"        <div class="summary">
            <div class="card">
                <h3>Taux de Conformité</h3>
                <div class="value {}">{}%</div>
            </div>
            <div class="card">
                <h3>État de Conformité</h3>
                <div class="value"><span class="status-badge {}">{}</span></div>
            </div>
            <div class="card">
                <h3>Couverture</h3>
                <div class="value neutral">{}%</div>
            </div>
            <div class="card">
                <h3>Pages Auditées</h3>
                <div class="value neutral">{}/{}</div>
            </div>
        </div>"#,
        conformity_badge_class,
        calculate_conformity_rate(&bundle.summary),
        conformity_badge_class,
        conformity_text,
        calculate_coverage(&bundle.summary),
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
                    <th>Statut</th>
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
        summary.passed,
        summary.failed,
        summary.needs_review,
        summary.passed + summary.failed + summary.needs_review,
        summary.errors
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

fn calculate_conformity_rate(summary: &AuditSummary) -> f64 {
    let total = summary.passed + summary.failed + summary.needs_review;
    if total == 0 {
        return 0.0;
    }
    (summary.passed as f64 / total as f64) * 100.0
}

fn calculate_coverage(summary: &AuditSummary) -> f64 {
    if summary.total_findings == 0 {
        return 100.0;
    }
    let tested = summary.passed + summary.failed + summary.needs_review;
    (tested as f64 / summary.total_findings as f64).min(1.0) * 100.0
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
    use rgaa_core::AuditConfig;

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
}
