use std::collections::HashMap;
use std::sync::OnceLock;
use serde::Deserialize;

const CRITERES_JSON: &str = include_str!("../data/rgaa-4.1.2/criteres.json");

#[derive(Debug, Clone, Deserialize)]
struct RawRoot {
    topics: Vec<CatalogTheme>,
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
        let raw: RawRoot =
            serde_json::from_str(CRITERES_JSON).expect("criteres.json must parse");
        Self { themes: raw.topics }
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
}
