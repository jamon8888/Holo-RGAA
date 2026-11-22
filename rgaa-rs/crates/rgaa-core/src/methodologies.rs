//! Official RGAA 4.1.2 test methodologies, one per RGAA test.
//!
//! Source: `RGAA/methodologies.json` in `DISIC/accessibilite.numerique.gouv.fr`,
//! the same upstream repository as `criteres.json`, embedded at a pinned commit.
//! Each entry is the official numbered verification method for one RGAA test,
//! keyed by the RGAA test key (e.g. `"1.2.3"` for criterion 1.2, test 3).
//!
//! Upstream commit: `b676fede5b8a94d55c440df98ceba016a80e58de`
//! File SHA-256: `199dcd99b3c9783465c74936e721f2d905bfbd286ff6a3197358912e2ba1b84d`
//!
//! Re-fetch with `scripts/update-methodologies.sh` and verify the hash before
//! unpinning to a newer commit.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::OnceLock;

const METHODOLOGIES_JSON: &str = include_str!("../data/rgaa-4.1.2/methodologies.json");

/// Commit pinned in the upstream `DISIC/accessibilite.numerique.gouv.fr`
/// repository whose `RGAA/methodologies.json` is embedded here.
pub const METHODOLOGIES_COMMIT: &str = "b676fede5b8a94d55c440df98ceba016a80e58de";

/// SHA-256 of the embedded `RGAA/methodologies.json`, so the pinned file can be
/// re-fetched and verified byte-for-byte before unpinning to a newer commit.
pub const METHODOLOGIES_SHA256: &str =
    "199dcd99b3c9783465c74936e721f2d905bfbd286ff6a3197358912e2ba1b84d";

/// The official method for one RGAA test: the numbered verification steps.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Methodology {
    pub test_key: String,
    pub steps: String,
}

/// All official RGAA 4.1.2 test methods, keyed by test key (`"1.1.1"`,
/// `"1.2.3"`, …). Built once per process from the embedded JSON.
pub struct RgaaMethodologies {
    map: BTreeMap<String, String>,
}

impl RgaaMethodologies {
    fn load() -> Self {
        let map: BTreeMap<String, String> =
            serde_json::from_str(METHODOLOGIES_JSON).expect("methodologies.json must parse");
        Self { map }
    }

    fn instance() -> &'static Self {
        static INSTANCE: OnceLock<RgaaMethodologies> = OnceLock::new();
        INSTANCE.get_or_init(Self::load)
    }

    /// Every test key, in ascending order.
    pub fn all() -> &'static BTreeMap<String, String> {
        &Self::instance().map
    }

    /// The official method for `test_key` (e.g. `"1.2.3"`), or `None` when the
    /// key is not an RGAA 4.1.2 test.
    #[must_use]
    pub fn get(test_key: &str) -> Option<&'static str> {
        Self::instance().map.get(test_key).map(String::as_str)
    }

    /// Every test key that carries a method, in ascending order.
    pub fn test_keys() -> Vec<String> {
        Self::instance().map.keys().cloned().collect()
    }
}

impl Methodology {
    /// Build the structured methodology for a test key from the embedded text.
    #[must_use]
    pub fn for_test(test_key: &str) -> Option<Self> {
        RgaaMethodologies::get(test_key).map(|steps| Self {
            test_key: test_key.to_string(),
            steps: steps.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::RgaaCatalog;

    /// Acceptance: every RGAA test in the embedded catalog carries an official
    /// methodology from `RGAA/methodologies.json`.
    #[test]
    fn every_catalog_test_has_a_methodology() {
        let missing: Vec<String> = RgaaCatalog::all_test_keys()
            .into_iter()
            .map(|(criterion_id, test_key)| format!("{criterion_id}.{test_key}"))
            .filter(|key| RgaaMethodologies::get(key).is_none())
            .collect();
        assert!(
            missing.is_empty(),
            "catalog tests without an official methodology: {missing:?}"
        );
    }

    /// The embedded method set is exactly the catalog's test set: no test left
    /// behind, no method for a test that does not exist.
    #[test]
    fn method_keys_match_the_catalog() {
        let mut catalog_keys: Vec<String> = RgaaCatalog::all_test_keys()
            .into_iter()
            .map(|(criterion_id, test_key)| format!("{criterion_id}.{test_key}"))
            .collect();
        catalog_keys.sort_unstable();
        let mut method_keys = RgaaMethodologies::test_keys();
        method_keys.sort_unstable();
        assert_eq!(catalog_keys, method_keys);
    }

    #[test]
    fn get_returns_known_method() {
        let method = RgaaMethodologies::get("1.2.3").expect("1.2.3 has a method");
        assert!(method.contains("object"));
        assert!(method.contains("validé"));
    }

    #[test]
    fn get_returns_none_for_unknown_key() {
        assert!(RgaaMethodologies::get("99.99.1").is_none());
        assert!(RgaaMethodologies::get("1.1").is_none());
        assert!(RgaaMethodologies::get("").is_none());
    }

    #[test]
    fn for_test_builds_structured_methodology() {
        let method = Methodology::for_test("1.1.1").expect("1.1.1 has a method");
        assert_eq!(method.test_key, "1.1.1");
        assert!(!method.steps.is_empty());
        assert!(Methodology::for_test("99.99.1").is_none());
    }
}
