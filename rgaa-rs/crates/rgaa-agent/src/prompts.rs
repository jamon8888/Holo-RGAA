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
