//! Request and response types for Exa's `POST /search`.
//!
//! The recommended Exa request is the query, `type: "auto"`, and
//! `contents: { "highlights": true }` — nothing else. Every other field is
//! opt-in on purpose: decorating the request with `category`, domain filters
//! or freshness controls without a task reason is the most common Exa
//! integration mistake, so those knobs are absent from this type rather than
//! merely defaulted.

use serde::{Deserialize, Serialize};

/// Latency/quality preset. `auto` is the server default and what this crate
/// sends; the other variants exist so a caller with a stated reason can pick
/// one without hand-rolling a request.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SearchType {
    #[default]
    Auto,
    Fast,
    Instant,
    DeepLite,
    Deep,
    DeepReasoning,
}

/// Content extraction attached to each result. On `/search` these controls
/// live *inside* `contents`, never at the top level.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ContentsOptions {
    /// Token-efficient excerpts. Bare `true` is the recommended mode: it
    /// auto-selects an excerpt length per page, so there is nothing to tune.
    pub highlights: bool,
}

impl Default for ContentsOptions {
    fn default() -> Self {
        Self { highlights: true }
    }
}

/// A `POST /search` body.
///
/// Build it with [`SearchRequest::new`], which yields exactly the recommended
/// request. `num_results` is the only extra field this crate exposes, and it
/// serialises only when set.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequest {
    /// Retrieval intent only, phrased like a search box entry. Keep/drop
    /// rules do not belong here.
    pub query: String,
    #[serde(rename = "type")]
    pub search_type: SearchType,
    pub contents: ContentsOptions,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_results: Option<u32>,
}

impl SearchRequest {
    /// The recommended request: query + `type: "auto"` + bare highlights.
    #[must_use]
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            search_type: SearchType::Auto,
            contents: ContentsOptions::default(),
            num_results: None,
        }
    }

    /// Caps the number of results. The server default is 10; set this only as
    /// a deliberate product decision, not as boilerplate.
    #[must_use]
    pub fn with_num_results(mut self, n: u32) -> Self {
        self.num_results = Some(n);
        self
    }

    /// Switches the search type away from `auto`. Needs a task reason.
    #[must_use]
    pub fn with_search_type(mut self, t: SearchType) -> Self {
        self.search_type = t;
        self
    }
}

/// One retrieved page.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    pub url: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub published_date: Option<String>,
    /// Excerpts requested through `contents.highlights`. Absent when Exa
    /// could not extract content for the page.
    #[serde(default)]
    pub highlights: Vec<String>,
    #[serde(default)]
    pub highlight_scores: Vec<f64>,
}

/// A `/search` response. Fields Exa may add later are ignored rather than
/// failing the whole decode.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub results: Vec<SearchResult>,
    /// Billing breakdown; shape is provider-defined, kept opaque on purpose.
    #[serde(default)]
    pub cost_dollars: Option<serde_json::Value>,
    #[serde(default)]
    pub search_time: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn recommended_request_sends_nothing_extra() {
        let body = serde_json::to_value(SearchRequest::new("critère RGAA 1.3")).unwrap();
        assert_eq!(
            body,
            json!({
                "query": "critère RGAA 1.3",
                "type": "auto",
                "contents": { "highlights": true }
            }),
            "the recommended request must not carry numResults, category or filters"
        );
    }

    #[test]
    fn num_results_serialises_only_when_set() {
        let body = serde_json::to_value(SearchRequest::new("q").with_num_results(5)).unwrap();
        assert_eq!(body["numResults"], json!(5));
        assert_eq!(body["contents"], json!({ "highlights": true }));
    }

    #[test]
    fn highlights_stay_nested_under_contents() {
        let body = serde_json::to_value(SearchRequest::new("q")).unwrap();
        assert!(
            body.get("highlights").is_none(),
            "highlights at the top level is the /contents shape, not /search"
        );
    }

    #[test]
    fn search_types_use_the_wire_spelling() {
        let body =
            serde_json::to_value(SearchRequest::new("q").with_search_type(SearchType::DeepLite))
                .unwrap();
        assert_eq!(body["type"], json!("deep-lite"));
    }

    #[test]
    fn response_decodes_and_tolerates_unknown_fields() {
        let raw = json!({
            "requestId": "req_1",
            "results": [{
                "id": "https://accessibilite.numerique.gouv.fr/methode/criteres-et-tests/#1.3",
                "title": "Critère 1.3",
                "url": "https://accessibilite.numerique.gouv.fr/methode/criteres-et-tests/#1.3",
                "publishedDate": "2024-01-01",
                "highlights": ["L'alternative textuelle doit être pertinente."],
                "highlightScores": [0.42],
                "somethingNew": true
            }],
            "costDollars": { "total": 0.005 },
            "searchTime": 812.5
        });
        let parsed: SearchResponse = serde_json::from_value(raw).unwrap();
        assert_eq!(parsed.request_id.as_deref(), Some("req_1"));
        assert_eq!(parsed.results.len(), 1);
        assert_eq!(parsed.results[0].highlights.len(), 1);
        assert!(parsed.results[0].author.is_none());
    }

    #[test]
    fn result_without_content_still_decodes() {
        let parsed: SearchResult =
            serde_json::from_value(json!({ "url": "https://example.org" })).unwrap();
        assert!(parsed.highlights.is_empty());
        assert!(parsed.title.is_none());
    }
}
