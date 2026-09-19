//! SEO / GEO / AEO rules evaluated on the same crawl as the RGAA pass.
//!
//! The catalog lives in `data/seo-geo-aeo/rules.json`; each rule maps to one
//! evaluator in [`rules`]. Output mirrors [`crate::AxeMapper`]: one
//! [`CriterionResult`] per catalog rule, keyed by rule id, in catalog order.

pub mod catalog;
mod rules;
pub mod snapshot;

pub use catalog::{Criticality, Family, SeoCatalog, SeoRule};
pub use snapshot::{BusinessProfile, Heading, Hreflang, PageSnapshot};

use indexmap::IndexMap;
use rgaa_core::{Classification, CriterionResult, CriterionStatus, RgaaError};
use rules::{Ctx, Outcome};

pub const SOURCE: &str = "seo-rules";

pub struct SeoMapper;

impl SeoMapper {
    /// Evaluate every catalog rule against a page snapshot JSON
    /// (see [`PageSnapshot::extraction_snippet`]).
    ///
    /// NAP rules report `NotApplicable` when `profile` is `None`.
    pub fn map(
        snapshot_json: &str,
        profile: Option<&BusinessProfile>,
    ) -> Result<IndexMap<String, CriterionResult>, RgaaError> {
        let snapshot = PageSnapshot::from_json(snapshot_json)?;
        Ok(Self::evaluate(&snapshot, profile))
    }

    #[must_use]
    pub fn evaluate(
        snapshot: &PageSnapshot,
        profile: Option<&BusinessProfile>,
    ) -> IndexMap<String, CriterionResult> {
        let catalog = SeoCatalog::get();
        let ctx = Ctx::new(snapshot, profile);
        let mut results = IndexMap::with_capacity(catalog.rules().len());

        for rule in catalog.rules().values() {
            let (status, violations) = match rules::evaluate(rule, &ctx) {
                Some(Outcome::Pass) => (CriterionStatus::Pass, vec![]),
                Some(Outcome::NotApplicable) => (CriterionStatus::NotApplicable, vec![]),
                Some(Outcome::Fail(v)) => (CriterionStatus::Fail, v),
                None => (CriterionStatus::NotTested, vec![]),
            };
            results.insert(
                rule.id.clone(),
                CriterionResult {
                    criterion_id: rule.id.clone(),
                    title: rule.title.clone(),
                    classification: Classification::Deterministe,
                    status,
                    violations,
                    confidence: None,
                    justification: None,
                    source: SOURCE.to_string(),
                },
            );
        }
        results
    }

    /// Criticality tier of a rule id, for ordering merged findings.
    #[must_use]
    pub fn criticality(rule_id: &str) -> Option<Criticality> {
        SeoCatalog::get().find(rule_id).map(|r| r.criticality)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EMPTY: &str = r#"{"url":"https://a.test/"}"#;

    #[test]
    fn empty_page_yields_one_result_per_catalog_rule_in_order() {
        let results = SeoMapper::map(EMPTY, None).unwrap();
        let catalog = SeoCatalog::get();
        assert_eq!(results.len(), catalog.rules().len());
        let expected: Vec<&String> = catalog.rules().keys().collect();
        let got: Vec<&String> = results.keys().collect();
        assert_eq!(got, expected);
        assert!(results.values().all(|r| r.source == SOURCE));
        assert!(results
            .values()
            .all(|r| r.status != CriterionStatus::NotTested));
    }

    #[test]
    fn empty_page_fails_presence_rules_and_na_conditional_ones() {
        let results = SeoMapper::map(EMPTY, None).unwrap();
        assert_eq!(results["SEO-META-01"].status, CriterionStatus::Fail);
        assert_eq!(
            results["SEO-META-02"].status,
            CriterionStatus::NotApplicable
        );
        assert_eq!(results["SEO-NAP-01"].status, CriterionStatus::NotApplicable);
        assert_eq!(
            results["SEO-SCHEMA-01"].status,
            CriterionStatus::NotApplicable
        );
        assert_eq!(results["SEO-META-05"].status, CriterionStatus::Pass);
    }

    #[test]
    fn violations_carry_criticality_in_impact() {
        let results = SeoMapper::map(
            r#"{"url":"https://a.test/","meta_robots":["noindex"]}"#,
            None,
        )
        .unwrap();
        let r = &results["SEO-META-05"];
        assert_eq!(r.status, CriterionStatus::Fail);
        assert_eq!(r.violations[0].impact, "P0");
        assert_eq!(SeoMapper::criticality("SEO-META-05"), Some(Criticality::P0));
    }

    #[test]
    fn invalid_snapshot_json_is_an_error() {
        assert!(matches!(
            SeoMapper::map("nope", None),
            Err(RgaaError::Seo(_))
        ));
    }

    #[test]
    fn full_page_fixture_passes_core_rules() {
        let json = r#"{
            "url": "https://example.test/plombier-lyon",
            "lang": "fr",
            "title": "Plombier à Lyon — Dépannage 24h/24 | Dupont",
            "meta_description": "Plomberie Dupont intervient en moins d'une heure sur Lyon pour fuites, chauffe-eau et canalisations bouchées. Devis gratuit.",
            "meta_robots": ["index", "follow"],
            "canonicals": ["https://example.test/plombier-lyon"],
            "headings": [
                {"level": 1, "text": "Plombier à Lyon", "next_paragraph": null},
                {"level": 2, "text": "Combien coûte un dépannage ?", "next_paragraph": "Un dépannage simple coûte entre 80 et 150 euros TTC, déplacement compris, selon l'heure d'intervention."}
            ],
            "lead_paragraph": "Plomberie Dupont est une entreprise familiale lyonnaise spécialisée dans le dépannage d'urgence. Nos plombiers certifiés interviennent 24h/24 sur toute la métropole pour les fuites, chauffe-eau, WC et canalisations bouchées, avec un devis gratuit avant toute intervention.",
            "images_total": 2,
            "images_missing_alt": 0,
            "json_ld": [
                "{\"@context\":\"https://schema.org\",\"@graph\":[{\"@type\":\"LocalBusiness\",\"name\":\"Plomberie Dupont\",\"url\":\"https://example.test\",\"address\":\"12 rue X\",\"telephone\":\"+33412345678\",\"sameAs\":[\"https://www.wikidata.org/wiki/Q1\"]},{\"@type\":\"FAQPage\",\"mainEntity\":[{\"@type\":\"Question\",\"name\":\"Combien coûte un dépannage ?\",\"acceptedAnswer\":{\"@type\":\"Answer\",\"text\":\"Entre 80 et 150 euros.\"}}]},{\"@type\":\"BreadcrumbList\",\"itemListElement\":[{\"@type\":\"ListItem\",\"position\":1,\"name\":\"Accueil\"}]}]}"
            ],
            "body_text": "Plomberie Dupont 12 rue X 69001 Lyon 04 12 34 56 78"
        }"#;
        let profile = BusinessProfile {
            name: "Plomberie Dupont".into(),
            phone: "04 12 34 56 78".into(),
            address: "12 rue X 69001 Lyon".into(),
        };
        let results = SeoMapper::map(json, Some(&profile)).unwrap();
        let failed: Vec<&String> = results
            .iter()
            .filter(|(_, r)| r.status == CriterionStatus::Fail)
            .map(|(id, _)| id)
            .collect();
        assert_eq!(failed, vec!["AEO-05"], "only speakable should fail");
    }
}
