use crate::criteria_defs::get_criterion_definition;
use crate::references::{self, References};
use rgaa_holo::{format_page_context, PageContext};

/// Hard cap, in bytes, on the rendered page-context section of a prompt.
/// Keeps prompts bounded even for pages with huge amounts of extracted
/// content (many headings/images/forms) — a `ponytail:` ceiling, not a
/// content-quality decision.
pub const MAX_CONTEXT_CHARS: usize = 8_000;

const TRUNCATION_MARKER: &str = "\n\n[…page context truncated…]";

/// Builds structured evaluation prompts for Holo3.
///
/// The prompt includes the criterion definition, WCAG references,
/// and the page context (headings, images, forms, etc.).
pub struct PromptBuilder;

impl PromptBuilder {
    /// Renders `context` to text and caps it at [`MAX_CONTEXT_CHARS`].
    ///
    /// Call this once per URL and reuse the result across every criterion's
    /// prompt via [`Self::build_from_rendered`] — rendering is not free for
    /// pages with many elements, and the audit pipeline evaluates up to
    /// ~20+ criteria against the same page context.
    pub fn render_context(context: &PageContext) -> String {
        let rendered = format_page_context(context);
        if rendered.len() <= MAX_CONTEXT_CHARS {
            return rendered;
        }
        let mut end = MAX_CONTEXT_CHARS;
        while end > 0 && !rendered.is_char_boundary(end) {
            end -= 1;
        }
        let mut truncated = rendered[..end].to_string();
        truncated.push_str(TRUNCATION_MARKER);
        truncated
    }

    /// Builds a text-only evaluation prompt for `criterion_id`, rendering
    /// `context` fresh. Prefer [`Self::build_from_rendered`] with a context
    /// already rendered via [`Self::render_context`] when evaluating several
    /// criteria against the same page.
    ///
    /// # Returns
    /// A formatted prompt string ready to send to the Holo3 API.
    pub fn build(criterion_id: &str, context: &PageContext) -> String {
        Self::build_from_rendered(criterion_id, &Self::render_context(context))
    }

    /// Builds a text-only evaluation prompt for `criterion_id` from an
    /// already-rendered (and capped) page context — see
    /// [`Self::render_context`]. No retrieved-document references section.
    pub fn build_from_rendered(criterion_id: &str, rendered_context: &str) -> String {
        Self::build_from_rendered_with_references(
            criterion_id,
            rendered_context,
            &References::default(),
        )
    }

    /// As [`Self::build_from_rendered`], plus a "## Références" section
    /// grounding the evaluation in retrieved documents (regulatory corpus
    /// first, then crawl evidence — see [`crate::references::render`]).
    ///
    /// Passing an empty [`References`] (as [`Self::build_from_rendered`]
    /// does) produces byte-identical output to not calling this at all —
    /// [`references::render`] returns an empty string for an empty
    /// [`References`], so no criterion without retrieval gets an empty
    /// "## Références" header.
    pub fn build_from_rendered_with_references(
        criterion_id: &str,
        rendered_context: &str,
        references: &References,
    ) -> String {
        let def = get_criterion_definition(criterion_id);

        let mut prompt = format!(
            "Évalue le critère RGAA {} sur cette page web.\n\n",
            criterion_id
        );

        if let Some(def) = def {
            prompt.push_str("## Critère à évaluer\n\n");
            prompt.push_str(&format!("- **ID:** {}\n", def.id));
            prompt.push_str(&format!("- **Titre:** {}\n", def.title));
            prompt.push_str(&format!("- **Références WCAG:** {}\n", def.wcag_refs));
            prompt.push_str(&format!("- **Définition:** {}\n\n", def.definition));
        }

        prompt.push_str(rendered_context);
        prompt.push_str(&references::render(references));

        prompt.push_str("\n## Instructions\n\n");
        prompt.push_str(
            "1. Analyse le critère en fonction de la définition et des éléments ci-dessus\n",
        );
        prompt.push_str("2. Si une capture d'écran est fournie, utilise-la pour juger\n");
        prompt.push_str("3. Retourne un JSON avec les champs:\n");
        prompt.push_str("   - verdict: \"pass\", \"fail\", ou \"na\"\n");
        prompt.push_str("   - confidence: nombre entre 0.0 et 1.0\n");
        prompt.push_str("   - justification: explication détaillée en français\n");

        prompt
    }

    /// Builds a batch evaluation prompt for multiple criteria from an
    /// already-rendered page context. Returns a JSON array of results.
    pub fn build_batch_from_rendered(criterion_ids: &[&str], rendered_context: &str) -> String {
        let mut prompt = String::new();

        prompt.push_str("Évalue les critères RGAA suivants sur cette page web.\n\n");

        prompt.push_str("## Contexte de la page\n\n");
        prompt.push_str(rendered_context);
        prompt.push_str("\n\n");

        prompt.push_str("## Critères à évaluer\n\n");
        for criterion_id in criterion_ids {
            if let Some(def) = get_criterion_definition(criterion_id) {
                prompt.push_str(&format!("### Critère {}\n", criterion_id));
                prompt.push_str(&format!("- **Titre:** {}\n", def.title));
                prompt.push_str(&format!("- **Références WCAG:** {}\n", def.wcag_refs));
                prompt.push_str(&format!("- **Définition:** {}\n\n", def.definition));
            }
        }

        prompt.push_str("## Instructions\n\n");
        prompt.push_str(
            "1. Analyse chaque critère en fonction de la définition et du contexte de la page\n",
        );
        prompt.push_str(
            "2. Retourne un JSON array où chaque élément correspond à un critère dans l'ordre:\n",
        );
        prompt.push_str("   - criterion_id: l'ID du critère\n");
        prompt.push_str("   - verdict: \"pass\", \"fail\", ou \"na\"\n");
        prompt.push_str("   - confidence: nombre entre 0.0 et 1.0\n");
        prompt.push_str("   - justification: explication détaillée en français\n\n");
        prompt.push_str("Exemple de format de réponse:\n");
        prompt.push_str("[\n");
        prompt.push_str("  {\"criterion_id\": \"1.1\", \"verdict\": \"pass\", \"confidence\": 0.95, \"justification\": \"...\"},\n");
        prompt.push_str("  {\"criterion_id\": \"1.3\", \"verdict\": \"fail\", \"confidence\": 0.8, \"justification\": \"...\"}\n");
        prompt.push_str("]\n");

        prompt
    }
}

/// Returns the page discovery section of the agent preamble.
///
/// Tells the LLM how to use the `crawl_site` tool and how to prioritize
/// pages for RGAA accessibility auditing: mandatory pages first, then
/// site-specific pages (forms, search, checkout, etc.), then a random sample.
pub fn page_discovery_preamble() -> String {
    r#"## Page Discovery

You have access to the `crawl_site` tool to discover pages on the target website.

**How to use it:**
- Call `crawl_site` with the website URL, optional `max_pages` (default 20), and `max_depth` (default 3)
- The tool returns each discovered page with its URL, raw HTML, links, and HTTP status code
- HTML content is truncated to 50,000 characters per page; `truncated: true` indicates truncation

**Page selection strategy for accessibility auditing:**

1. **Mandatory pages** — always audit these when present:
   - Home page (/)
   - Sitemap (/sitemap.xml or /sitemap)
   - Contact page (/contact, /nous-contacter)
   - Legal mentions (/legal, /mentions-legales, /politique-de-confidentialite)

2. **Site-specific pages** — audit these when present:
   - Forms: /contact, /signup, /register, /signin, /login, /newsletter, /comment
   - Search: /search, /recherche, /find
   - Product/catalogue: /products, /catalogue, /shop, /boutique, /produit/*
   - User account: /account, /profile, /dashboard, /mon-compte
   - Navigation: any page with more than 10 links in the main nav

3. **Random sample** — if the site has >10 non-mandatory pages, audit a random sample of up to 5 additional pages to catch edge cases.

**After crawling:** Use the discovered pages to determine which ones to audit for RGAA criteria. Prioritize pages with forms, authentication, navigation, and interactive content.
"#.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_context_leaves_short_context_untouched() {
        let context = PageContext {
            title: Some("Home".to_string()),
            lang: Some("fr".to_string()),
            headings: vec![],
            images: vec![],
            iframes: vec![],
            links: vec![],
            forms: vec![],
            media: vec![],
            navigation: vec![],
        };
        let rendered = PromptBuilder::render_context(&context);
        assert!(rendered.len() <= MAX_CONTEXT_CHARS);
        assert!(!rendered.contains("truncated"));
    }

    #[test]
    fn render_context_caps_huge_context() {
        let context = PageContext {
            title: Some("A".repeat(50_000)),
            lang: None,
            headings: vec![],
            images: vec![],
            iframes: vec![],
            links: vec![],
            forms: vec![],
            media: vec![],
            navigation: vec![],
        };
        let rendered = PromptBuilder::render_context(&context);
        assert!(
            rendered.len() <= MAX_CONTEXT_CHARS + TRUNCATION_MARKER.len(),
            "rendered context must stay bounded, got {} chars",
            rendered.len()
        );
        assert!(rendered.contains("truncated"));
    }

    #[test]
    fn build_from_rendered_matches_build() {
        let context = PageContext {
            title: Some("Home".to_string()),
            lang: Some("fr".to_string()),
            headings: vec![],
            images: vec![],
            iframes: vec![],
            links: vec![],
            forms: vec![],
            media: vec![],
            navigation: vec![],
        };
        let via_build = PromptBuilder::build("1.1", &context);
        let via_rendered =
            PromptBuilder::build_from_rendered("1.1", &PromptBuilder::render_context(&context));
        assert_eq!(via_build, via_rendered);
    }

    #[test]
    fn build_with_empty_references_matches_build_without_references() {
        // AC: "Le rendu reste identique sans documents récupérés" — no
        // criterion without retrieval gets an empty "## Références" header.
        let without = PromptBuilder::build_from_rendered("1.1", "page context");
        let with_empty = PromptBuilder::build_from_rendered_with_references(
            "1.1",
            "page context",
            &References::default(),
        );
        assert_eq!(without, with_empty);
    }

    #[test]
    fn build_with_references_matches_snapshot() {
        use crate::references::{CrawlReference, ReferentielReference};

        let references = References {
            referentiel: vec![ReferentielReference {
                test_id: "1.1.1".into(),
                referentiel_version: "2024.1".into(),
                content: "Chaque image porteuse d'information a une alternative textuelle.".into(),
            }],
            crawl: vec![CrawlReference {
                url: "https://example.test/".into(),
                content: "Image sans attribut alt détectée sur la page d'accueil.".into(),
            }],
        };
        let prompt = PromptBuilder::build_from_rendered_with_references(
            "1.1",
            "## Contexte de la page\n\nTitre: Accueil",
            &references,
        );
        insta::assert_snapshot!(prompt, @r###"
        Évalue le critère RGAA 1.1 sur cette page web.

        ## Critère à évaluer

        - **ID:** 1.1
        - **Titre:** Alternative textuelle image porteuse d'information
        - **Références WCAG:** 1.1.1
        - **Définition:** Chaque image porteuse d'information a-t-elle une alternative textuelle ?

        ## Contexte de la page

        Titre: Accueil

        ## Références

        ### Référentiel

        - [1.1.1 v2024.1] Chaque image porteuse d'information a une alternative textuelle.

        ### Crawl

        - [https://example.test/] Image sans attribut alt détectée sur la page d'accueil.

        ## Instructions

        1. Analyse le critère en fonction de la définition et des éléments ci-dessus
        2. Si une capture d'écran est fournie, utilise-la pour juger
        3. Retourne un JSON avec les champs:
           - verdict: "pass", "fail", ou "na"
           - confidence: nombre entre 0.0 et 1.0
           - justification: explication détaillée en français
        "###);
    }

    #[test]
    fn references_section_is_ordered_before_instructions() {
        use crate::references::ReferentielReference;

        let references = References {
            referentiel: vec![ReferentielReference {
                test_id: "1.1.1".into(),
                referentiel_version: "2024.1".into(),
                content: "content".into(),
            }],
            crawl: vec![],
        };
        let prompt =
            PromptBuilder::build_from_rendered_with_references("1.1", "page context", &references);
        let refs_pos = prompt.find("## Références").unwrap();
        let instructions_pos = prompt.find("## Instructions").unwrap();
        assert!(refs_pos < instructions_pos);
    }
}
