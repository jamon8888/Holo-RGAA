use crate::catalog::{Automatable, RgaaCatalog};
use crate::types::Classification;

#[derive(Debug, Clone)]
pub struct Criterion {
    pub id: &'static str,
    pub title: String,
    pub classification: Classification,
    pub wcag_refs: &'static str,
}

/// Static classification + WCAG references for all 106 RGAA 4.1.2 criteria.
/// Titles are derived from the official `criteres.json` catalog at runtime.
const CLASSIFICATION: &[(&str, Classification, &str)] = &[
    ("1.1", Classification::Deterministe, "1.1.1"),
    ("1.2", Classification::Deterministe, "1.1.1, 4.1.2"),
    ("1.3", Classification::IaAssiste, "1.1.1, 4.1.2"),
    ("1.4", Classification::Deterministe, "1.1.1"),
    ("1.5", Classification::Deterministe, "1.1.1"),
    ("1.6", Classification::Deterministe, "1.1.1"),
    ("1.7", Classification::IaAssiste, "1.1.1"),
    ("1.8", Classification::Deterministe, "1.4.5"),
    ("1.9", Classification::Deterministe, "1.1.1, 4.1.2"),
    ("2.1", Classification::Deterministe, "1.3.1, 4.1.2"),
    ("2.2", Classification::IaAssiste, "4.1.2"),
    ("3.1", Classification::IaAssiste, "1.3.1, 1.4.1"),
    ("3.2", Classification::Deterministe, "1.4.1"),
    ("3.3", Classification::Deterministe, "1.4.3, 1.4.6"),
    ("4.1", Classification::Deterministe, "1.2.1"),
    ("4.2", Classification::IaAssiste, "1.2.1, 1.2.3"),
    ("4.3", Classification::Deterministe, "1.2.2"),
    ("4.4", Classification::IaAssiste, "1.2.2"),
    ("4.5", Classification::Deterministe, "1.2.3"),
    ("4.6", Classification::IaAssiste, "1.2.5"),
    ("4.7", Classification::Deterministe, "1.2.4"),
    ("4.8", Classification::Deterministe, "1.2.3"),
    ("4.9", Classification::IaAssiste, "1.1.1"),
    ("4.10", Classification::Deterministe, "1.2.1"),
    ("4.11", Classification::Deterministe, "2.1.1, 2.1.2"),
    ("4.12", Classification::Deterministe, "2.1.1, 2.1.2"),
    ("4.13", Classification::Deterministe, "4.1.2"),
    ("5.1", Classification::Deterministe, "1.3.1"),
    ("5.2", Classification::IaAssiste, "1.3.1"),
    ("5.3", Classification::IaAssiste, "1.3.2, 4.1.2"),
    ("5.4", Classification::Deterministe, "1.3.1"),
    ("5.5", Classification::IaAssiste, "1.3.1"),
    ("5.6", Classification::Deterministe, "1.3.1"),
    ("5.7", Classification::Deterministe, "1.3.2"),
    ("5.8", Classification::Deterministe, "1.3.1"),
    ("6.1", Classification::Deterministe, "1.3.1"),
    ("6.2", Classification::Deterministe, "1.3.1"),
    ("7.1", Classification::Deterministe, "2.1.1"),
    ("7.2", Classification::IaAssiste, "1.1.1, 4.1.2"),
    ("7.3", Classification::Deterministe, "2.1.2"),
    ("7.4", Classification::Deterministe, "3.2.1, 3.2.2"),
    ("7.5", Classification::Manuel, "4.1.3"),
    ("8.1", Classification::Deterministe, "3.1.1"),
    ("8.2", Classification::Deterministe, "3.1.1"),
    ("8.3", Classification::Deterministe, "3.1.1"),
    ("8.4", Classification::IaAssiste, "3.1.1"),
    ("8.5", Classification::Deterministe, "3.1.1"),
    ("8.6", Classification::IaAssiste, "2.4.2"),
    ("8.7", Classification::Deterministe, "2.4.2"),
    ("8.8", Classification::IaAssiste, "3.1.2"),
    ("8.9", Classification::Deterministe, "3.1.2"),
    ("8.10", Classification::Deterministe, "1.3.2"),
    ("9.1", Classification::Deterministe, "1.3.1"),
    ("9.2", Classification::IaAssiste, "1.3.1"),
    ("9.3", Classification::Deterministe, "1.3.1"),
    ("9.4", Classification::Deterministe, "1.3.1"),
    ("10.1", Classification::Deterministe, "1.3.2"),
    ("10.2", Classification::Deterministe, "1.3.2"),
    ("10.3", Classification::IaAssiste, "1.3.2, 2.4.3"),
    ("10.4", Classification::Deterministe, "1.4.4"),
    ("10.5", Classification::Deterministe, "1.4.4"),
    ("10.6", Classification::Deterministe, "1.4.4"),
    ("10.7", Classification::Deterministe, "1.4.4"),
    ("10.8", Classification::Deterministe, "1.4.4"),
    ("10.9", Classification::Deterministe, "1.3.2"),
    ("10.10", Classification::IaAssiste, "1.3.3, 1.4.1"),
    ("10.11", Classification::Deterministe, "1.3.2"),
    ("10.12", Classification::Deterministe, "2.4.6"),
    ("10.13", Classification::Deterministe, "2.4.6"),
    ("10.14", Classification::Deterministe, "1.3.1"),
    ("11.1", Classification::Deterministe, "1.3.1, 4.1.2"),
    ("11.2", Classification::IaAssiste, "2.4.6, 2.5.3, 3.3.2"),
    ("11.3", Classification::IaAssiste, "3.2.4"),
    ("11.4", Classification::Deterministe, "1.3.1, 3.3.2"),
    ("11.5", Classification::Deterministe, "3.3.2"),
    ("11.6", Classification::Deterministe, "3.3.2"),
    ("11.7", Classification::IaAssiste, "1.3.1, 3.3.2"),
    ("11.8", Classification::IaAssiste, "1.3.1"),
    ("11.9", Classification::IaAssiste, "2.5.3, 4.1.2"),
    ("11.10", Classification::IaAssiste, "3.3.1, 3.3.2"),
    ("11.11", Classification::Deterministe, "3.3.1"),
    ("11.12", Classification::Deterministe, "3.3.1"),
    ("11.13", Classification::Deterministe, "3.3.3"),
    ("12.1", Classification::Deterministe, "2.4.1"),
    ("12.2", Classification::Deterministe, "2.4.1"),
    ("12.3", Classification::IaAssiste, "2.4.5"),
    ("12.4", Classification::Deterministe, "2.4.5"),
    ("12.5", Classification::Deterministe, "2.4.2"),
    ("12.6", Classification::Deterministe, "2.4.3"),
    ("12.7", Classification::Deterministe, "2.4.4"),
    ("12.8", Classification::IaAssiste, "2.4.3"),
    ("12.9", Classification::Deterministe, "2.4.4"),
    ("12.10", Classification::Deterministe, "2.1.4"),
    ("12.11", Classification::Deterministe, "2.1.1"),
    ("13.1", Classification::Deterministe, "3.1.1"),
    ("13.2", Classification::Deterministe, "3.1.2"),
    ("13.3", Classification::Deterministe, "3.2.1"),
    ("13.4", Classification::Deterministe, "3.2.2"),
    ("13.5", Classification::Deterministe, "3.2.3"),
    ("13.6", Classification::IaAssiste, "1.1.1"),
    ("13.7", Classification::Deterministe, "2.1.1"),
    ("13.8", Classification::Deterministe, "2.2.1, 2.2.2"),
    ("13.9", Classification::Deterministe, "1.3.4"),
    ("13.10", Classification::Deterministe, "2.5.1"),
    ("13.11", Classification::Deterministe, "2.5.2"),
    ("13.12", Classification::Deterministe, "2.5.4"),
];

pub struct RgaaCriteria;

impl RgaaCriteria {
    pub fn all() -> Vec<Criterion> {
        CLASSIFICATION
            .iter()
            .map(|(id, classification, wcag_refs)| Criterion {
                id,
                title: RgaaCatalog::title(id).unwrap_or("unknown").to_string(),
                classification: *classification,
                wcag_refs,
            })
            .collect()
    }

    pub fn deterministe() -> Vec<Criterion> {
        Self::all()
            .into_iter()
            .filter(|c| c.classification == Classification::Deterministe)
            .collect()
    }

    pub fn ia_assiste() -> Vec<Criterion> {
        Self::all()
            .into_iter()
            .filter(|c| c.classification == Classification::IaAssiste)
            .collect()
    }

    pub fn partiellement_automatique() -> Vec<Criterion> {
        Self::all()
            .into_iter()
            .filter(|c| {
                RgaaCatalog::by_id(c.id)
                    .is_some_and(|(_, cat)| cat.automatable == Automatable::PartiallyAutomatable)
            })
            .collect()
    }

    pub fn count() -> usize {
        CLASSIFICATION.len()
    }

    /// Returns the classification for a given criterion ID, or None if not found.
    pub fn classification_for(id: &str) -> Option<Classification> {
        CLASSIFICATION
            .iter()
            .find(|(criterion_id, _, _)| *criterion_id == id)
            .map(|(_, classification, _)| *classification)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_matches_106() {
        assert_eq!(RgaaCriteria::count(), 106);
    }

    #[test]
    fn all_returns_106() {
        assert_eq!(RgaaCriteria::all().len(), 106);
    }

    #[test]
    fn titles_derived_from_catalog() {
        let criteria = RgaaCriteria::all();
        let c1 = criteria.iter().find(|c| c.id == "1.1").unwrap();
        assert!(!c1.title.is_empty());
        assert_ne!(c1.title, "unknown");
    }

    #[test]
    fn deterministe_ia_assiste_and_manuel_partition() {
        let all = RgaaCriteria::all();
        let det = RgaaCriteria::deterministe();
        let ia = RgaaCriteria::ia_assiste();
        let manuel = all
            .iter()
            .filter(|c| c.classification == Classification::Manuel)
            .count();
        // Deterministe + IaAssiste + Manuel = all
        assert_eq!(det.len() + ia.len() + manuel, all.len());
    }

    #[test]
    fn every_criterion_has_classification() {
        for c in RgaaCriteria::all() {
            assert!(
                c.classification == Classification::Deterministe
                    || c.classification == Classification::IaAssiste
                    || c.classification == Classification::Manuel,
                "criterion {} has unhandled classification",
                c.id
            );
        }
    }
}
