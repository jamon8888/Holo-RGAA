//! The "Références" section of an evaluation prompt: the retrieved
//! documents an evaluator is grounded on, rendered in a fixed order
//! (regulatory corpus first, then crawl evidence) with bounded size.
//!
//! Deliberately independent of `rgaa_agent::rag` (which is gated behind the
//! `vector-store` feature and talks to LanceDB) — this module only knows how
//! to render plain document structs, so [`crate::prompts::PromptBuilder`]
//! stays buildable without the heavy vector-store dependency chain. The
//! router (#126) is what converts `rag::ReferentielDocument`/
//! `rag::CrawlDocument` into [`ReferentielReference`]/[`CrawlReference`].

/// One document from the versioned regulatory corpus, ready to render.
#[derive(Debug, Clone, PartialEq)]
pub struct ReferentielReference {
    pub test_id: String,
    pub referentiel_version: String,
    pub content: String,
}

/// One document from the per-audit crawl index, ready to render.
#[derive(Debug, Clone, PartialEq)]
pub struct CrawlReference {
    pub url: String,
    pub content: String,
}

/// The retrieved documents to ground a single criterion evaluation on.
/// Order between the two lists is not significant here — [`render`]
/// always emits the regulatory corpus before the crawl evidence,
/// regardless of the order documents are passed in, per the fixed
/// referentiel-then-crawl ordering the spec requires.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct References {
    pub referentiel: Vec<ReferentielReference>,
    pub crawl: Vec<CrawlReference>,
}

impl References {
    pub fn is_empty(&self) -> bool {
        self.referentiel.is_empty() && self.crawl.is_empty()
    }
}

/// Hard cap, in bytes, on one document's rendered content within the
/// references section. Keeps one huge retrieved document from crowding out
/// every other reference (and the rest of the prompt).
pub const MAX_DOC_CHARS: usize = 800;

/// Hard cap, in bytes, on the whole rendered references section.
pub const MAX_SECTION_CHARS: usize = 4_000;

const DOC_TRUNCATION_MARKER: &str = " […]";
const SECTION_TRUNCATION_MARKER: &str = "\n\n[…références truncated…]";

/// Renders `references` as a "## Références" prompt section: regulatory
/// corpus first, then crawl evidence, each document capped at
/// [`MAX_DOC_CHARS`] and the whole section capped at [`MAX_SECTION_CHARS`].
///
/// Returns an empty string when `references` is empty, so a caller that
/// appends this to a prompt gets byte-identical output to not calling it at
/// all — no empty "## Références" header for a criterion with no retrieval.
pub fn render(references: &References) -> String {
    if references.is_empty() {
        return String::new();
    }

    let mut section = String::from("\n\n## Références\n");

    if !references.referentiel.is_empty() {
        section.push_str("\n### Référentiel\n\n");
        for doc in &references.referentiel {
            section.push_str(&format!(
                "- [{} v{}] {}\n",
                doc.test_id,
                doc.referentiel_version,
                cap(&doc.content, MAX_DOC_CHARS)
            ));
        }
    }

    if !references.crawl.is_empty() {
        section.push_str("\n### Crawl\n\n");
        for doc in &references.crawl {
            section.push_str(&format!(
                "- [{}] {}\n",
                doc.url,
                cap(&doc.content, MAX_DOC_CHARS)
            ));
        }
    }

    cap_section(section)
}

/// Truncates `text` to at most `max_chars` bytes at a char boundary,
/// appending [`DOC_TRUNCATION_MARKER`] when truncated.
fn cap(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        return text.to_string();
    }
    let mut end = max_chars;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    let mut truncated = text[..end].to_string();
    truncated.push_str(DOC_TRUNCATION_MARKER);
    truncated
}

/// Truncates the whole rendered section to [`MAX_SECTION_CHARS`], appending
/// [`SECTION_TRUNCATION_MARKER`] when truncated.
fn cap_section(section: String) -> String {
    if section.len() <= MAX_SECTION_CHARS {
        return section;
    }
    let mut end = MAX_SECTION_CHARS;
    while end > 0 && !section.is_char_boundary(end) {
        end -= 1;
    }
    let mut truncated = section[..end].to_string();
    truncated.push_str(SECTION_TRUNCATION_MARKER);
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_references() -> References {
        References {
            referentiel: vec![ReferentielReference {
                test_id: "1.1.1".into(),
                referentiel_version: "2024.1".into(),
                content: "Chaque image porteuse d'information a une alternative textuelle.".into(),
            }],
            crawl: vec![CrawlReference {
                url: "https://example.test/contact".into(),
                content: "Formulaire de contact sans label visible sur le champ email.".into(),
            }],
        }
    }

    #[test]
    fn empty_references_render_to_empty_string() {
        assert_eq!(render(&References::default()), "");
    }

    #[test]
    fn rendered_section_matches_expected_snapshot() {
        let rendered = render(&sample_references());
        insta::assert_snapshot!(rendered, @r###"

        ## Références

        ### Référentiel

        - [1.1.1 v2024.1] Chaque image porteuse d'information a une alternative textuelle.

        ### Crawl

        - [https://example.test/contact] Formulaire de contact sans label visible sur le champ email.
        "###);
    }

    #[test]
    fn referentiel_always_precedes_crawl_regardless_of_input_order() {
        // References passed with crawl "first" in the struct still render
        // referentiel before crawl — the order is fixed by `render`, not by
        // caller-supplied order.
        let refs = sample_references();
        let rendered = render(&refs);
        let referentiel_pos = rendered.find("### Référentiel").unwrap();
        let crawl_pos = rendered.find("### Crawl").unwrap();
        assert!(referentiel_pos < crawl_pos);
    }

    #[test]
    fn referentiel_only_omits_crawl_heading() {
        let refs = References {
            referentiel: sample_references().referentiel,
            crawl: vec![],
        };
        let rendered = render(&refs);
        assert!(rendered.contains("### Référentiel"));
        assert!(!rendered.contains("### Crawl"));
    }

    #[test]
    fn long_document_is_capped_at_max_doc_chars() {
        let refs = References {
            referentiel: vec![ReferentielReference {
                test_id: "1.1.1".into(),
                referentiel_version: "2024.1".into(),
                content: "x".repeat(10_000),
            }],
            crawl: vec![],
        };
        let rendered = render(&refs);
        assert!(rendered.contains(DOC_TRUNCATION_MARKER));
        assert!(rendered.len() <= MAX_SECTION_CHARS + SECTION_TRUNCATION_MARKER.len());
    }

    #[test]
    fn many_documents_cap_whole_section() {
        let referentiel: Vec<_> = (0..50)
            .map(|i| ReferentielReference {
                test_id: format!("1.1.{i}"),
                referentiel_version: "2024.1".into(),
                content: "y".repeat(500),
            })
            .collect();
        let refs = References {
            referentiel,
            crawl: vec![],
        };
        let rendered = render(&refs);
        assert!(rendered.len() <= MAX_SECTION_CHARS + SECTION_TRUNCATION_MARKER.len());
        assert!(rendered.contains(SECTION_TRUNCATION_MARKER));
    }
}
