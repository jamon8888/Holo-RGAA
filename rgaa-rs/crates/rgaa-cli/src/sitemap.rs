//! Sitemap-based page discovery.
//!
//! Some sites (client-rendered SPAs in particular) expose almost no `<a
//! href>` markup in their raw HTML, so link-crawling (`SpiderTool`) and the
//! fixed RGAA-path guessing (`sample_mode`) both find few or no pages. A
//! `sitemap.xml` sidesteps that: it's already the site's own declared list
//! of pages. This picks the site's top-level "pillar" pages — one path
//! segment deep, e.g. `/oeuvres/` but not `/oeuvres/item/` — ranked by the
//! sitemap's own `<priority>`, which is how sitemap generators typically
//! mark section/index pages above individual content leaves.

use crate::CliError;

#[derive(Debug, Clone, PartialEq)]
struct SitemapEntry {
    url: String,
    priority: f64,
}

/// Fetches `sitemap_url` and returns up to `limit` top-level page URLs,
/// ordered by descending sitemap priority (ties broken by URL). Locale
/// alternates (a first path segment that looks like a language code, e.g.
/// `/en/...`) are excluded so the audit doesn't mix languages.
pub async fn discover_pillar_pages(sitemap_url: &str, limit: usize) -> Result<Vec<String>, CliError> {
    let body = reqwest::get(sitemap_url)
        .await
        .map_err(|e| CliError::execution(format!("failed to fetch sitemap {sitemap_url}: {e}")))?
        .text()
        .await
        .map_err(|e| CliError::execution(format!("failed to read sitemap {sitemap_url}: {e}")))?;

    Ok(select_pillar_pages(&body, limit))
}

fn select_pillar_pages(xml: &str, limit: usize) -> Vec<String> {
    let mut entries: Vec<SitemapEntry> = parse_sitemap(xml)
        .into_iter()
        .filter(|e| is_top_level(&e.url) && !is_locale_alternate(&e.url))
        .collect();

    entries.sort_by(|a, b| {
        b.priority
            .partial_cmp(&a.priority)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.url.cmp(&b.url))
    });

    entries.into_iter().take(limit).map(|e| e.url).collect()
}

/// Minimal `<url><loc>...</loc>...<priority>...</priority></url>` extraction
/// — sitemap.xml is machine-generated, well-formed, flat markup, so this
/// avoids pulling in a full XML parser for a handful of tags.
fn parse_sitemap(xml: &str) -> Vec<SitemapEntry> {
    xml.split("<url>")
        .skip(1)
        .filter_map(|block| {
            let block = block.split("</url>").next().unwrap_or(block);
            let url = extract_tag(block, "loc")?;
            let priority = extract_tag(block, "priority")
                .and_then(|p| p.parse::<f64>().ok())
                .unwrap_or(0.5);
            Some(SitemapEntry { url, priority })
        })
        .collect()
}

fn extract_tag(block: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = block.find(&open)? + open.len();
    let end = block[start..].find(&close)?;
    Some(block[start..start + end].trim().to_string())
}

fn is_top_level(url: &str) -> bool {
    path_segments(url).len() <= 1
}

fn is_locale_alternate(url: &str) -> bool {
    // Heuristic: a 2-letter first segment is treated as a language code
    // (en, fr, de, ...) rather than a real section name.
    path_segments(url)
        .first()
        .is_some_and(|seg| seg.len() == 2 && seg.chars().all(|c| c.is_ascii_alphabetic()))
}

fn path_segments(url: &str) -> Vec<String> {
    reqwest::Url::parse(url)
        .ok()
        .and_then(|parsed| {
            parsed
                .path_segments()
                .map(|segments| segments.filter(|s| !s.is_empty()).map(String::from).collect())
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_SITEMAP: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
<url><loc>https://example.test/</loc><priority>1</priority></url>
<url><loc>https://example.test/en/</loc><priority>0.9</priority></url>
<url><loc>https://example.test/oeuvres/</loc><priority>0.9</priority></url>
<url><loc>https://example.test/oeuvres/item-1/</loc><priority>0.9</priority></url>
<url><loc>https://example.test/a-propos/</loc><priority>0.8</priority></url>
<url><loc>https://example.test/series/</loc><priority>0.7</priority></url>
</urlset>"#;

    #[test]
    fn selects_only_top_level_non_locale_pages() {
        let pages = select_pillar_pages(SAMPLE_SITEMAP, 10);
        assert_eq!(
            pages,
            vec![
                "https://example.test/",
                "https://example.test/oeuvres/",
                "https://example.test/a-propos/",
                "https://example.test/series/",
            ]
        );
    }

    #[test]
    fn respects_limit() {
        let pages = select_pillar_pages(SAMPLE_SITEMAP, 2);
        assert_eq!(
            pages,
            vec!["https://example.test/", "https://example.test/oeuvres/"]
        );
    }

    #[test]
    fn missing_priority_defaults_to_mid_rank() {
        let xml = r#"<url><loc>https://example.test/no-priority/</loc></url>
<url><loc>https://example.test/high/</loc><priority>0.9</priority></url>"#;
        let pages = select_pillar_pages(xml, 10);
        assert_eq!(
            pages,
            vec!["https://example.test/high/", "https://example.test/no-priority/"]
        );
    }
}
