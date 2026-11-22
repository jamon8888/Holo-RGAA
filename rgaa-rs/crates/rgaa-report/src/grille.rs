//! CSV export of the audit grid in the Ara (DINUM) format.
//!
//! Reference: `DISIC/Ara`, `confiture-rest-api/src/audits/audit-export.service.ts`
//! (commit `b3ece0a`). The header row is `Critères`, `Éléments transverses`,
//! then one column per page; each row is a criterion id followed by its
//! status on the transverse page and on every audited page. Ara encodes the
//! four statuses as `C`, `NC`, `NA` and `NT`.

use std::collections::HashMap;

use rgaa_core::{AuditResult, CriterionResult, CriterionStatus, PageResult, RgaaCriteria};

/// Status letter for a human decision that the criterion conforms.
const COMPLIANT: &str = "C";
/// Status letter for a human decision that the criterion does not conform.
const NOT_COMPLIANT: &str = "NC";
/// Status letter for a human decision that the criterion does not apply.
const NOT_APPLICABLE: &str = "NA";
/// Status letter for everything that is not a human decision.
const NOT_TESTED: &str = "NT";

/// Renders an audit as the criteria × pages CSV grid used by Ara.
///
/// Rows follow the official RGAA criterion order from [`RgaaCriteria::all`],
/// including criteria the audit never evaluated (they read `NT`). Pages keep
/// their audit order. Only human decisions populate the status cells.
#[must_use]
pub fn render_grille_csv(audit: &AuditResult) -> String {
    let mut output = String::with_capacity(64 + audit.pages.len() * 106 * 4);
    let mut row: Vec<String> = Vec::with_capacity(audit.pages.len() + 2);

    row.push("Critères".to_string());
    row.push("Éléments transverses".to_string());
    for page in &audit.pages {
        row.push(page_column_name(page));
    }
    write_row(&mut output, &row);

    // One index per page; duplicate results for a criterion keep the first one.
    let by_page: Vec<HashMap<&str, &CriterionResult>> = audit
        .pages
        .iter()
        .map(|page| {
            let mut index = HashMap::with_capacity(page.criteria.len());
            for criterion in &page.criteria {
                index
                    .entry(criterion.criterion_id.as_str())
                    .or_insert(criterion);
            }
            index
        })
        .collect();

    row.clear();
    for criterion in RgaaCriteria::all() {
        row.push(criterion.id.to_string());
        // Transverse elements do not exist yet (#429): the column is present
        // but every cell stays NT until they are modelled and reviewed.
        row.push(NOT_TESTED.to_string());
        for page in &by_page {
            row.push(decision_letter(page.get(criterion.id).copied()).to_string());
        }
        write_row(&mut output, &row);
        row.clear();
    }

    output
}

/// Column name for one page: its title when it has one, its URL otherwise.
#[must_use]
fn page_column_name(page: &PageResult) -> String {
    page.title
        .as_deref()
        .filter(|title| !title.trim().is_empty())
        .unwrap_or(&page.url)
        .to_string()
}

/// Writes one RFC 4180 record: commas between fields, quotes doubled, LF ending.
fn write_row(output: &mut String, fields: &[String]) {
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        if field
            .bytes()
            .any(|byte| matches!(byte, b',' | b'"' | b'\r' | b'\n'))
        {
            output.push('"');
            for character in field.chars() {
                if character == '"' {
                    output.push('"');
                }
                output.push(character);
            }
            output.push('"');
        } else {
            output.push_str(field);
        }
    }
    output.push('\n');
}

/// Translates one cell to an Ara status letter.
///
/// Only `CriterionResult::verified_status` is a human decision (R7: no tool
/// writes a status). A machine verdict in `status` — including an error — is
/// never promoted to C/NC/NA.
#[must_use]
fn decision_letter(result: Option<&CriterionResult>) -> &'static str {
    match result.and_then(|criterion| criterion.verified_status.as_ref()) {
        Some(CriterionStatus::Pass) => COMPLIANT,
        Some(CriterionStatus::Fail) => NOT_COMPLIANT,
        Some(CriterionStatus::NotApplicable) => NOT_APPLICABLE,
        _ => NOT_TESTED,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgaa_core::{Classification, PageResult, RgaaCriteria};

    fn result(
        id: &str,
        status: CriterionStatus,
        verified: Option<CriterionStatus>,
    ) -> CriterionResult {
        CriterionResult {
            criterion_id: id.into(),
            title: "titre de test".into(),
            classification: Classification::Deterministe,
            status,
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
            verified_status: verified,
            review_events: Vec::new(),
        }
    }

    fn page(url: &str, title: Option<&str>, criteria: Vec<CriterionResult>) -> PageResult {
        PageResult {
            url: url.into(),
            title: title.map(str::to_string),
            criteria,
            compliance_rate: 0.0,
            crawl_depth: 0,
        }
    }

    fn audit(pages: Vec<PageResult>) -> AuditResult {
        AuditResult {
            audit_id: "audit-test".into(),
            url: "https://example.test".into(),
            pages,
            total_criteria: 106,
            passed: 0,
            failed: 0,
            na: 0,
            overall_compliance: 0.0,
            taux_global: 0.0,
            coverage_percent: 0.0,
            automatic_verdict_coverage_percent: 0.0,
            test_evidence_coverage_percent: 0.0,
            verified_compliance_percent: 0.0,
            etat_conformite: "non conforme".into(),
            duration_ms: 0,
            audit_complete: false,
        }
    }

    /// The Ara reference fixes the header names and the four status letters.
    /// An audit with no results is still a full 106-row grid, all NT.
    #[test]
    fn header_and_empty_grid_follow_ara_reference() {
        assert_eq!(RgaaCriteria::all().len(), 106);
        let csv = render_grille_csv(&audit(vec![page("https://example.test/", None, vec![])]));
        let lines: Vec<&str> = csv.lines().collect();

        assert_eq!(
            lines.first().copied(),
            Some("Critères,Éléments transverses,https://example.test/")
        );
        assert_eq!(lines.len(), 107, "one header + the 106 RGAA criteria");
        assert_eq!(lines[1], "1.1,NT,NT");
        assert_eq!(lines[106], "13.12,NT,NT");
    }

    /// Reference: only a human decision yields C/NC/NA. A machine status,
    /// even a deterministic axe-core `pass` or an `error`, yields NT.
    /// Duplicate results for one criterion on one page: the first one wins.
    #[test]
    fn only_human_decisions_give_c_nc_na() {
        let criteria = vec![
            result("1.1", CriterionStatus::Fail, Some(CriterionStatus::Pass)),
            result("1.2", CriterionStatus::Pass, None),
            result(
                "1.3",
                CriterionStatus::NeedsReview,
                Some(CriterionStatus::Fail),
            ),
            result(
                "1.4",
                CriterionStatus::Pass,
                Some(CriterionStatus::NotApplicable),
            ),
            result("1.5", CriterionStatus::Error, None),
            result(
                "1.6",
                CriterionStatus::NeedsReview,
                Some(CriterionStatus::Pass),
            ),
            result(
                "1.1",
                CriterionStatus::NotApplicable,
                Some(CriterionStatus::Fail),
            ),
        ];
        let csv = render_grille_csv(&audit(vec![page("https://example.test/", None, criteria)]));
        let lines: Vec<&str> = csv.lines().collect();

        let expected = [
            "1.1,NT,C",
            "1.2,NT,NT",
            "1.3,NT,NC",
            "1.4,NT,NA",
            "1.5,NT,NT",
            "1.6,NT,C",
        ];
        assert_eq!(&lines[1..7], expected.as_slice());
    }

    /// Page names come from the title, falling back to the URL, and must be
    /// escaped the way RFC 4180 / the xlsx CSV writer do it (doubled quotes).
    #[test]
    fn page_names_are_escaped_and_keep_column_order() {
        let csv = render_grille_csv(&audit(vec![
            page("https://example.test/", Some("Accueil, \"rapide\""), vec![]),
            page("https://example.test/contact", None, vec![]),
        ]));
        let lines: Vec<&str> = csv.lines().collect();

        assert_eq!(
            lines.first().copied(),
            Some("Critères,Éléments transverses,\"Accueil, \"\"rapide\"\"\",https://example.test/contact")
        );
        assert_eq!(lines[1], "1.1,NT,NT,NT");
    }

    /// The transverse column is always present and NT until #429 lands.
    #[test]
    fn transverse_column_is_always_nt_until_it_exists() {
        let csv = render_grille_csv(&audit(vec![page(
            "https://example.test/",
            None,
            vec![result(
                "1.1",
                CriterionStatus::Fail,
                Some(CriterionStatus::Pass),
            )],
        )]));
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[1], "1.1,NT,C");
    }
}
