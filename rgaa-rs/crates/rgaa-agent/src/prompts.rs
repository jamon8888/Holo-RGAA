use crate::criteria_defs::get_criterion_definition;
use rgaa_holo::{format_page_context, PageContext};

/// Builds structured evaluation prompts for Holo3.
///
/// The prompt includes the criterion definition, WCAG references,
/// and the full page context (headings, images, forms, etc.).
pub struct PromptBuilder;

impl PromptBuilder {
    /// Builds a text-only evaluation prompt for `criterion_id`.
    ///
    /// # Returns
    /// A formatted prompt string ready to send to the Holo3 API.
    pub fn build(criterion_id: &str, context: &PageContext) -> String {
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

        prompt.push_str(&format_page_context(context));

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

    /// Builds a prompt that includes an image description.
    ///
    /// Useful when a screenshot is available and the evaluator should
    /// incorporate visual information into the assessment.
    pub fn build_with_image(
        criterion_id: &str,
        context: &PageContext,
        image_description: &str,
    ) -> String {
        let mut prompt = Self::build(criterion_id, context);
        prompt.push_str(&format!(
            "\n\n## Capture d'écran\n\nUne capture d'écran de la page est fournie. Utilise-la pour évaluer le critère {}.\nDescription: {}",
            criterion_id, image_description
        ));
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
