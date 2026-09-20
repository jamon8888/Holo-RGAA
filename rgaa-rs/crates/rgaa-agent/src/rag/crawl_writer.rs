//! Writes structured deep-extraction evidence into the per-audit crawl
//! index ([`schema::RAG_CRAWL_TABLE`](super::schema::RAG_CRAWL_TABLE)).
//!
//! Takes a [`PageContext`] — never a raw HTML string, so the crawler's
//! large-scale HTML is excluded from indexing at the type level, not by
//! convention. [`PageContext`] is exactly the deep-extraction output
//! (headings, images, forms, links, media — never markup); there is no
//! HTML field to accidentally index.

use super::store::{CrawlRecord, RagStore};
use super::EmbedQuery;
use crate::error::AgentError;
use rgaa_holo::PageContext;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Default expiry for crawl evidence: long enough to outlive one audit run
/// comfortably, short enough that a stale audit's evidence is reliably
/// purged before the next one starts (see
/// [`RagStore::purge_expired_crawl`]).
pub const DEFAULT_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// Builds and writes [`CrawlRecord`]s for one page's [`PageContext`] into
/// [`RagStore`]'s crawl table.
pub struct CrawlWriter<'a, E> {
    embedder: &'a E,
    ttl: Duration,
}

impl<'a, E: EmbedQuery> CrawlWriter<'a, E> {
    /// Uses [`DEFAULT_TTL`].
    pub fn new(embedder: &'a E) -> Self {
        Self::with_ttl(embedder, DEFAULT_TTL)
    }

    pub fn with_ttl(embedder: &'a E, ttl: Duration) -> Self {
        Self { embedder, ttl }
    }

    /// Builds one [`CrawlRecord`] per non-empty structured-evidence chunk
    /// of `context`, embedding each chunk with `embedder`. `url` is
    /// normalized (scheme/host lowercased, default port and trailing
    /// slash/fragment stripped) before being stored.
    ///
    /// # Errors
    /// Returns [`AgentError::Embedding`] if embedding any chunk fails.
    pub async fn build_records(
        &self,
        url: &str,
        context: &PageContext,
        captured_at: &str,
    ) -> Result<Vec<CrawlRecord>, AgentError> {
        let normalized_url = normalize_url(url);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let expires_at = (now + self.ttl).as_secs() as i64;

        let mut records = Vec::new();
        for (label, content) in evidence_chunks(context) {
            if content.trim().is_empty() {
                continue;
            }
            let embedding = self
                .embedder
                .embed_query(&content)
                .await
                .map_err(AgentError::Embedding)?;
            let evidence_hash = content_fingerprint(&content);
            records.push(CrawlRecord {
                id: format!("{normalized_url}#{label}-{evidence_hash}"),
                url: normalized_url.clone(),
                captured_at: captured_at.to_string(),
                evidence_hash,
                content,
                expires_at: Some(expires_at),
                embedding,
            });
        }
        Ok(records)
    }

    /// Builds and writes every record for `context` into `store`. Returns
    /// the number of rows written.
    ///
    /// # Errors
    /// Returns [`AgentError`] if embedding or the LanceDB write fails.
    pub async fn write(
        &self,
        store: &RagStore,
        url: &str,
        context: &PageContext,
        captured_at: &str,
    ) -> Result<usize, AgentError> {
        let records = self.build_records(url, context, captured_at).await?;
        let count = records.len();
        store.insert_crawl(records).await?;
        Ok(count)
    }
}

/// Splits `context` into labeled structured-evidence chunks, one per
/// section, so retrieval can return the specific piece of evidence a
/// verdict relies on instead of one page-sized blob. Never touches raw
/// HTML — `context` has no such field.
fn evidence_chunks(context: &PageContext) -> Vec<(&'static str, String)> {
    let mut chunks = Vec::new();

    let mut page_meta = String::new();
    if let Some(title) = &context.title {
        page_meta.push_str(&format!("Titre de la page: {title}\n"));
    }
    if let Some(lang) = &context.lang {
        page_meta.push_str(&format!("Langue: {lang}\n"));
    }
    if !page_meta.is_empty() {
        chunks.push(("page", page_meta));
    }

    if !context.headings.is_empty() {
        let text = context
            .headings
            .iter()
            .map(|h| format!("h{}: {}", h.level, h.text))
            .collect::<Vec<_>>()
            .join("\n");
        chunks.push(("headings", text));
    }

    for (i, image) in context.images.iter().enumerate() {
        let text = format!(
            "Image {}: alt={:?} has_alt={} decorative={}",
            image.src, image.alt, image.has_alt, image.is_decorative
        );
        chunks.push(("image", format!("{i}\n{text}")));
    }

    for (i, form) in context.forms.iter().enumerate() {
        let inputs = form
            .inputs
            .iter()
            .map(|g| {
                format!(
                    "type={} has_label={} aria_label={:?} placeholder={:?}",
                    g.input_type, g.has_label, g.aria_label, g.placeholder
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        chunks.push((
            "form",
            format!(
                "{i}\nFormulaire id={:?} has_labels={} has_submit={}\n{inputs}",
                form.id, form.has_labels, form.has_submit
            ),
        ));
    }

    if !context.links.is_empty() {
        let text = context
            .links
            .iter()
            .filter(|l| l.is_empty || !l.has_text)
            .map(|l| format!("href={} text={:?} empty={}", l.href, l.text, l.is_empty))
            .collect::<Vec<_>>()
            .join("\n");
        if !text.is_empty() {
            chunks.push(("links", text));
        }
    }

    if !context.media.is_empty() {
        let text = context
            .media
            .iter()
            .enumerate()
            .map(|(i, m)| format!("{i}: {m:?}"))
            .collect::<Vec<_>>()
            .join("\n");
        chunks.push(("media", text));
    }

    chunks
}

/// Normalizes `url`: lowercases scheme and host, drops a default port
/// (80/443), and strips a trailing `/` (except for the root path) and any
/// fragment. Not a full RFC 3986 normalizer — just enough that
/// `https://Example.com/Path/` and `https://example.com/Path` (both
/// commonly seen from a crawler) collapse to the same indexed URL.
fn normalize_url(url: &str) -> String {
    let Ok(mut parsed) = url::Url::parse(url) else {
        return url.to_string();
    };
    parsed.set_fragment(None);
    let scheme = parsed.scheme().to_ascii_lowercase();
    let default_port = match scheme.as_str() {
        "https" => Some(443),
        "http" => Some(80),
        _ => None,
    };
    if parsed.port() == default_port {
        let _ = parsed.set_port(None);
    }
    let _ = parsed.set_scheme(&scheme);
    if let Some(host) = parsed.host_str() {
        let host = host.to_ascii_lowercase();
        let _ = parsed.set_host(Some(&host));
    }
    let path = parsed.path().to_string();
    if path.len() > 1 && path.ends_with('/') {
        parsed.set_path(path.trim_end_matches('/'));
    }
    parsed.to_string()
}

/// A stable FNV-1a content fingerprint, same shape as
/// [`rgaa_core::FindingFingerprint`] — deterministic and dependency-free.
fn content_fingerprint(content: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in content.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("sha1n-v1-{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgaa_holo::prompts::{FormGroupInfo, FormInfo, HeadingInfo, ImageInfo, LinkInfo};
    use tempfile::TempDir;

    struct FakeEmbedder;

    impl EmbedQuery for FakeEmbedder {
        async fn embed_query(&self, text: &str) -> Result<Vec<f32>, String> {
            let bytes = text.as_bytes();
            Ok((0..crate::vector::schema::EMBEDDING_DIM)
                .map(|i| {
                    let b = bytes.get(i % bytes.len().max(1)).copied().unwrap_or(0);
                    (b as f32 / 255.0) - 0.5
                })
                .collect())
        }
    }

    fn sample_context() -> PageContext {
        PageContext {
            title: Some("Accueil".into()),
            lang: Some("fr".into()),
            headings: vec![HeadingInfo {
                level: 1,
                text: "Bienvenue".into(),
            }],
            images: vec![ImageInfo {
                src: "/logo.png".into(),
                alt: None,
                has_alt: false,
                is_decorative: false,
            }],
            iframes: vec![],
            links: vec![LinkInfo {
                href: "/en-savoir-plus".into(),
                text: String::new(),
                has_text: false,
                is_empty: true,
            }],
            forms: vec![FormInfo {
                id: Some("contact".into()),
                has_labels: false,
                has_submit: true,
                inputs: vec![FormGroupInfo {
                    input_type: "email".into(),
                    has_label: false,
                    aria_label: None,
                    placeholder: Some("Email".into()),
                }],
            }],
            media: vec![],
            navigation: vec![],
        }
    }

    #[test]
    fn normalize_url_lowercases_and_strips_default_port_and_trailing_slash() {
        assert_eq!(
            normalize_url("HTTPS://Example.COM:443/Contact/"),
            "https://example.com/Contact"
        );
        assert_eq!(
            normalize_url("https://example.com/"),
            "https://example.com/"
        );
        assert_eq!(
            normalize_url("https://example.com/a#section"),
            "https://example.com/a"
        );
    }

    #[test]
    fn evidence_chunks_never_contain_raw_html() {
        let context = sample_context();
        for (_, chunk) in evidence_chunks(&context) {
            assert!(!chunk.contains('<'), "chunk must not carry markup: {chunk}");
        }
    }

    #[tokio::test]
    async fn build_records_produces_one_record_per_non_empty_chunk() {
        let writer = CrawlWriter::new(&FakeEmbedder);
        let records = writer
            .build_records(
                "https://Example.com/Contact/",
                &sample_context(),
                "2025-01-01T00:00:00Z",
            )
            .await
            .unwrap();

        assert!(!records.is_empty());
        for record in &records {
            assert_eq!(record.url, "https://example.com/Contact");
            assert_eq!(record.embedding.len(), crate::vector::schema::EMBEDDING_DIM);
            assert!(record.expires_at.is_some());
        }
    }

    #[tokio::test]
    async fn write_inserts_into_crawl_index_and_is_queryable() {
        use super::super::tools::{CrawlSearchArgs, CrawlSearchTool};
        use rig_core::tool::PortableTool;

        let dir = TempDir::new().unwrap();
        let store = RagStore::open(dir.path().to_str().unwrap()).await.unwrap();
        let writer = CrawlWriter::new(&FakeEmbedder);
        let written = writer
            .write(
                &store,
                "https://example.test/contact",
                &sample_context(),
                "2025-01-01T00:00:00Z",
            )
            .await
            .unwrap();
        assert!(written > 0);

        let tool = CrawlSearchTool::new(store.reader(), FakeEmbedder);
        let output = tool
            .call(CrawlSearchArgs {
                query: "Formulaire id contact".into(),
                k: Some(10),
                url: None,
            })
            .await
            .unwrap();
        assert_eq!(output.stats.count, written.min(10));
    }

    #[tokio::test]
    async fn purge_removes_only_expired_rows() {
        let dir = TempDir::new().unwrap();
        let store = RagStore::open(dir.path().to_str().unwrap()).await.unwrap();

        // One already-expired row, one far-future row.
        store
            .insert_crawl(vec![
                CrawlRecord {
                    id: "expired".into(),
                    url: "https://example.test/a".into(),
                    captured_at: "2020-01-01T00:00:00Z".into(),
                    evidence_hash: "sha1n-v1-0".into(),
                    content: "stale evidence".into(),
                    expires_at: Some(1), // 1970-01-01T00:00:01Z, long expired
                    embedding: FakeEmbedder.embed_query("stale evidence").await.unwrap(),
                },
                CrawlRecord {
                    id: "fresh".into(),
                    url: "https://example.test/b".into(),
                    captured_at: "2099-01-01T00:00:00Z".into(),
                    evidence_hash: "sha1n-v1-1".into(),
                    content: "fresh evidence".into(),
                    expires_at: Some(32_503_680_000), // year ~3000
                    embedding: FakeEmbedder.embed_query("fresh evidence").await.unwrap(),
                },
            ])
            .await
            .unwrap();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        store.purge_expired_crawl(now).await.unwrap();

        let reader = store.reader();
        let remaining = reader
            .query_crawl(
                &FakeEmbedder.embed_query("evidence").await.unwrap(),
                10,
                None,
            )
            .await
            .unwrap();
        assert_eq!(remaining.documents.len(), 1);
        assert_eq!(remaining.documents[0].url, "https://example.test/b");
    }
}
