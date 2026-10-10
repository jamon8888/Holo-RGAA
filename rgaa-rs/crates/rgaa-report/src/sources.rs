//! Rendering of the RAG evidence behind a criterion verdict.
//!
//! [`CriterionResult::citations`] has existed since the dual-router work,
//! and the evaluator populates it whenever a verdict leans on retrieved
//! documents — but no renderer ever read it. Every format dropped the
//! citations on the floor, so a verdict that *was* sourced arrived at the
//! reader indistinguishable from one reached by guesswork. That is the
//! opposability problem: an auditor cannot defend "Fail on 1.1" without
//! naming what the machine read to get there.
//!
//! This module is the single place that turns a [`Citation`] into text, so
//! Markdown and HTML cannot drift into describing the same source two
//! different ways.
//!
//! [`CriterionResult::citations`]: rgaa_core::CriterionResult::citations

use rgaa_core::{Citation, CriterionResult};

/// One citation as a single human-readable line.
///
/// Deliberately lossless for the fields that make a citation checkable
/// later: the référentiel version (a verdict is only valid against the
/// version it was checked against) and the evidence hash (the crawl index
/// is purged, the hash is what survives to prove what was retrieved).
pub fn format_citation(citation: &Citation) -> String {
    match citation {
        Citation::Referentiel {
            test_id,
            referentiel_version,
        } => format!("référentiel RGAA test {test_id} (version {referentiel_version})"),
        Citation::Crawl {
            url,
            captured_at,
            evidence_hash,
        } => format!("crawl {url} (capturé {captured_at}, {evidence_hash})"),
    }
}

/// The `Sources` line for a criterion, or `None` when the verdict was
/// reached without retrieval.
///
/// Returning `None` rather than an empty string is the point: a
/// deterministic pass, a manual review and a "not tested" all legitimately
/// have no sources, and printing an empty `Sources:` next to them would
/// read as missing evidence rather than as evidence not being applicable.
pub fn sources_line(result: &CriterionResult) -> Option<String> {
    if result.citations.is_empty() {
        return None;
    }
    let rendered: Vec<String> = result.citations.iter().map(format_citation).collect();
    Some(rendered.join(" ; "))
}

/// Every criterion in the bundle that carries citations, in the order the
/// pages and their criteria appear.
pub fn sourced_criteria(bundle: &rgaa_core::AuditBundle) -> Vec<(&str, &CriterionResult)> {
    let mut out = Vec::new();
    for page in &bundle.pages {
        for criterion in &page.criteria {
            if !criterion.citations.is_empty() {
                out.push((page.url.as_str(), criterion));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgaa_core::{Classification, CriterionStatus};

    fn result_with(citations: Vec<Citation>) -> CriterionResult {
        CriterionResult {
            criterion_id: "1.1".into(),
            title: "Image porteuse d'information".into(),
            classification: Classification::IaAssiste,
            status: CriterionStatus::Fail,
            violations: Vec::new(),
            confidence: Some(0.9),
            justification: Some("alt manquant".into()),
            source: "holo3".into(),
            citations,
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
        }
    }

    #[test]
    fn a_referentiel_citation_names_the_version_it_was_checked_against() {
        // Without the version, a citation cannot be re-checked: the corpus
        // is rebuilt wholesale on a version bump.
        let line = format_citation(&Citation::referentiel("1.1.1", "2024.1"));
        assert!(line.contains("1.1.1"), "{line}");
        assert!(line.contains("2024.1"), "{line}");
    }

    #[test]
    fn a_crawl_citation_keeps_the_evidence_hash() {
        // The crawl index is purged after the audit; the hash is the only
        // thing left that proves what was actually retrieved.
        let line = format_citation(&Citation::crawl(
            "https://example.org/contact",
            "2025-01-01T00:00:00Z",
            "sha256:abc",
        ));
        assert!(line.contains("https://example.org/contact"), "{line}");
        assert!(line.contains("sha256:abc"), "{line}");
    }

    #[test]
    fn an_unsourced_verdict_has_no_sources_line() {
        assert_eq!(sources_line(&result_with(Vec::new())), None);
    }

    #[test]
    fn several_citations_are_joined_into_one_line() {
        let line = sources_line(&result_with(vec![
            Citation::referentiel("1.1.1", "2024.1"),
            Citation::crawl("https://example.org/", "2025-01-01T00:00:00Z", "sha256:x"),
        ]))
        .expect("sourced");
        assert!(line.contains("1.1.1"), "{line}");
        assert!(line.contains("https://example.org/"), "{line}");
        assert!(line.contains(" ; "), "{line}");
    }
}
