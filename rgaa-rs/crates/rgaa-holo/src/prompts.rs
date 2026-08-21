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

pub fn format_page_context(context: &PageContext) -> String {
    let mut prompt = String::new();

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

    prompt
}

pub struct PromptBuilder;

impl PromptBuilder {
    pub fn build(criterion_id: &str, context: &PageContext) -> String {
        let mut prompt = format!(
            "Évalue le critère RGAA {} sur cette page web.\n\n",
            criterion_id
        );

        prompt.push_str(&format_page_context(context));

        prompt.push_str(&format!(
            "\nÉvalue le critère {} en te basant sur ces éléments. Retourne un JSON.",
            criterion_id
        ));

        prompt
    }

    pub fn build_for_criterion(criterion_id: &str, context: &PageContext) -> String {
        let prefix = criterion_id.split('-').next().unwrap_or(criterion_id);
        let base_criterion = Self::get_base_criterion(prefix);

        if base_criterion != prefix {
            format!(
                "{}\n\nNote: Ce critère fait partie du groupe {}. Concentre-toi sur {}.",
                Self::build(criterion_id, context),
                base_criterion,
                Self::get_criterion_focus(criterion_id)
            )
        } else {
            Self::build(criterion_id, context)
        }
    }

    fn get_base_criterion(criterion_id: &str) -> String {
        match criterion_id {
            "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "10" => "1".to_string(),
            "11" | "12" | "13" => "11".to_string(),
            "14" | "15" | "16" | "17" => "14".to_string(),
            "18" | "19" | "20" | "21" | "22" | "23" | "24" | "25" | "26" | "27" => "18".to_string(),
            "28" | "29" | "30" | "31" | "32" | "33" | "34" | "35" | "36" | "37" | "38" | "39"
            | "40" | "41" | "42" | "43" | "44" | "45" | "46" | "47" | "48" | "49" | "50" | "51"
            | "52" | "53" | "54" | "55" | "56" | "57" | "58" | "59" | "60" | "61" | "62" | "63"
            | "64" | "65" | "66" | "67" | "68" | "69" | "70" | "71" | "72" | "73" | "74" | "75"
            | "76" | "77" | "78" | "79" | "80" | "81" | "82" | "83" | "84" | "85" | "86" | "87"
            | "88" | "89" | "90" | "91" | "92" | "93" | "94" | "95" | "96" | "97" | "98" | "99"
            | "100" | "101" | "102" | "103" | "104" | "105" | "106" | "107" | "108" | "109"
            | "110" | "111" | "112" | "113" | "114" | "115" | "116" | "117" | "118" | "119"
            | "120" => "28".to_string(),
            _ => criterion_id.to_string(),
        }
    }

    fn get_criterion_focus(criterion_id: &str) -> String {
        let parts: Vec<&str> = criterion_id.split('-').collect();
        if parts.len() > 1 {
            format!("le sous-critère {}", parts[1])
        } else {
            "ce critère".to_string()
        }
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
        assert!(prompt.contains("H1: Titre principal"));
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
        assert!(output.contains("**Titre:** Test Page"));
        assert!(output.contains("**Langue:** fr"));
        assert!(output.contains("### Titres"));
        assert!(output.contains("H1: Titre principal"));
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
}
