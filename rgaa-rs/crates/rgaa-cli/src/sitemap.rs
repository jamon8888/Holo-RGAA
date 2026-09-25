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
//!
//! The sitemap document itself is untrusted content (it comes from the site
//! being audited), so this module treats it defensively: bounded response
//! size, bounded child-sitemap fan-out, and every fetch — including
//! redirects — restricted to the audited site's own origin.

use std::time::Duration;

use futures::StreamExt;

use crate::CliError;

/// Applies to both the top-level sitemap fetch and each sitemap-index child
/// fetch — the async default `reqwest` client has no timeout at all, so an
/// unresponsive host would otherwise hang the audit indefinitely.
const FETCH_TIMEOUT: Duration = Duration::from_secs(15);

/// A `<sitemapindex>` can reference arbitrarily many child sitemaps; this
/// bounds how many we'll follow so a malicious or oversized index can't turn
/// page discovery into an unbounded fetch loop.
const MAX_SITEMAP_INDEX_CHILDREN: usize = 10;

/// Caps the buffered size of any single sitemap response. Generous for any
/// legitimate sitemap, and a hard stop against a misbehaving or malicious
/// host streaming unbounded data at the client (CWE-400, uncontrolled
/// resource consumption) — `.text()`/`.bytes()` would otherwise buffer the
/// entire body regardless of size.
const MAX_SITEMAP_BYTES: usize = 10 * 1024 * 1024;

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
    // Every request this module makes — the initial fetch and every
    // redirect hop, not just child-sitemap fetches — is restricted to the
    // audited site's origin. Without this, a redirect (which reqwest
    // follows by default) or a crafted sitemap-index child could induce a
    // request to an arbitrary internal address (CWE-918, SSRF).
    let target_origin = target_url.to_string();
    let client = reqwest::Client::builder()
        .timeout(FETCH_TIMEOUT)
        .redirect(reqwest::redirect::Policy::custom(move |attempt| {
            if same_origin(attempt.url().as_str(), &target_origin) {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()
        .map_err(|e| CliError::execution(format!("failed to build HTTP client: {e}")))?;

    let entries = fetch_sitemap_entries(&client, sitemap_url, target_url).await?;
    Ok(select_pillar_pages(entries, target_url, limit))
}

/// Fetches `url`'s body, capped at [`MAX_SITEMAP_BYTES`] — streamed rather
/// than buffered all at once via `.text()`/`.bytes()`, so an oversized
/// response is rejected instead of exhausting memory.
async fn fetch_text(client: &reqwest::Client, url: &str) -> Result<String, CliError> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| CliError::execution(format!("failed to fetch {url}: {e}")))?;

    let mut stream = response.bytes_stream();
    let mut buf = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| CliError::execution(format!("failed to read {url}: {e}")))?;
        if buf.len() + chunk.len() > MAX_SITEMAP_BYTES {
            return Err(CliError::execution(format!(
                "sitemap {url} exceeds the {MAX_SITEMAP_BYTES}-byte limit"
            )));
        }
        buf.extend_from_slice(&chunk);
    }

    String::from_utf8(buf)
        .map_err(|e| CliError::execution(format!("sitemap {url} is not valid UTF-8: {e}")))
}

async fn fetch_sitemap_entries(
    client: &reqwest::Client,
    sitemap_url: &str,
    target_url: &str,
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
        // The index's own content is untrusted (it's the audited site's
        // content) — reject any child sitemap that isn't on the target
        // site's origin before ever fetching it (CWE-918, SSRF).
        if !same_origin(&child_url, target_url) {
            continue;
        }
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

/// Decodes XML character references: the 5 predefined named entities
/// (`&lt;`, `&gt;`, `&quot;`, `&apos;`, `&amp;`) and numeric references
/// (`&#38;`, `&#x26;`). A single left-to-right pass — rather than sequential
/// whole-string `.replace()` calls — so a decoded reference is never
/// re-scanned as if it were part of the original markup (e.g. a literal
/// `&amp;#38;` must decode to the text `&#38;`, not all the way down to
/// `&`). An unrecognized or malformed reference is left as-is rather than
/// dropped.
fn decode_xml_entities(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut rest = s;

    while let Some(amp_pos) = rest.find('&') {
        result.push_str(&rest[..amp_pos]);
        let after_amp = &rest[amp_pos + 1..];

        match after_amp.find(';').and_then(|semi_pos| {
            decode_entity(&after_amp[..semi_pos]).map(|decoded| (decoded, semi_pos))
        }) {
            Some((decoded, semi_pos)) => {
                result.push(decoded);
                rest = &after_amp[semi_pos + 1..];
            }
            None => {
                // Not a recognized reference — keep the '&' literally and
                // keep scanning from just past it.
                result.push('&');
                rest = after_amp;
            }
        }
    }
    result.push_str(rest);
    result
}

fn decode_entity(entity: &str) -> Option<char> {
    match entity {
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "amp" => Some('&'),
        _ => {
            let digits = entity
                .strip_prefix("#x")
                .or_else(|| entity.strip_prefix("#X"))
                .map(|hex| u32::from_str_radix(hex, 16))
                .or_else(|| entity.strip_prefix('#').map(|dec| dec.parse::<u32>()))?
                .ok()?;
            char::from_u32(digits)
        }
    }
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
    fn decodes_numeric_decimal_and_hex_references() {
        assert_eq!(decode_xml_entities("a&#38;b"), "a&b");
        assert_eq!(decode_xml_entities("a&#x26;b"), "a&b");
        assert_eq!(decode_xml_entities("a&#X26;b"), "a&b");
    }

    #[test]
    fn does_not_double_decode_escaped_named_entities() {
        // The literal text "&lt;" escaped once more, as it would appear in
        // a well-formed sitemap wanting to convey literal "&lt;" text.
        assert_eq!(decode_xml_entities("&amp;lt;"), "&lt;");
    }

    #[test]
    fn leaves_unrecognized_references_untouched() {
        assert_eq!(decode_xml_entities("a & b"), "a & b");
        assert_eq!(decode_xml_entities("a &bogus; b"), "a &bogus; b");
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
