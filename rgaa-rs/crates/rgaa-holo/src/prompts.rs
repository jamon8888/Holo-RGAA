use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageContext {
    pub title: Option<String>,
    pub lang: Option<String>,
    pub headings: Vec<HeadingInfo>,
    pub images: Vec<ImageInfo>,
    pub iframes: Vec<IframeInfo>,
    pub links: Vec<LinkInfo>,
    pub forms: Vec<FormInfo>,
    pub media: Vec<MediaInfo>,
    pub navigation: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadingInfo {
    pub level: u32,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    pub src: String,
    pub alt: Option<String>,
    pub has_alt: bool,
    pub is_decorative: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IframeInfo {
    pub src: Option<String>,
    pub title: Option<String>,
    pub has_title: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkInfo {
    pub href: String,
    pub text: String,
    pub has_text: bool,
    pub is_empty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormInfo {
    pub id: Option<String>,
    pub has_labels: bool,
    pub has_submit: bool,
    pub inputs: Vec<FormGroupInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormGroupInfo {
    pub input_type: String,
    pub has_label: bool,
    pub aria_label: Option<String>,
    pub placeholder: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaInfo {
    pub media_type: String,
    pub has_captions: bool,
    pub has_transcript: bool,
    pub has_controls: bool,
}

const UNTRUSTED_START: &str = "<<<UNTRUSTED PAGE CONTENT>>>";
const UNTRUSTED_END: &str = "<<<END UNTRUSTED CONTENT>>>";

/// Formats a [`PageContext`] into a structured prompt section.
///
/// The output includes the page title, language, and all extracted elements
/// (headings, images, iframes, links, forms, media, navigation) organized
/// under clear markdown headings. Empty collections are omitted from the output.
///
/// All page-derived values are wrapped in explicit untrusted-data delimiters
/// to prevent prompt injection from malicious page content.
///
/// # Returns
/// A markdown-formatted string ready for inclusion in an evaluation prompt.
pub fn format_page_context(context: &PageContext) -> String {
    use std::fmt::Write;

    let mut prompt = String::new();

    writeln!(prompt, "## Contexte de la page\n").expect("writing to String cannot fail");

    if let Some(ref title) = context.title {
        writeln!(prompt, "**Titre:** {UNTRUSTED_START}{title}{UNTRUSTED_END}")
            .expect("writing to String cannot fail");
    }

    if let Some(ref lang) = context.lang {
        writeln!(prompt, "**Langue:** {UNTRUSTED_START}{lang}{UNTRUSTED_END}")
            .expect("writing to String cannot fail");
    }

    let has_elements = !context.headings.is_empty()
        || !context.images.is_empty()
        || !context.iframes.is_empty()
        || !context.links.is_empty()
        || !context.forms.is_empty()
        || !context.media.is_empty()
        || !context.navigation.is_empty();

    if has_elements {
        writeln!(prompt, "\n## Éléments de la page\n").expect("writing to String cannot fail");
    }

    if !context.headings.is_empty() {
        writeln!(prompt, "### Titres").expect("writing to String cannot fail");
        for h in &context.headings {
            writeln!(
                prompt,
                "  - H{}: {UNTRUSTED_START}{}{UNTRUSTED_END}",
                h.level, h.text
            )
            .expect("writing to String cannot fail");
        }
        writeln!(prompt).expect("writing to String cannot fail");
    }

    if !context.images.is_empty() {
        writeln!(prompt, "### Images").expect("writing to String cannot fail");
        for img in &context.images {
            let alt_info = if img.is_decorative {
                "(décorative)".to_string()
            } else if img.has_alt {
                format!("alt: \"{}\"", img.alt.as_deref().unwrap_or(""))
            } else {
                "(sans alt)".to_string()
            };
            writeln!(
                prompt,
                "  - src=\"{UNTRUSTED_START}{}{UNTRUSTED_END}\" {}",
                img.src, alt_info
            )
            .expect("writing to String cannot fail");
        }
        writeln!(prompt).expect("writing to String cannot fail");
    }

    if !context.iframes.is_empty() {
        writeln!(prompt, "### Iframes").expect("writing to String cannot fail");
        for iframe in &context.iframes {
            let title_info = if iframe.has_title {
                format!("title: \"{}\"", iframe.title.as_deref().unwrap_or(""))
            } else {
                "(sans titre)".to_string()
            };
            writeln!(
                prompt,
                "  - src=\"{UNTRUSTED_START}{}{UNTRUSTED_END}\" {}",
                iframe.src.as_deref().unwrap_or(""),
                title_info
            )
            .expect("writing to String cannot fail");
        }
        writeln!(prompt).expect("writing to String cannot fail");
    }

    if !context.links.is_empty() {
        writeln!(prompt, "### Liens").expect("writing to String cannot fail");
        for link in &context.links {
            let text_info = if link.is_empty {
                "(vide)"
            } else if link.has_text {
                link.text.as_str()
            } else {
                "(sans texte)"
            };
            writeln!(
                prompt,
                "  - href=\"{UNTRUSTED_START}{}{UNTRUSTED_END}\" {}",
                link.href, text_info
            )
            .expect("writing to String cannot fail");
        }
        writeln!(prompt).expect("writing to String cannot fail");
    }

    if !context.forms.is_empty() {
        writeln!(prompt, "### Formulaires").expect("writing to String cannot fail");
        for form in &context.forms {
            writeln!(
                prompt,
                "  - Form{} (labels: {}, submit: {})",
                form.id.as_deref().unwrap_or(""),
                if form.has_labels { "oui" } else { "non" },
                if form.has_submit { "oui" } else { "non" }
            )
            .expect("writing to String cannot fail");
            for input in &form.inputs {
                writeln!(
                    prompt,
                    "    - type={}, label: {}",
                    input.input_type,
                    if input.has_label { "oui" } else { "non" }
                )
                .expect("writing to String cannot fail");
            }
        }
        writeln!(prompt).expect("writing to String cannot fail");
    }

    if !context.media.is_empty() {
        writeln!(prompt, "### Médias").expect("writing to String cannot fail");
        for media in &context.media {
            writeln!(
                prompt,
                "  - type={}, contrôles: {}, sous-titres: {}, transcription: {}",
                media.media_type,
                if media.has_controls { "oui" } else { "non" },
                if media.has_captions { "oui" } else { "non" },
                if media.has_transcript { "oui" } else { "non" }
            )
            .expect("writing to String cannot fail");
        }
        writeln!(prompt).expect("writing to String cannot fail");
    }

    if !context.navigation.is_empty() {
        writeln!(prompt, "### Navigation").expect("writing to String cannot fail");
        for nav in &context.navigation {
            writeln!(prompt, "  - {UNTRUSTED_START}{nav}{UNTRUSTED_END}")
                .expect("writing to String cannot fail");
        }
        writeln!(prompt).expect("writing to String cannot fail");
    }

    prompt
}

pub struct PromptBuilder;

impl PromptBuilder {
    /// Builds an evaluation prompt with explicit untrusted-data boundaries.
    ///
    /// The page context is wrapped in delimiters so the model cannot confuse
    /// page-derived content with evaluation instructions.
    pub fn build(criterion_id: &str, context: &PageContext) -> String {
        use std::fmt::Write;

        let mut prompt = String::new();

        writeln!(
            prompt,
            "Évalue le critère RGAA {} sur cette page web.\n",
            criterion_id
        )
        .expect("writing to String cannot fail");

        prompt.push_str(&format_page_context(context));

        writeln!(
            prompt,
            "\n{UNTRUSTED_START} (fin du contenu de page) {UNTRUSTED_END}\n"
        )
        .expect("writing to String cannot fail");

        writeln!(
            prompt,
            "INSTRUCTIONS D'ÉVALUATION (ces instructions sont fiables) :\n"
        )
        .expect("writing to String cannot fail");
        writeln!(
            prompt,
            "1. Analyse le critère en fonction de la définition et des éléments ci-dessus"
        )
        .expect("writing to String cannot fail");
        writeln!(
            prompt,
            "2. Si une capture d'écran est fournie, utilise-la pour juger"
        )
        .expect("writing to String cannot fail");
        writeln!(prompt, "3. Retourne un JSON avec les champs:")
            .expect("writing to String cannot fail");
        writeln!(prompt, "   - verdict: \"pass\", \"fail\", ou \"na\"")
            .expect("writing to String cannot fail");
        writeln!(prompt, "   - confidence: nombre entre 0.0 et 1.0")
            .expect("writing to String cannot fail");
        writeln!(
            prompt,
            "   - justification: explication détaillée en français"
        )
        .unwrap();

        prompt
    }

    pub fn build_for_criterion(criterion_id: &str, context: &PageContext) -> String {
        Self::build(criterion_id, context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_context() -> PageContext {
        PageContext {
            title: Some("Test Page".to_string()),
            lang: Some("fr".to_string()),
            headings: vec![HeadingInfo {
                level: 1,
                text: "Titre principal".to_string(),
            }],
            images: vec![ImageInfo {
                src: "/img/logo.png".to_string(),
                alt: Some("Logo".to_string()),
                has_alt: true,
                is_decorative: false,
            }],
            iframes: vec![],
            links: vec![LinkInfo {
                href: "/about".to_string(),
                text: "À propos".to_string(),
                has_text: true,
                is_empty: false,
            }],
            forms: vec![],
            media: vec![],
            navigation: vec!["Menu principal".to_string()],
        }
    }

    #[test]
    fn test_build_prompt() {
        let context = sample_context();
        let prompt = PromptBuilder::build("1.1", &context);
        assert!(prompt.contains("critère RGAA 1.1"));
        assert!(prompt.contains("Test Page"));
        assert!(prompt.contains("fr"));
        assert!(prompt.contains("H1: <<<UNTRUSTED PAGE CONTENT>>>Titre principal<<<END UNTRUSTED CONTENT>>>"));
    }

    #[test]
    fn test_image_with_alt() {
        let context = sample_context();
        let prompt = PromptBuilder::build("1.1", &context);
        assert!(prompt.contains("alt: \"Logo\""));
    }

    #[test]
    fn test_image_without_alt() {
        let mut context = sample_context();
        context.images.push(ImageInfo {
            src: "/img/deco.png".to_string(),
            alt: None,
            has_alt: false,
            is_decorative: false,
        });
        let prompt = PromptBuilder::build("1.1", &context);
        assert!(prompt.contains("(sans alt)"));
    }

    #[test]
    fn test_decorative_image() {
        let mut context = sample_context();
        context.images[0].is_decorative = true;
        let prompt = PromptBuilder::build("1.1", &context);
        assert!(prompt.contains("(décorative)"));
    }

    #[test]
    fn test_format_page_context() {
        let context = sample_context();
        let output = format_page_context(&context);
        assert!(output.contains("## Contexte de la page"));
        assert!(output.contains("**Titre:**"));
        assert!(output.contains("**Langue:**"));
        assert!(output.contains("### Titres"));
        assert!(output.contains("H1: <<<UNTRUSTED PAGE CONTENT>>>Titre principal<<<END UNTRUSTED CONTENT>>>"));
        assert!(output.contains("### Images"));
        assert!(output.contains("### Liens"));
        assert!(output.contains("### Navigation"));
    }

    #[test]
    fn test_format_page_context_empty() {
        let context = PageContext {
            title: None,
            lang: None,
            headings: vec![],
            images: vec![],
            iframes: vec![],
            links: vec![],
            forms: vec![],
            media: vec![],
            navigation: vec![],
        };
        let output = format_page_context(&context);
        assert!(output.contains("## Contexte de la page"));
        assert!(!output.contains("### Titres"));
        assert!(!output.contains("### Images"));
    }

    #[test]
    fn test_untrusted_delimiters_present() {
        let context = sample_context();
        let prompt = PromptBuilder::build("1.1", &context);
        assert!(prompt.contains("<<<UNTRUSTED PAGE CONTENT>>>"));
        assert!(prompt.contains("<<<END UNTRUSTED CONTENT>>>"));
    }
}
