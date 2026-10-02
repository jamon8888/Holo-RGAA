use crate::catalog::{Automatable, RgaaCatalog};
use crate::types::Classification;
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct Criterion {
    pub id: &'static str,
    pub title: String,
    pub classification: Classification,
    pub wcag_refs: &'static str,
}

static CRITERIA_CACHE: OnceLock<Vec<Criterion>> = OnceLock::new();

/// Static classification + WCAG references for all 106 RGAA 4.1.2 criteria.
/// Titles are derived from the official `criteres.json` catalog at runtime.
const CLASSIFICATION: &[(&str, Classification, &str)] = &[
    ("1.1", Classification::Deterministe, "1.1.1"),
    ("1.2", Classification::Deterministe, "1.1.1, 4.1.2"),
    ("1.3", Classification::IaAssiste, "1.1.1, 4.1.2"),
    ("1.4", Classification::IaAssiste, "1.1.1"),
    ("1.5", Classification::Deterministe, "1.1.1"),
    ("1.6", Classification::Deterministe, "1.1.1"),
    ("1.7", Classification::IaAssiste, "1.1.1"),
    ("1.8", Classification::Deterministe, "1.4.5"),
    ("1.9", Classification::Deterministe, "1.1.1, 4.1.2"),
    // Deterministe, not IaAssiste: the criterion's single test is "every frame has a
    // frame title", which axe-core's frame-title decides outright (#201 AC4).
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
    ("4.7", Classification::IaAssiste, "1.2.4"),
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
    ("8.10", Classification::IaAssiste, "1.3.2"),
    ("9.1", Classification::IaAssiste, "1.3.1"),
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
    /// Built once per process and shared read-only — cloning happens only for the
    /// filtered subsets below, never for the full 106-item catalog on every call.
    pub fn all() -> &'static [Criterion] {
        CRITERIA_CACHE.get_or_init(|| {
            CLASSIFICATION
                .iter()
                .map(|(id, classification, wcag_refs)| Criterion {
                    id,
                    title: RgaaCatalog::title(id).unwrap_or("unknown").to_string(),
                    classification: *classification,
                    wcag_refs,
                })
                .collect()
        })
    }

    /// Filtered once per process, in `all()`'s order, and shared read-only.
    pub fn deterministe() -> &'static [Criterion] {
        static SUBSET: OnceLock<Vec<Criterion>> = OnceLock::new();
        SUBSET.get_or_init(|| Self::subset(|c| c.classification == Classification::Deterministe))
    }

    /// Filtered once per process, in `all()`'s order, and shared read-only.
    pub fn ia_assiste() -> &'static [Criterion] {
        static SUBSET: OnceLock<Vec<Criterion>> = OnceLock::new();
        SUBSET.get_or_init(|| Self::subset(|c| c.classification == Classification::IaAssiste))
    }

    /// Filtered once per process, in `all()`'s order, and shared read-only — the
    /// catalog lookup behind the filter no longer runs 106 times per audit.
    pub fn partiellement_automatique() -> &'static [Criterion] {
        static SUBSET: OnceLock<Vec<Criterion>> = OnceLock::new();
        SUBSET.get_or_init(|| {
            Self::subset(|c| {
                RgaaCatalog::by_id(c.id)
                    .is_some_and(|(_, cat)| cat.automatable == Automatable::PartiallyAutomatable)
            })
        })
    }

    fn subset(keep: impl Fn(&Criterion) -> bool) -> Vec<Criterion> {
        Self::all().iter().filter(|c| keep(c)).cloned().collect()
    }

    pub fn count() -> usize {
        CLASSIFICATION.len()
    }

    /// Criterion id → its index in [`Self::all`], built once per process so a lookup
    /// by id is a single hash probe instead of a linear scan of the 106 criteria.
    fn id_index() -> &'static HashMap<&'static str, usize> {
        static INDEX: OnceLock<HashMap<&'static str, usize>> = OnceLock::new();
        INDEX.get_or_init(|| {
            Self::all()
                .iter()
                .enumerate()
                .map(|(i, c)| (c.id, i))
                .collect()
        })
    }

    /// The criterion with this exact id, or `None`. Matches on the literal id, as a
    /// scan of [`Self::all`] did: unlike [`RgaaCatalog::by_id`], `"01.1"` is not `"1.1"`.
    #[must_use]
    pub fn find(id: &str) -> Option<&'static Criterion> {
        Self::id_index().get(id).map(|&i| &Self::all()[i])
    }

    /// Returns the classification for a given criterion ID, or None if not found.
    pub fn classification_for(id: &str) -> Option<Classification> {
        Self::find(id).map(|c| c.classification)
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

    /// The index must answer exactly what the linear scan it replaced answered,
    /// for every criterion and for ids the catalog does not hold (#43).
    #[test]
    fn find_agrees_with_a_linear_scan() {
        for probe in RgaaCriteria::all()
            .iter()
            .map(|c| c.id)
            .chain(["01.1", "1.01", "99.99", "", "1", "abc"])
        {
            let scanned = RgaaCriteria::all().iter().find(|c| c.id == probe);
            let found = RgaaCriteria::find(probe);
            assert_eq!(scanned.map(|c| c.id), found.map(|c| c.id), "id {probe}");
            assert_eq!(
                RgaaCriteria::classification_for(probe),
                scanned.map(|c| c.classification),
                "classification for {probe}"
            );
        }
    }

    /// Repeated calls hand out the same build, not a fresh one per call, and the
    /// shared list still converts to the owned `Vec` the agent's API consumes.
    #[test]
    fn lists_are_built_once_and_shared() {
        let owned: Vec<Criterion> = RgaaCriteria::ia_assiste().to_vec();
        assert_eq!(owned.len(), RgaaCriteria::ia_assiste().len());
        assert!(std::ptr::eq(RgaaCriteria::all(), RgaaCriteria::all()));
        assert!(std::ptr::eq(
            RgaaCriteria::ia_assiste(),
            RgaaCriteria::ia_assiste()
        ));
        assert!(std::ptr::eq(
            RgaaCriteria::deterministe(),
            RgaaCriteria::deterministe()
        ));
        assert!(std::ptr::eq(
            RgaaCriteria::partiellement_automatique(),
            RgaaCriteria::partiellement_automatique()
        ));
    }
}
