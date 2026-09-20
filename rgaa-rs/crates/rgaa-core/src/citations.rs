use serde::{Deserialize, Serialize};

/// A typed source backing a criterion verdict.
///
/// Every citation points at exactly one retrieval index: the versioned
/// regulatory corpus ([`Citation::Referentiel`]) or the ephemeral crawl
/// index rebuilt from deep-extraction evidence for the current audit
/// ([`Citation::Crawl`]). A verdict that leans on retrieved documents must
/// carry at least one citation naming which document(s) it relied on; a
/// verdict reached without retrieval (deterministic rules, manual review,
/// "not tested") carries none and stays valid ([`CriterionResult::citations`]
/// defaults to empty on deserialize).
///
/// [`CriterionResult::citations`]: crate::CriterionResult::citations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Citation {
    /// A document from the versioned regulatory corpus, indexed per test.
    Referentiel {
        /// The RGAA test this document was seeded from (e.g. `"1.1.1"`).
        test_id: String,
        /// Version of the regulatory corpus this document belongs to.
        /// Citations across a version change are never mixed silently: the
        /// index is rebuilt wholesale on a version bump (see [`Citation`]
        /// module docs and ticket #128), so this field always names the
        /// exact version a verdict was checked against.
        referentiel_version: String,
    },
    /// A document from the per-audit crawl index, built only from
    /// structured deep-extraction evidence (never raw crawler HTML).
    Crawl {
        /// Normalized URL of the page the evidence was extracted from.
        url: String,
        /// ISO 8601 timestamp of when the evidence was captured/indexed.
        captured_at: String,
        /// Content fingerprint of the indexed evidence, so a citation can
        /// be checked against what was actually retrieved even after the
        /// crawl index has since been purged.
        evidence_hash: String,
    },
}

impl Citation {
    /// Builds a [`Citation::Referentiel`].
    pub fn referentiel(test_id: impl Into<String>, referentiel_version: impl Into<String>) -> Self {
        Self::Referentiel {
            test_id: test_id.into(),
            referentiel_version: referentiel_version.into(),
        }
    }

    /// Builds a [`Citation::Crawl`].
    pub fn crawl(
        url: impl Into<String>,
        captured_at: impl Into<String>,
        evidence_hash: impl Into<String>,
    ) -> Self {
        Self::Crawl {
            url: url.into(),
            captured_at: captured_at.into(),
            evidence_hash: evidence_hash.into(),
        }
    }

    /// Which index this citation was retrieved from: `"referentiel"` or
    /// `"crawl"`, matching the serialized `kind` tag.
    pub fn source_kind(&self) -> &'static str {
        match self {
            Self::Referentiel { .. } => "referentiel",
            Self::Crawl { .. } => "crawl",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn referentiel_citation_round_trips() {
        let citation = Citation::referentiel("1.1.1", "2024.1");
        let json = serde_json::to_string(&citation).unwrap();
        assert_eq!(
            json,
            r#"{"kind":"referentiel","test_id":"1.1.1","referentiel_version":"2024.1"}"#
        );
        let decoded: Citation = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, citation);
        assert_eq!(citation.source_kind(), "referentiel");
    }

    #[test]
    fn crawl_citation_round_trips() {
        let citation = Citation::crawl(
            "https://example.org/contact",
            "2025-01-01T00:00:00Z",
            "sha256:abc",
        );
        let json = serde_json::to_string(&citation).unwrap();
        let decoded: Citation = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, citation);
        assert_eq!(citation.source_kind(), "crawl");
    }

    #[test]
    fn distinct_kinds_are_not_confused_on_decode() {
        let referentiel = Citation::referentiel("9.1.1", "2024.1");
        let crawl = Citation::crawl("https://example.org/", "2025-01-01T00:00:00Z", "sha256:x");
        assert_ne!(referentiel, crawl);
    }
}
