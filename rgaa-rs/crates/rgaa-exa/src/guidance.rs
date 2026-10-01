//! The audit-facing layer: turn a failing RGAA criterion into a grounded set
//! of remediation references retrieved from the live web.
//!
//! The vector store in `rgaa-agent` grounds an evaluation on the *frozen*
//! regulatory corpus. This module covers the other half: current, external
//! guidance (WCAG techniques, ARIA authoring practices, vendor notes) for a
//! criterion that has already failed, so a remediation proposal cites
//! something checkable instead of model recall.

use crate::client::ExaClient;
use crate::error::Result;
use crate::search::{SearchRequest, SearchResult};
use rgaa_core::Criterion;

/// How many references one criterion is grounded on.
///
/// A deliberate product decision, not boilerplate: the prompt's "Références"
/// section is byte-capped (`rgaa_agent::references::MAX_SECTION_CHARS`), so
/// asking for the server default of 10 would only pay for excerpts that get
/// truncated away.
pub const REFERENCES_PER_CRITERION: u32 = 5;

/// Hard cap on one reference's rendered excerpt, mirroring
/// `rgaa_agent::references::MAX_DOC_CHARS`.
pub const MAX_EXCERPT_CHARS: usize = 800;

/// One piece of external guidance, ready to render into a prompt or a report.
#[derive(Debug, Clone, PartialEq)]
pub struct GuidanceReference {
    pub title: String,
    pub url: String,
    pub published_date: Option<String>,
    /// Exa highlights joined and capped at [`MAX_EXCERPT_CHARS`].
    pub excerpt: String,
}

impl GuidanceReference {
    /// Converts an Exa result, dropping anything with no extracted content —
    /// a bare title and URL grounds nothing.
    #[must_use]
    pub fn from_result(result: &SearchResult) -> Option<Self> {
        let excerpt = cap(&result.highlights.join(" … "), MAX_EXCERPT_CHARS);
        if excerpt.trim().is_empty() {
            return None;
        }
        Some(Self {
            title: result.title.clone().unwrap_or_else(|| result.url.clone()),
            url: result.url.clone(),
            published_date: result.published_date.clone(),
            excerpt,
        })
    }
}

/// Builds the retrieval queries this crate sends. Stateless, per the
/// workspace's unit-struct convention.
pub struct GuidanceQuery;

impl GuidanceQuery {
    /// Query for remediating a failing criterion.
    ///
    /// Everything that steers retrieval is phrased in natural language:
    /// recency ("à jour"), source character ("documentation officielle"), and
    /// the WCAG success criteria behind the RGAA criterion. No `category`, no
    /// domain allowlist — those are hard filters that would silently drop
    /// good pages, and Exa's own guidance is to express preferences in the
    /// query text instead.
    #[must_use]
    pub fn remediation(criterion: &Criterion, failure_context: Option<&str>) -> String {
        let mut q = format!(
            "Comment corriger le critère RGAA {} « {} » (WCAG {}) : documentation officielle et techniques de remédiation à jour",
            criterion.id, criterion.title, criterion.wcag_refs
        );
        if let Some(ctx) = failure_context.map(str::trim).filter(|c| !c.is_empty()) {
            q.push_str(". Cas observé : ");
            q.push_str(&cap(ctx, 300));
        }
        q
    }

    /// Query for the criterion's own test methodology, for an auditor who
    /// needs to justify a verdict rather than fix it.
    #[must_use]
    pub fn methodology(criterion: &Criterion) -> String {
        format!(
            "Méthodologie de test du critère RGAA {} « {} » (WCAG {}) : tests, cas particuliers et conditions de conformité",
            criterion.id, criterion.title, criterion.wcag_refs
        )
    }
}

/// Retrieves external remediation guidance for a failing criterion.
///
/// # Errors
/// Propagates any [`crate::ExaError`] from the underlying search.
#[tracing::instrument(name = "exa.remediation_guidance", skip(client, criterion), fields(criterion = %criterion.id))]
pub async fn remediation_guidance(
    client: &ExaClient,
    criterion: &Criterion,
    failure_context: Option<&str>,
) -> Result<Vec<GuidanceReference>> {
    let query = GuidanceQuery::remediation(criterion, failure_context);
    let request = SearchRequest::new(query).with_num_results(REFERENCES_PER_CRITERION);
    let response = client.search(&request).await?;
    Ok(response
        .results
        .iter()
        .filter_map(GuidanceReference::from_result)
        .collect())
}

/// Retrieves test-methodology guidance for a criterion.
///
/// # Errors
/// Propagates any [`crate::ExaError`] from the underlying search.
#[tracing::instrument(name = "exa.methodology_guidance", skip(client, criterion), fields(criterion = %criterion.id))]
pub async fn methodology_guidance(
    client: &ExaClient,
    criterion: &Criterion,
) -> Result<Vec<GuidanceReference>> {
    let request = SearchRequest::new(GuidanceQuery::methodology(criterion))
        .with_num_results(REFERENCES_PER_CRITERION);
    let response = client.search(&request).await?;
    Ok(response
        .results
        .iter()
        .filter_map(GuidanceReference::from_result)
        .collect())
}

/// Renders references as a prompt section, mirroring the layout
/// `rgaa_agent::references::render` uses for the regulatory corpus.
/// Returns an empty string for an empty slice, so appending it is a no-op.
#[must_use]
pub fn render(references: &[GuidanceReference]) -> String {
    if references.is_empty() {
        return String::new();
    }
    let mut section = String::from("\n\n### Sources externes (Exa)\n\n");
    for r in references {
        let date = r
            .published_date
            .as_deref()
            .map(|d| format!(" ({d})"))
            .unwrap_or_default();
        section.push_str(&format!(
            "- [{}]({}){} — {}\n",
            r.title, r.url, date, r.excerpt
        ));
    }
    section
}

fn cap(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{} […]", &s[..end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgaa_core::types::Classification;

    fn criterion() -> Criterion {
        Criterion {
            id: "1.3",
            title: "Image porteuse d'information : alternative pertinente ?".to_string(),
            classification: Classification::IaAssiste,
            wcag_refs: "1.1.1, 4.1.2",
        }
    }

    #[test]
    fn remediation_query_carries_id_title_and_wcag() {
        let q = GuidanceQuery::remediation(&criterion(), None);
        assert!(q.contains("RGAA 1.3"), "{q}");
        assert!(q.contains("Image porteuse d'information"), "{q}");
        assert!(q.contains("WCAG 1.1.1, 4.1.2"), "{q}");
    }

    #[test]
    fn remediation_query_has_no_keep_drop_clauses() {
        // Keep/drop rules belong in systemPrompt, never in the query text.
        let q = GuidanceQuery::remediation(&criterion(), None).to_lowercase();
        for banned in ["uniquement", "exclure", "only", "exclude"] {
            assert!(!q.contains(banned), "query steers filtering: {q}");
        }
    }

    #[test]
    fn failure_context_is_appended_and_capped() {
        let ctx = "x".repeat(400);
        let q = GuidanceQuery::remediation(&criterion(), Some(&ctx));
        assert!(q.contains("Cas observé :"));
        assert!(q.contains("[…]"), "long context must be capped");
    }

    #[test]
    fn blank_failure_context_is_ignored() {
        let q = GuidanceQuery::remediation(&criterion(), Some("   "));
        assert!(!q.contains("Cas observé"), "{q}");
    }

    #[test]
    fn methodology_query_differs_from_remediation() {
        let c = criterion();
        assert_ne!(
            GuidanceQuery::methodology(&c),
            GuidanceQuery::remediation(&c, None)
        );
    }

    #[test]
    fn result_without_highlights_is_dropped() {
        let r = SearchResult {
            url: "https://example.org".to_string(),
            ..Default::default()
        };
        assert!(GuidanceReference::from_result(&r).is_none());
    }

    #[test]
    fn reference_falls_back_to_url_when_untitled() {
        let r = SearchResult {
            url: "https://example.org".to_string(),
            highlights: vec!["contenu".to_string()],
            ..Default::default()
        };
        let reference = GuidanceReference::from_result(&r).unwrap();
        assert_eq!(reference.title, "https://example.org");
        assert_eq!(reference.excerpt, "contenu");
    }

    #[test]
    fn multiple_highlights_are_joined_and_capped() {
        let r = SearchResult {
            url: "https://example.org".to_string(),
            highlights: vec!["a".repeat(500), "b".repeat(500)],
            ..Default::default()
        };
        let reference = GuidanceReference::from_result(&r).unwrap();
        assert!(reference.excerpt.len() <= MAX_EXCERPT_CHARS + 6);
        assert!(reference.excerpt.ends_with("[…]"));
    }

    #[test]
    fn render_is_empty_for_no_references() {
        assert_eq!(render(&[]), "");
    }

    #[test]
    fn render_lists_every_reference_with_its_url() {
        let refs = vec![GuidanceReference {
            title: "Technique H37".to_string(),
            url: "https://www.w3.org/WAI/WCAG21/Techniques/html/H37".to_string(),
            published_date: Some("2023-05-01".to_string()),
            excerpt: "Utiliser l'attribut alt.".to_string(),
        }];
        let out = render(&refs);
        assert!(out.contains("Sources externes (Exa)"));
        assert!(out.contains("https://www.w3.org/WAI/WCAG21/Techniques/html/H37"));
        assert!(out.contains("(2023-05-01)"));
    }
}
