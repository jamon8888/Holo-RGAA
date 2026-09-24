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

use std::time::Duration;

use crate::CliError;

/// Applies to both the top-level sitemap fetch and each sitemap-index child
/// fetch — the async default `reqwest` client has no timeout at all, so an
/// unresponsive host would otherwise hang the audit indefinitely.
const FETCH_TIMEOUT: Duration = Duration::from_secs(15);

/// A `<sitemapindex>` can reference arbitrarily many child sitemaps; this
/// bounds how many we'll follow so a malicious or oversized index can't turn
/// page discovery into an unbounded fetch loop.
const MAX_SITEMAP_INDEX_CHILDREN: usize = 10;

#[derive(Debug, Clone, PartialEq)]
struct SitemapEntry {
    url: String,
    priority: f64,
}

/// Fetches `sitemap_url` (following one level of `<sitemapindex>` if
/// present) and returns up to `limit` top-level page URLs on the same
/// origin as `target_url`, ordered by descending sitemap priority (ties
/// broken by URL). Locale alternates (a first path segment that looks like
/// a language code, e.g. `/en/...`) are excluded so the audit doesn't mix
/// languages, and pages on a different origin than `target_url` are
/// excluded so a sitemap referencing other sites can't get their pages
/// audited and attributed to the requested site.
pub async fn discover_pillar_pages(
    sitemap_url: &str,
    target_url: &str,
    limit: usize,
) -> Result<Vec<String>, CliError> {
    let client = reqwest::Client::builder()
        .timeout(FETCH_TIMEOUT)
        .build()
        .map_err(|e| CliError::execution(format!("failed to build HTTP client: {e}")))?;

    let entries = fetch_sitemap_entries(&client, sitemap_url).await?;
    Ok(select_pillar_pages(entries, target_url, limit))
}

async fn fetch_text(client: &reqwest::Client, url: &str) -> Result<String, CliError> {
    client
        .get(url)
        .send()
        .await
        .map_err(|e| CliError::execution(format!("failed to fetch {url}: {e}")))?
        .text()
        .await
        .map_err(|e| CliError::execution(format!("failed to read {url}: {e}")))
}

async fn fetch_sitemap_entries(
    client: &reqwest::Client,
    sitemap_url: &str,
) -> Result<Vec<SitemapEntry>, CliError> {
    let body = fetch_text(client, sitemap_url).await?;

    if !is_sitemap_index(&body) {
        return Ok(parse_sitemap(&body));
    }

    let mut entries = Vec::new();
    for child_url in parse_sitemap_index(&body)
        .into_iter()
        .take(MAX_SITEMAP_INDEX_CHILDREN)
    {
        // Best-effort: one unreachable child sitemap shouldn't fail
        // discovery for every other sitemap the index lists.
        if let Ok(child_body) = fetch_text(client, &child_url).await {
            entries.extend(parse_sitemap(&child_body));
        }
    }
    Ok(entries)
}

fn select_pillar_pages(entries: Vec<SitemapEntry>, target_url: &str, limit: usize) -> Vec<String> {
    let mut entries: Vec<SitemapEntry> = entries
        .into_iter()
        .filter(|e| {
            is_top_level(&e.url) && !is_locale_alternate(&e.url) && same_origin(&e.url, target_url)
        })
        .collect();

    entries.sort_by(|a, b| {
        b.priority
            .partial_cmp(&a.priority)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.url.cmp(&b.url))
    });

    entries.into_iter().take(limit).map(|e| e.url).collect()
}

/// True for a `<sitemapindex>` document (a sitemap of sitemaps, per the
/// sitemaps.org protocol), as opposed to a regular `<urlset>` of pages.
fn is_sitemap_index(xml: &str) -> bool {
    xml.contains("<sitemapindex")
}

/// Minimal `<sitemap><loc>...</loc></sitemap>` extraction for a sitemap
/// index's child sitemap URLs.
fn parse_sitemap_index(xml: &str) -> Vec<String> {
    xml.split("<sitemap>")
        .skip(1)
        .filter_map(|block| {
            let block = block.split("</sitemap>").next().unwrap_or(block);
            extract_tag(block, "loc").map(|loc| decode_xml_entities(&loc))
        })
        .collect()
}

/// Minimal `<url><loc>...</loc>...<priority>...</priority></url>` extraction
/// — sitemap.xml is machine-generated, well-formed, flat markup, so this
/// avoids pulling in a full XML parser for a handful of tags.
fn parse_sitemap(xml: &str) -> Vec<SitemapEntry> {
    xml.split("<url>")
        .skip(1)
        .filter_map(|block| {
            let block = block.split("</url>").next().unwrap_or(block);
            let url = decode_xml_entities(&extract_tag(block, "loc")?);
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

/// Decodes the 5 predefined XML entities. `sitemap.xml` values are expected
/// to only ever need these (URLs don't contain raw `<`/`"`/etc. — they'd be
/// percent-encoded), so this deliberately skips numeric character
/// references rather than pulling in a full XML/HTML entity decoder.
/// `&amp;` is decoded last so an escaped literal like `&amp;lt;` becomes the
/// literal text `&lt;`, not a second decode pass down to `<`.
fn decode_xml_entities(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
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

fn same_origin(a: &str, b: &str) -> bool {
    match (reqwest::Url::parse(a), reqwest::Url::parse(b)) {
        (Ok(a), Ok(b)) => a.origin() == b.origin(),
        _ => false,
    }
}

fn path_segments(url: &str) -> Vec<String> {
    reqwest::Url::parse(url)
        .ok()
        .and_then(|parsed| {
            parsed.path_segments().map(|segments| {
                segments
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .collect()
            })
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TARGET: &str = "https://example.test/";

    const SAMPLE_SITEMAP: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
<url><loc>https://example.test/</loc><priority>1</priority></url>
<url><loc>https://example.test/en/</loc><priority>0.9</priority></url>
<url><loc>https://example.test/oeuvres/</loc><priority>0.9</priority></url>
<url><loc>https://example.test/oeuvres/item-1/</loc><priority>0.9</priority></url>
<url><loc>https://example.test/a-propos/</loc><priority>0.8</priority></url>
<url><loc>https://example.test/series/</loc><priority>0.7</priority></url>
</urlset>"#;

    fn select(xml: &str, target: &str, limit: usize) -> Vec<String> {
        select_pillar_pages(parse_sitemap(xml), target, limit)
    }

    #[test]
    fn selects_only_top_level_non_locale_pages() {
        let pages = select(SAMPLE_SITEMAP, TARGET, 10);
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
        let pages = select(SAMPLE_SITEMAP, TARGET, 2);
        assert_eq!(
            pages,
            vec!["https://example.test/", "https://example.test/oeuvres/"]
        );
    }

    #[test]
    fn missing_priority_defaults_to_mid_rank() {
        let xml = r#"<url><loc>https://example.test/no-priority/</loc></url>
<url><loc>https://example.test/high/</loc><priority>0.9</priority></url>"#;
        let pages = select(xml, TARGET, 10);
        assert_eq!(
            pages,
            vec![
                "https://example.test/high/",
                "https://example.test/no-priority/"
            ]
        );
    }

    #[test]
    fn rejects_pages_from_a_different_origin() {
        let xml = r#"<url><loc>https://example.test/legit/</loc><priority>0.5</priority></url>
<url><loc>https://attacker.test/foreign/</loc><priority>1.0</priority></url>"#;
        let pages = select(xml, TARGET, 10);
        assert_eq!(pages, vec!["https://example.test/legit/"]);
    }

    #[test]
    fn decodes_ampersand_in_loc() {
        let xml = r#"<url><loc>https://example.test/search?a=1&amp;b=2</loc></url>"#;
        // A query string makes the URL 2+ path-agnostic segments deep at
        // path level "/", so it still counts as top-level; what matters
        // here is that the entity was decoded, not double-decoded.
        let pages = select(xml, TARGET, 10);
        assert_eq!(pages, vec!["https://example.test/search?a=1&b=2"]);
    }

    #[test]
    fn detects_sitemap_index_and_parses_child_locs() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
<sitemap><loc>https://example.test/sitemap-pages.xml</loc></sitemap>
<sitemap><loc>https://example.test/sitemap-posts.xml</loc></sitemap>
</sitemapindex>"#;
        assert!(is_sitemap_index(xml));
        assert_eq!(
            parse_sitemap_index(xml),
            vec![
                "https://example.test/sitemap-pages.xml",
                "https://example.test/sitemap-posts.xml",
            ]
        );
    }

    #[test]
    fn regular_urlset_is_not_a_sitemap_index() {
        assert!(!is_sitemap_index(SAMPLE_SITEMAP));
    }
}
