use rgaa_core::RgaaError;
use serde::{Deserialize, Serialize};

/// Page facts extracted in the browser by [`PageSnapshot::extraction_snippet`].
/// Shared by every SEO/GEO/AEO rule; produced from the same crawl as the RGAA pass.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PageSnapshot {
    pub url: String,
    #[serde(default)]
    pub lang: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub meta_description: Option<String>,
    /// Lower-cased robots directives, e.g. `["noindex", "nofollow"]`.
    #[serde(default)]
    pub meta_robots: Vec<String>,
    #[serde(default)]
    pub canonicals: Vec<String>,
    #[serde(default)]
    pub hreflangs: Vec<Hreflang>,
    #[serde(default)]
    pub headings: Vec<Heading>,
    /// First paragraph following the first `<h1>`.
    #[serde(default)]
    pub lead_paragraph: Option<String>,
    #[serde(default)]
    pub images_total: usize,
    #[serde(default)]
    pub images_missing_alt: usize,
    /// Raw text of each `<script type="application/ld+json">`, unparsed.
    #[serde(default)]
    pub json_ld: Vec<String>,
    /// Visible body text, used for NAP matching.
    #[serde(default)]
    pub body_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Hreflang {
    pub lang: String,
    pub href: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    /// Text of the first `<p>` after this heading, before the next heading.
    #[serde(default)]
    pub next_paragraph: Option<String>,
}

/// Google Business Profile facts to check NAP consistency against.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BusinessProfile {
    pub name: String,
    pub phone: String,
    pub address: String,
}

impl PageSnapshot {
    pub fn from_json(json: &str) -> Result<Self, RgaaError> {
        serde_json::from_str(json)
            .map_err(|e| RgaaError::Seo(format!("Failed to parse page snapshot JSON: {e}")))
    }

    /// Browser-side extractor. Returns the JSON string that [`from_json`](Self::from_json) accepts.
    /// Executed the same way as `GapFixRules` snippets, on the crawl already captured for RGAA.
    #[must_use]
    pub fn extraction_snippet() -> &'static str {
        r#"
        (() => {
            const text = el => (el && el.textContent || '').replace(/\s+/g, ' ').trim();
            const attr = (sel, name) => { const el = document.querySelector(sel); return el ? el.getAttribute(name) : null; };
            const robots = (attr('meta[name="robots" i]', 'content') || '')
                .split(',').map(s => s.trim().toLowerCase()).filter(Boolean);
            const canonicals = [...document.querySelectorAll('link[rel="canonical"]')]
                .map(l => l.href).filter(Boolean);
            const hreflangs = [...document.querySelectorAll('link[rel="alternate"][hreflang]')]
                .map(l => ({ lang: l.getAttribute('hreflang'), href: l.href }));
            const nextParagraph = h => {
                let el = h.nextElementSibling;
                while (el && !/^H[1-6]$/.test(el.tagName)) {
                    if (el.tagName === 'P' && text(el)) return text(el);
                    const p = el.querySelector && el.querySelector('p');
                    if (p && text(p)) return text(p);
                    el = el.nextElementSibling;
                }
                return null;
            };
            const headingEls = [...document.querySelectorAll('h1,h2,h3,h4,h5,h6')];
            const headings = headingEls.map(h => ({
                level: Number(h.tagName[1]), text: text(h), next_paragraph: nextParagraph(h)
            }));
            const h1 = headingEls.find(h => h.tagName === 'H1');
            const imgs = [...document.querySelectorAll('img')];
            const json_ld = [...document.querySelectorAll('script[type="application/ld+json"]')]
                .map(s => s.textContent || '');
            return JSON.stringify({
                url: location.href,
                lang: document.documentElement.getAttribute('lang'),
                title: text(document.querySelector('title')) || null,
                meta_description: attr('meta[name="description" i]', 'content'),
                meta_robots: robots,
                canonicals, hreflangs, headings,
                lead_paragraph: h1 ? nextParagraph(h1) : null,
                images_total: imgs.length,
                images_missing_alt: imgs.filter(i => !i.hasAttribute('alt')).length,
                json_ld,
                body_text: text(document.body).slice(0, 200000)
            });
        })()
        "#
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_snapshot() {
        let snap = PageSnapshot::from_json(r#"{"url":"https://a.test/"}"#).unwrap();
        assert_eq!(snap.url, "https://a.test/");
        assert!(snap.headings.is_empty());
        assert!(snap.title.is_none());
    }

    #[test]
    fn rejects_invalid_json() {
        let err = PageSnapshot::from_json("{").unwrap_err();
        assert!(err.to_string().contains("page snapshot"));
    }

    #[test]
    fn snippet_is_self_invoking_expression() {
        let s = PageSnapshot::extraction_snippet().trim();
        assert!(s.starts_with("(() =>"));
        assert!(s.ends_with("})()"));
    }
}
