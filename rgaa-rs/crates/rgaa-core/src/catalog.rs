use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

const CRITERES_JSON: &str = include_str!("../data/rgaa-4.1.2/criteres.json");
const AUTOMATABLE_JSON: &str = include_str!("../data/rgaa-4.1.2/automatable_criteres.json");

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum Automatable {
    FullyAutomatable,
    PartiallyAutomatable,
    #[default]
    NotAutomatable,
}

#[derive(Debug, Clone, Deserialize)]
struct RawRoot {
    topics: Vec<CatalogTheme>,
}

#[derive(Debug, Clone, Deserialize)]
struct AutomatableRoot {
    criteria: Vec<AutomatableCriterion>,
}

#[derive(Debug, Clone, Deserialize)]
struct AutomatableCriterion {
    criterion_id: String,
    classification: Automatable,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CatalogTheme {
    pub topic: String,
    pub number: u8,
    pub criteria: Vec<CatalogCriterionWrapper>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CatalogCriterionWrapper {
    pub criterium: CatalogCriterion,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CatalogCriterion {
    pub number: u8,
    pub title: String,
    pub tests: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub automatable: Automatable,
}

impl CatalogCriterion {
    /// Full criterion ID as `"theme.criterion"` (e.g. `"1.1"`, `"13.12"`).
    pub fn id_for_theme(&self, theme_number: u8) -> String {
        format!("{theme_number}.{}", self.number)
    }

    pub fn test_count(&self) -> usize {
        self.tests.len()
    }
}

pub struct RgaaCatalog {
    themes: Vec<CatalogTheme>,
}

impl RgaaCatalog {
    fn load() -> Self {
        let raw: RawRoot = serde_json::from_str(CRITERES_JSON).expect("criteres.json must parse");
        let automatable_root: AutomatableRoot =
            serde_json::from_str(AUTOMATABLE_JSON).expect("automatable_criteres.json must parse");
        let mut automatable_map: HashMap<String, Automatable> = HashMap::new();
        for ac in automatable_root.criteria {
            automatable_map.insert(ac.criterion_id, ac.classification);
        }
        let mut themes = raw.topics;
        for theme in &mut themes {
            for cw in &mut theme.criteria {
                let criterion_id = cw.criterium.id_for_theme(theme.number);
                cw.criterium.automatable = automatable_map
                    .remove(&criterion_id)
                    .unwrap_or_default();
            }
        }
        Self { themes }
    }

    fn instance() -> &'static Self {
        static INSTANCE: OnceLock<RgaaCatalog> = OnceLock::new();
        INSTANCE.get_or_init(Self::load)
    }

    pub fn all() -> &'static [CatalogTheme] {
        &Self::instance().themes
    }

    #[must_use]
    pub fn count() -> usize {
        Self::instance()
            .themes
            .iter()
            .map(|t| t.criteria.len())
            .sum()
    }

    pub fn by_id(criterion_id: &str) -> Option<(u8, &'static CatalogCriterion)> {
        let mut parts = criterion_id.splitn(2, '.');
        let theme: u8 = parts.next()?.parse().ok()?;
        let crit_num: u8 = parts.next()?.parse().ok()?;
        let theme_data = Self::all().iter().find(|t| t.number == theme)?;
        theme_data
            .criteria
            .iter()
            .find(|cw| cw.criterium.number == crit_num)
            .map(|cw| (theme, &cw.criterium))
    }

    pub fn title(criterion_id: &str) -> Option<&'static str> {
        Self::by_id(criterion_id).map(|(_, c)| c.title.as_str())
    }

    pub fn tests(criterion_id: &str) -> Option<&'static HashMap<String, Vec<String>>> {
        Self::by_id(criterion_id).map(|(_, c)| &c.tests)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_106_criteria() {
        assert_eq!(RgaaCatalog::count(), 106);
    }

    #[test]
    fn has_13_themes() {
        assert_eq!(RgaaCatalog::all().len(), 13);
    }

    #[test]
    fn by_id_returns_known_criteria() {
        let (_, c) = RgaaCatalog::by_id("1.1").expect("1.1 must exist");
        assert_eq!(c.number, 1);
        assert!(!c.tests.is_empty());
    }

    #[test]
    fn by_id_returns_none_for_missing() {
        assert!(RgaaCatalog::by_id("99.99").is_none());
    }

    #[test]
    fn all_ids_match_expected_format() {
        for theme in RgaaCatalog::all() {
            for cw in &theme.criteria {
                let id = cw.criterium.id_for_theme(theme.number);
                let mut parts = id.splitn(2, '.');
                let t: u8 = parts.next().unwrap().parse().unwrap();
                let c: u8 = parts.next().unwrap().parse().unwrap();
                assert_eq!(t, theme.number);
                assert_eq!(c, cw.criterium.number);
            }
        }
    }

    #[test]
    fn test_all_criteria_have_automatability() {
        let catalog = RgaaCatalog::all();
        let mut fully = 0;
        let mut partially = 0;
        let mut not_automatable = 0;
        for theme in catalog {
            for cw in &theme.criteria {
                match cw.criterium.automatable {
                    Automatable::FullyAutomatable => fully += 1,
                    Automatable::PartiallyAutomatable => partially += 1,
                    Automatable::NotAutomatable => not_automatable += 1,
                }
            }
        }
        assert_eq!(fully, 39, "expected 39 FullyAutomatable criteria");
        assert_eq!(partially, 45, "expected 45 PartiallyAutomatable criteria");
        assert_eq!(not_automatable, 22, "expected 22 NotAutomatable criteria");
        assert_eq!(fully + partially + not_automatable, 106);
    }
}
