use indexmap::IndexMap;
use rgaa_core::RgaaError;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

const RULES_JSON: &str = include_str!("../../data/seo-geo-aeo/rules.json");

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Criticality {
    P0,
    P1,
    P2,
    P3,
}

impl Criticality {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::P0 => "P0",
            Self::P1 => "P1",
            Self::P2 => "P2",
            Self::P3 => "P3",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Family {
    Seo,
    Geo,
    Aeo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeoRule {
    pub id: String,
    pub family: Family,
    pub group: String,
    pub title: String,
    pub description: String,
    pub criticality: Criticality,
    #[serde(default)]
    pub rgaa_overlap: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Provenance {
    pub source: String,
    pub validated_by: String,
    pub validated_at: String,
    pub notes: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RawCatalog {
    schema_version: String,
    provenance: Provenance,
    rules: Vec<SeoRule>,
}

#[derive(Debug)]
pub struct SeoCatalog {
    pub schema_version: String,
    pub provenance: Provenance,
    rules: IndexMap<String, SeoRule>,
}

impl SeoCatalog {
    /// Parsed once per process from the embedded `rules.json`.
    ///
    /// # Panics
    ///
    /// Panics if the embedded catalog is malformed — a build-time invariant,
    /// covered by `embedded_catalog_parses` below.
    #[must_use]
    pub fn get() -> &'static SeoCatalog {
        static CATALOG: OnceLock<SeoCatalog> = OnceLock::new();
        CATALOG.get_or_init(|| {
            Self::parse(RULES_JSON).expect("embedded seo-geo-aeo rules.json is valid")
        })
    }

    pub fn parse(json: &str) -> Result<Self, RgaaError> {
        let raw: RawCatalog = serde_json::from_str(json).map_err(|e| {
            RgaaError::Seo(format!("Failed to parse seo-geo-aeo rules catalog: {e}"))
        })?;
        let mut rules = IndexMap::with_capacity(raw.rules.len());
        for rule in raw.rules {
            if rules.insert(rule.id.clone(), rule).is_some() {
                return Err(RgaaError::Seo(
                    "duplicate rule id in seo-geo-aeo catalog".into(),
                ));
            }
        }
        Ok(Self {
            schema_version: raw.schema_version,
            provenance: raw.provenance,
            rules,
        })
    }

    #[must_use]
    pub fn rules(&self) -> &IndexMap<String, SeoRule> {
        &self.rules
    }

    #[must_use]
    pub fn find(&self, id: &str) -> Option<&SeoRule> {
        self.rules.get(id)
    }

    pub fn by_family(&self, family: Family) -> impl Iterator<Item = &SeoRule> {
        self.rules.values().filter(move |r| r.family == family)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_catalog_parses() {
        let catalog = SeoCatalog::get();
        assert_eq!(catalog.schema_version, "1.0");
        assert!(catalog.rules().len() >= 30);
    }

    #[test]
    fn every_family_is_represented() {
        let catalog = SeoCatalog::get();
        for family in [Family::Seo, Family::Geo, Family::Aeo] {
            assert!(
                catalog.by_family(family).next().is_some(),
                "{family:?} has no rules"
            );
        }
    }

    #[test]
    fn ids_follow_family_prefix() {
        for rule in SeoCatalog::get().rules().values() {
            let prefix = match rule.family {
                Family::Seo => "SEO-",
                Family::Geo => "GEO-",
                Family::Aeo => "AEO-",
            };
            assert!(
                rule.id.starts_with(prefix),
                "{} should start with {prefix}",
                rule.id
            );
        }
    }

    #[test]
    fn rgaa_overlap_ids_look_like_criteria() {
        for rule in SeoCatalog::get().rules().values() {
            for id in &rule.rgaa_overlap {
                assert!(
                    id.split_once('.')
                        .is_some_and(|(a, b)| a.parse::<u8>().is_ok() && b.parse::<u8>().is_ok()),
                    "{}: bad RGAA overlap id {id}",
                    rule.id
                );
            }
        }
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let json = r#"{"schema_version":"1.0","provenance":{"source":"","validated_by":"","validated_at":"","notes":""},
            "rules":[
              {"id":"SEO-X","family":"seo","group":"g","title":"t","description":"d","criticality":"P1"},
              {"id":"SEO-X","family":"seo","group":"g","title":"t","description":"d","criticality":"P1"}
            ]}"#;
        assert!(SeoCatalog::parse(json).is_err());
    }

    #[test]
    fn criticality_orders_p0_first() {
        assert!(Criticality::P0 < Criticality::P1);
        assert!(Criticality::P2 < Criticality::P3);
    }
}
