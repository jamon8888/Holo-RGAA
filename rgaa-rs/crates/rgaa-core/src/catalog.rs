use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

const CRITERES_JSON: &str = include_str!("../data/rgaa-4.1.2/criteres.json");
const AUTOMATABLE_JSON: &str = include_str!("../data/rgaa-4.1.2/automatable_criteres.json");
const AXE_MAPPING_JSON: &str = include_str!("../data/rgaa-4.1.2/axe_mapping.json");

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
struct AxeMappingEntry {
    criterion_id: String,
    axe_rules: Vec<String>,
    #[serde(default)]
    coverage: AxeCoverage,
    provenance: AxeProvenance,
}

/// How much of a criterion its mapped axe-core rules actually decide.
///
/// This is what stops a rule that covers one of a criterion's fifteen tests from
/// asserting the whole criterion conforms. `meta-refresh` finding nothing says
/// nothing about the other fourteen tests of 13.1, so 13.1 is [`Partial`] and axe
/// may only *fail* it, never pass it (#201). [`Complete`] is reserved for criteria
/// whose mapped rules decide every test, where axe's silence really is evidence.
///
/// [`Partial`]: AxeCoverage::Partial
/// [`Complete`]: AxeCoverage::Complete
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AxeCoverage {
    /// The mapped rules decide every test of the criterion: axe may assert `Pass`.
    Complete,
    /// The mapped rules detect some violations, but their silence proves nothing:
    /// axe may assert `Fail` only. The default, so an entry that forgets to declare
    /// its coverage cannot accidentally claim conformance.
    #[default]
    Partial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxeProvenance {
    pub source: String,
    pub validated_by: String,
    pub validated_at: String,
    pub notes: String,
}

#[derive(Debug, Clone, Deserialize)]
struct AutomatableCriterion {
    criterion_id: String,
    classification: Automatable,
    #[serde(default)]
    automatable_test_count: usize,
    #[serde(default)]
    total_test_count: usize,
    #[serde(default)]
    test_keys: Vec<String>,
}

/// The catalog's per-test accounting for one criterion: how many of its RGAA tests a
/// mechanism could decide, out of how many, and which ones.
///
/// Shipped in `automatable_criteres.json` since the catalog was built and, until #203,
/// consumed by nothing but the pipeline's bulk `NeedsReview` branch. Exposed here
/// because the test-level reduction #203 recommends needs it, and because its coherence
/// is worth asserting: five rows currently contradict their own
/// [`Automatable`] label.
#[derive(Debug, Clone, Default)]
pub struct TestAccounting {
    /// RGAA tests of this criterion a mechanism could decide.
    pub automatable: usize,
    /// RGAA tests this criterion has in total.
    pub total: usize,
    /// Test identities, as the RGAA reference numbers them within the criterion.
    pub test_keys: Vec<String>,
}

impl TestAccounting {
    /// The [`Automatable`] label these counts imply, independent of the label the data
    /// file carries. Where the two disagree, one of them is wrong.
    #[must_use]
    pub fn implied_classification(&self) -> Automatable {
        if self.total == 0 || self.automatable == 0 {
            Automatable::NotAutomatable
        } else if self.automatable == self.total {
            Automatable::FullyAutomatable
        } else {
            Automatable::PartiallyAutomatable
        }
    }
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
    #[serde(default)]
    pub axe_rules: Vec<String>,
    #[serde(default)]
    pub axe_coverage: AxeCoverage,
    /// Per-test accounting from `automatable_criteres.json`. See [`TestAccounting`].
    #[serde(skip)]
    pub test_accounting: TestAccounting,
    #[serde(default)]
    pub axe_provenance: Option<AxeProvenance>,
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
        let axe_entries: Vec<AxeMappingEntry> =
            serde_json::from_str(AXE_MAPPING_JSON).expect("axe_mapping.json must parse");

        let mut automatable_map: HashMap<String, Automatable> = HashMap::new();
        let mut accounting_map: HashMap<String, TestAccounting> = HashMap::new();
        for ac in automatable_root.criteria {
            automatable_map.insert(ac.criterion_id.clone(), ac.classification);
            accounting_map.insert(
                ac.criterion_id,
                TestAccounting {
                    automatable: ac.automatable_test_count,
                    total: ac.total_test_count,
                    test_keys: ac.test_keys,
                },
            );
        }

        let mut axe_rules_map: HashMap<String, Vec<String>> = HashMap::new();
        let mut axe_coverage_map: HashMap<String, AxeCoverage> = HashMap::new();
        let mut axe_provenance_map: HashMap<String, AxeProvenance> = HashMap::new();
        for entry in axe_entries {
            axe_rules_map.insert(entry.criterion_id.clone(), entry.axe_rules);
            axe_coverage_map.insert(entry.criterion_id.clone(), entry.coverage);
            axe_provenance_map.insert(entry.criterion_id, entry.provenance);
        }

        let mut themes = raw.topics;
        for theme in &mut themes {
            for cw in &mut theme.criteria {
                let criterion_id = cw.criterium.id_for_theme(theme.number);
                cw.criterium.automatable =
                    automatable_map.remove(&criterion_id).unwrap_or_default();
                cw.criterium.test_accounting =
                    accounting_map.remove(&criterion_id).unwrap_or_default();
                cw.criterium.axe_rules = axe_rules_map.remove(&criterion_id).unwrap_or_default();
                cw.criterium.axe_coverage =
                    axe_coverage_map.remove(&criterion_id).unwrap_or_default();
                cw.criterium.axe_provenance = axe_provenance_map.remove(&criterion_id);
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

    /// `"theme.criterion"` id → `(theme, criterion)`, built once per process from
    /// `Self::all()` so `by_id` is a single hash lookup instead of a linear scan of
    /// themes and criteria on every call.
    fn id_index() -> &'static HashMap<String, (u8, &'static CatalogCriterion)> {
        static INDEX: OnceLock<HashMap<String, (u8, &'static CatalogCriterion)>> = OnceLock::new();
        INDEX.get_or_init(|| {
            let mut index = HashMap::new();
            for theme in Self::all() {
                for cw in &theme.criteria {
                    let id = cw.criterium.id_for_theme(theme.number);
                    index.insert(id, (theme.number, &cw.criterium));
                }
            }
            index
        })
    }

    /// Looks up by theme/criterion *numbers*, not the literal string: `"1.01"` and
    /// `"01.1"` both resolve to the same criterion as `"1.1"`, matching the
    /// pre-index behavior (`u8::from_str` on each half discards leading zeros).
    pub fn by_id(criterion_id: &str) -> Option<(u8, &'static CatalogCriterion)> {
        let mut parts = criterion_id.splitn(2, '.');
        let theme: u8 = parts.next()?.parse().ok()?;
        let crit_num: u8 = parts.next()?.parse().ok()?;
        Self::id_index()
            .get(&format!("{theme}.{crit_num}"))
            .copied()
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
    fn by_id_normalizes_leading_zeroes() {
        let (_, canonical) = RgaaCatalog::by_id("1.1").expect("1.1 must exist");
        let (_, leading_zero) = RgaaCatalog::by_id("01.01").expect("01.01 must resolve to 1.1");
        assert_eq!(canonical.number, leading_zero.number);
        assert_eq!(canonical.title, leading_zero.title);
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
        // 45 / 45 / 16. Was 39 / 45 / 22 before #201 corrected 2.1 and 12.3, and before
        // #203 corrected the remaining four rows whose label contradicted its own counts
        // (4.9, 5.5, 10.2, 10.14 — all NotAutomatable with every test automatable). Those
        // counts decide a criterion's verdict once results are reduced per test, so they
        // had to agree with the label first.
        assert_eq!(fully, 45, "expected 45 FullyAutomatable criteria");
        assert_eq!(partially, 45, "expected 45 PartiallyAutomatable criteria");
        assert_eq!(not_automatable, 16, "expected 16 NotAutomatable criteria");
        assert_eq!(fully + partially + not_automatable, 106);
    }

    /// #201 AC4. Both rows were wrong in `automatable_criteres.json`: 2.1 claimed 0 of
    /// 1 automatable tests although `frame-title` decides its single test, and 12.3
    /// was labelled `NotAutomatable` while carrying 3 of 3 automatable tests — the
    /// label contradicting its own counts.
    #[test]
    fn corrected_automatability_labels_stay_corrected() {
        let (_, two_one) = RgaaCatalog::by_id("2.1").expect("2.1 is in the catalog");
        assert_eq!(two_one.automatable, Automatable::FullyAutomatable);
        assert!(
            two_one.axe_rules.iter().any(|r| r == "frame-title"),
            "2.1 is automatable because frame-title decides it: {:?}",
            two_one.axe_rules
        );

        let (_, twelve_three) = RgaaCatalog::by_id("12.3").expect("12.3 is in the catalog");
        assert_eq!(twelve_three.automatable, Automatable::FullyAutomatable);
    }

    /// A criterion carrying axe rules must declare whether those rules decide it
    /// completely; the serde default is `Partial`, so a forgotten declaration fails
    /// closed rather than claiming a conformance axe cannot establish (#201 AC3).
    #[test]
    fn complete_axe_coverage_is_declared_not_inferred() {
        let (_, three_two) = RgaaCatalog::by_id("3.2").expect("3.2 is in the catalog");
        assert_eq!(three_two.axe_coverage, AxeCoverage::Complete);

        // 3.3 and 5.6 were declared complete although color-contrast measures text only
        // and td-headers-attr does not decide 5.6's four tests: axe's silence passed
        // them with nothing measured (#256).
        for id in ["3.3", "5.6"] {
            let (_, criterion) = RgaaCatalog::by_id(id).expect("criterion is in the catalog");
            assert_eq!(
                criterion.axe_coverage,
                AxeCoverage::Partial,
                "{id} must not be Pass-able by axe silence"
            );
        }

        // 13.1 has fifteen tests and one rule, meta-refresh.
        let (_, thirteen_one) = RgaaCatalog::by_id("13.1").expect("13.1 is in the catalog");
        assert_eq!(thirteen_one.axe_coverage, AxeCoverage::Partial);
        assert!(!thirteen_one.axe_rules.is_empty());
    }

    #[test]
    fn test_criteria_with_axe_rules_have_valid_mapping() {
        let catalog = RgaaCatalog::all();
        for theme in catalog {
            for cw in &theme.criteria {
                if !cw.criterium.axe_rules.is_empty() {
                    assert!(
                        cw.criterium.axe_provenance.is_some(),
                        "criterion {} has axe_rules but no axe_provenance",
                        cw.criterium.id_for_theme(theme.number)
                    );
                }
            }
        }
    }

    /// Rows whose `classification` contradicts its own test counts, tolerated for now.
    ///
    /// **Empty, and it must stay that way.** Five existed: #201 fixed the one its AC4
    /// named (12.3) and pinned the other four; #203 fixed those (4.9, 5.5, 10.2, 10.14)
    /// because reducing a criterion verdict from its test outcomes makes the counts
    /// decide the answer, so a label that disagrees with them is a latent wrong verdict.
    ///
    /// The list survives as a tripwire: an entry added here is a row someone chose to
    /// leave wrong, and the assertion below refuses to let it be left silently.
    const KNOWN_LABEL_COUNT_DISAGREEMENTS: &[&str] = &[];

    #[test]
    fn automatability_labels_agree_with_their_own_test_counts() {
        let mut disagreements: Vec<String> = Vec::new();
        for theme in RgaaCatalog::all() {
            for cw in &theme.criteria {
                let criterion = &cw.criterium;
                let id = criterion.id_for_theme(theme.number);
                let implied = criterion.test_accounting.implied_classification();
                if implied != criterion.automatable {
                    disagreements.push(format!(
                        "{id}: labelled {:?} but {}/{} tests imply {implied:?}",
                        criterion.automatable,
                        criterion.test_accounting.automatable,
                        criterion.test_accounting.total
                    ));
                }
            }
        }

        let unexpected: Vec<&String> = disagreements
            .iter()
            .filter(|d| {
                let id = d.split(':').next().unwrap_or_default();
                !KNOWN_LABEL_COUNT_DISAGREEMENTS.contains(&id)
            })
            .collect();
        assert!(
            unexpected.is_empty(),
            "new label/count disagreements in automatable_criteres.json: {unexpected:#?}"
        );
        assert_eq!(
            disagreements.len(),
            KNOWN_LABEL_COUNT_DISAGREEMENTS.len(),
            "the known-disagreement list is stale — it must shrink as rows are fixed, \
             never be left claiming rows that are now correct. Found: {disagreements:#?}"
        );
    }

    /// The counts #203's reduction would consume must actually be loaded — they were
    /// shipped in the data file and read by nothing.
    #[test]
    fn per_test_accounting_is_loaded_from_the_catalog() {
        let (_, one_one) = RgaaCatalog::by_id("1.1").expect("1.1 is in the catalog");
        assert_eq!(one_one.test_accounting.automatable, 12);
        assert_eq!(one_one.test_accounting.total, 20);
        assert!(!one_one.test_accounting.test_keys.is_empty());
        assert_eq!(
            one_one.test_accounting.implied_classification(),
            Automatable::PartiallyAutomatable
        );

        let total_tests: usize = RgaaCatalog::all()
            .iter()
            .flat_map(|t| t.criteria.iter())
            .map(|cw| cw.criterium.test_accounting.total)
            .sum();
        let automatable_tests: usize = RgaaCatalog::all()
            .iter()
            .flat_map(|t| t.criteria.iter())
            .map(|cw| cw.criterium.test_accounting.automatable)
            .sum();
        assert_eq!(total_tests, 693, "the catalog carries 693 RGAA tests");
        // 371, not the 370 #203 measured: #201 corrected 2.1 from 0 of 1 automatable
        // tests to 1 of 1, because frame-title decides its single test.
        assert_eq!(
            automatable_tests, 371,
            "371 of them are marked automatable (54%)"
        );
    }
}
