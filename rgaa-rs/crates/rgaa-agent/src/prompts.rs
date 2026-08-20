use crate::criteria_defs::get_criterion_definition;
use rgaa_holo::PageContext;

pub struct PromptBuilder;

impl PromptBuilder {
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

        prompt.push_str("## Contexte de la page\n\n");
        if let Some(ref title) = context.title {
            prompt.push_str(&format!("**Titre:** {}\n", title));
        }
        if let Some(ref lang) = context.lang {
            prompt.push_str(&format!("**Langue:** {}\n", lang));
        }

        prompt.push_str("\n## Éléments de la page\n\n");

        if !context.headings.is_empty() {
            prompt.push_str("### Titres\n");
            for h in &context.headings {
                prompt.push_str(&format!("  - H{}: {}\n", h.level, h.text));
            }
            prompt.push('\n');
        }

        if !context.images.is_empty() {
            prompt.push_str("### Images\n");
            for img in &context.images {
                let alt_info = if img.is_decorative {
                    "(décorative)".to_string()
                } else if img.has_alt {
                    format!("alt: \"{}\"", img.alt.as_deref().unwrap_or(""))
                } else {
                    "(sans alt)".to_string()
                };
                prompt.push_str(&format!("  - src=\"{}\" {}\n", img.src, alt_info));
            }
            prompt.push('\n');
        }

        if !context.iframes.is_empty() {
            prompt.push_str("### Iframes\n");
            for iframe in &context.iframes {
                let title_info = if iframe.has_title {
                    format!("title: \"{}\"", iframe.title.as_deref().unwrap_or(""))
                } else {
                    "(sans titre)".to_string()
                };
                prompt.push_str(&format!(
                    "  - src=\"{}\" {}\n",
                    iframe.src.as_deref().unwrap_or(""),
                    title_info
                ));
            }
            prompt.push('\n');
        }

        if !context.links.is_empty() {
            prompt.push_str("### Liens\n");
            for link in &context.links {
                let text_info = if link.is_empty {
                    "(vide)"
                } else if link.has_text {
                    link.text.as_str()
                } else {
                    "(sans texte)"
                };
                prompt.push_str(&format!("  - href=\"{}\" {}\n", link.href, text_info));
            }
            prompt.push('\n');
        }

        if !context.forms.is_empty() {
            prompt.push_str("### Formulaires\n");
            for form in &context.forms {
                prompt.push_str(&format!(
                    "  - Form{} (labels: {}, submit: {})\n",
                    form.id.as_deref().unwrap_or(""),
                    if form.has_labels { "oui" } else { "non" },
                    if form.has_submit { "oui" } else { "non" }
                ));
                for input in &form.inputs {
                    prompt.push_str(&format!(
                        "    - type={}, label: {}\n",
                        input.input_type,
                        if input.has_label { "oui" } else { "non" }
                    ));
                }
            }
            prompt.push('\n');
        }

        if !context.media.is_empty() {
            prompt.push_str("### Médias\n");
            for media in &context.media {
                prompt.push_str(&format!(
                    "  - type={}, contrôles: {}, sous-titres: {}, transcription: {}\n",
                    media.media_type,
                    if media.has_controls { "oui" } else { "non" },
                    if media.has_captions { "oui" } else { "non" },
                    if media.has_transcript { "oui" } else { "non" }
                ));
            }
            prompt.push('\n');
        }

        if !context.navigation.is_empty() {
            prompt.push_str("### Navigation\n");
            for nav in &context.navigation {
                prompt.push_str(&format!("  - {}\n", nav));
            }
            prompt.push('\n');
        }

        prompt.push_str("\n## Instructions\n\n");
        prompt.push_str("1. Analyse le critère en fonction de la définition et des éléments ci-dessus\n");
        prompt.push_str("2. Si une capture d'écran est fournie, utilise-la pour juger\n");
        prompt.push_str("3. Retourne un JSON avec les champs:\n");
        prompt.push_str("   - verdict: \"pass\", \"fail\", ou \"na\"\n");
        prompt.push_str("   - confidence: nombre entre 0.0 et 1.0\n");
        prompt.push_str("   - justification: explication détaillée en français\n");

        prompt
    }

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
