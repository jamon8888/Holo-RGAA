use indexmap::IndexMap;
use rgaa_core::catalog::AxeCoverage;
use rgaa_core::{
    Classification, CriterionResult, CriterionStatus, RgaaCatalog, RgaaError, Violation,
};
use std::sync::OnceLock;

pub struct AxeMapper;

impl AxeMapper {
    /// Map axe-core violations JSON to RGAA criterion results.
    /// Input: JSON array of axe violations from axe.run()
    /// Output: HashMap of criterion_id → CriterionResult
    #[must_use = "le résultat du mapping doit être utilisé"]
    pub fn map(violations_json: &str) -> Result<IndexMap<String, CriterionResult>, RgaaError> {
        let mapping = Self::rgaa_to_axe_map();
        let violations: Vec<AxeViolation> = serde_json::from_str(violations_json)
            .map_err(|e| RgaaError::AxeCore(format!("Failed to parse axe violations JSON: {e}")))?;

        let mut results: IndexMap<String, CriterionResult> = IndexMap::new();

        // Only a criterion whose mapped rules decide every one of its tests may be
        // asserted Pass: for those, axe finding nothing really is evidence. A criterion
        // with AxeCoverage::Partial gets no entry here — its rules can raise a Fail
        // below, but their silence proves nothing, so it is left to the pipeline's
        // fallback instead of being initialised to a Pass no rule could contradict
        // (#199, #201).
        for rgaa_id in mapping
            .iter()
            .filter(|(_, m)| m.coverage == AxeCoverage::Complete)
            .map(|(id, _)| id)
        {
            results.insert(
                rgaa_id.clone(),
                CriterionResult {
                    criterion_id: rgaa_id.clone(),
                    title: String::new(),
                    classification: Classification::Deterministe,
                    status: CriterionStatus::Pass,
                    violations: vec![],
                    confidence: None,
                    justification: None,
                    source: "axe-core".to_string(),
                    citations: vec![],
                    considered_sources: vec![],
                    tests: vec![],
                },
            );
        }

        // Map violations to criteria. A partial-coverage criterion enters the results
        // here and only here, which is why a violation may have to create its entry.
        for violation in &violations {
            for (rgaa_id, mechanism) in mapping {
                if !mechanism.rules.iter().any(|rule| rule == &violation.id) {
                    continue;
                }
                let result = results
                    .entry(rgaa_id.clone())
                    .or_insert_with(|| CriterionResult {
                        criterion_id: rgaa_id.clone(),
                        title: String::new(),
                        classification: Classification::Deterministe,
                        status: CriterionStatus::Fail,
                        violations: vec![],
                        confidence: None,
                        justification: Some(
                            "Partial axe coverage: a violation was found, but axe does not \
                             decide every test of this criterion"
                                .to_string(),
                        ),
                        source: "axe-core".to_string(),
                        citations: vec![],
                        considered_sources: vec![],
                        tests: vec![],
                    });
                result.status = CriterionStatus::Fail;
                result.violations.push(Violation {
                    rule_id: violation.id.clone(),
                    impact: violation.impact.clone(),
                    description: violation.description.clone(),
                    nodes_affected: violation.nodes.len(),
                });
            }
        }

        Ok(results)
    }

    /// Built once per process and shared read-only — this table (with its several
    /// hundred string allocations) no longer gets rebuilt on every `map()` call.
    fn rgaa_to_axe_map() -> &'static IndexMap<String, CriterionMechanism> {
        static MAPPING: OnceLock<IndexMap<String, CriterionMechanism>> = OnceLock::new();
        MAPPING.get_or_init(Self::build_rgaa_to_axe_map)
    }

    /// Derived from the catalog's `axe_mapping.json`, which is the single source of
    /// truth for RGAA → axe-core rule assignments.
    ///
    /// Criteria whose mapping carries no axe rule are **omitted**: initializing them
    /// to `Pass` would assert conformance no axe rule could ever contradict. They are
    /// left to the pipeline's declared fallback instead (#199).
    fn build_rgaa_to_axe_map() -> IndexMap<String, CriterionMechanism> {
        let mut m: IndexMap<String, CriterionMechanism> = IndexMap::new();
        for theme in RgaaCatalog::all() {
            for wrapper in &theme.criteria {
                let criterion = &wrapper.criterium;
                if criterion.axe_rules.is_empty() {
                    continue;
                }
                m.insert(
                    criterion.id_for_theme(theme.number),
                    CriterionMechanism {
                        rules: criterion.axe_rules.clone(),
                        coverage: criterion.axe_coverage,
                    },
                );
            }
        }
        m
    }
}

/// What axe-core can say about one criterion: the rules mapped to it, and whether
/// they decide it completely enough for silence to count as a `Pass`.
struct CriterionMechanism {
    rules: Vec<String>,
    coverage: AxeCoverage,
}

#[derive(serde::Deserialize)]
struct AxeViolation {
    id: String,
    impact: String,
    description: String,
    nodes: Vec<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The authoritative list of axe-core rule ids, as shipped with the catalog.
    const AXE_RULES_JSON: &str = include_str!("../../rgaa-core/data/rgaa-4.1.2/axe_rules.json");

    fn real_axe_rule_ids() -> std::collections::HashSet<String> {
        #[derive(serde::Deserialize)]
        struct Rule {
            id: String,
        }
        let rules: Vec<Rule> =
            serde_json::from_str(AXE_RULES_JSON).expect("axe_rules.json must parse");
        rules.into_iter().map(|r| r.id).collect()
    }

    /// A rule name axe-core never emits can never turn a criterion into a Fail, so a
    /// criterion mapped only to such names is initialized Pass and stays Pass whatever
    /// the page contains. See #199.
    #[test]
    fn every_mapped_rule_name_is_a_real_axe_rule_id() {
        let real = real_axe_rule_ids();
        let mut fictional: Vec<(&str, &str)> = Vec::new();
        for (criterion_id, mechanism) in AxeMapper::rgaa_to_axe_map() {
            for rule in &mechanism.rules {
                if !real.contains(rule) {
                    fictional.push((criterion_id.as_str(), rule.as_str()));
                }
            }
        }
        assert!(
            fictional.is_empty(),
            "{} mapped rule names are not axe-core rule ids: {:?}",
            fictional.len(),
            fictional
        );
    }

    /// 12.9 (keyboard trap) has no axe-core rule at all. 10.4, 13.8, 13.1, 7.1 and
    /// 11.13 gained real rules in #201 but only partial coverage — `meta-refresh`
    /// finding nothing says nothing about the other fourteen tests of 13.1. Neither
    /// kind may be reported as a deterministic Pass on axe's silence.
    #[test]
    fn criterion_without_a_real_axe_rule_is_absent_from_the_results() {
        let results = AxeMapper::map("[]").unwrap();
        for criterion_id in ["12.9", "10.4", "13.8", "13.1", "7.1", "11.13"] {
            assert!(
                !results.contains_key(criterion_id),
                "criterion {criterion_id} cannot be passed by axe, yet the mapper emitted {:?}",
                results.get(criterion_id).map(|r| &r.status)
            );
        }
    }

    #[test]
    fn empty_json_initializes_all_mapped_criteria_as_pass() {
        let result = AxeMapper::map("[]").unwrap();
        // 30 of the 43 criteria carrying axe rules: only those whose rules decide every
        // test of the criterion may be asserted Pass by axe's silence (#199, #201).
        assert_eq!(result.len(), COMPLETE_COVERAGE_CRITERIA.len());
        for (id, r) in &result {
            assert_eq!(
                r.status,
                CriterionStatus::Pass,
                "criterion {id} should be Pass"
            );
        }
    }

    #[test]
    fn invalid_json_returns_error() {
        let err = AxeMapper::map("not json").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Failed to parse axe violations JSON"));
    }

    #[test]
    fn mapped_violation_sets_status_to_fail() {
        let axe_json = r#"[
            {
                "id": "color-contrast",
                "impact": "serious",
                "description": "Elements must have sufficient color contrast",
                "nodes": [{"html": "<p>Low contrast</p>"}]
            }
        ]"#;
        let results = AxeMapper::map(axe_json).unwrap();
        // color-contrast maps to 3.2 (text contrast, RGAA-3.2.1)
        let r = results.get("3.2").expect("3.2 should be present");
        assert_eq!(r.status, CriterionStatus::Fail);
        assert!(!r.violations.is_empty());
    }

    #[test]
    fn unmapped_axe_rule_does_not_corrupt_results() {
        let axe_json = r#"[
            {
                "id": "unknown-rule-xyz",
                "impact": "minor",
                "description": "Unknown rule",
                "nodes": []
            }
        ]"#;
        let results = AxeMapper::map(axe_json).unwrap();
        // Should not create any entries for unmapped rules
        assert!(results.is_empty() || results.values().all(|r| r.status == CriterionStatus::Pass));
    }

    #[test]
    fn multiple_violations_same_criterion_merge() {
        let axe_json = r#"[
            {
                "id": "color-contrast",
                "impact": "serious",
                "description": "First",
                "nodes": [{"html": "<p>A</p>"}]
            },
            {
                "id": "color-contrast",
                "impact": "critical",
                "description": "Second",
                "nodes": [{"html": "<p>B</p>"}]
            }
        ]"#;
        let results = AxeMapper::map(axe_json).unwrap();
        let r = results.get("3.2").expect("3.2 should be present");
        assert_eq!(r.status, CriterionStatus::Fail);
        // Both violations should be present
        assert_eq!(r.violations.len(), 2);
    }

    #[test]
    fn all_mapped_criteria_initialized_as_pass() {
        let axe_json = "[]";
        let results = AxeMapper::map(axe_json).unwrap();
        // Only complete-coverage criteria (#199, #201).
        assert_eq!(results.len(), COMPLETE_COVERAGE_CRITERIA.len());
        for (id, r) in &results {
            assert_eq!(
                r.status,
                CriterionStatus::Pass,
                "criterion {id} should be Pass"
            );
        }
    }

    #[test]
    fn output_is_indexmap_insertion_ordered() {
        let axe_json = r#"[
            {
                "id": "color-contrast",
                "impact": "serious",
                "description": "test",
                "nodes": []
            }
        ]"#;
        let results = AxeMapper::map(axe_json).unwrap();
        let keys: Vec<_> = results.keys().cloned().collect();
        // First key should be "1.1" (first in rgaa_to_axe_map), not random
        assert_eq!(keys[0], "1.1");
    }

    // ---------------------------------------------------------------- #201

    /// Criteria whose mapped axe rules decide every one of their tests, so axe's
    /// silence is evidence and a criterion-level `Pass` is justified. Listed rather
    /// than derived so that moving a criterion between the two regimes is a visible,
    /// reviewed change and not a side effect of editing `axe_mapping.json`.
    const COMPLETE_COVERAGE_CRITERIA: &[&str] = &[
        "1.1", "2.1", "3.2", "4.3", "5.6", "5.7", "6.2", "8.3", "8.5", "9.3", "10.6", "10.8",
        "11.1", "12.7",
    ];

    /// Criteria whose mapped rules catch real violations but cannot establish
    /// conformance: axe may fail them, never pass them.
    const PARTIAL_COVERAGE_CRITERIA: &[&str] = &[
        "1.2", "4.10", "5.4", "6.1", "7.1", "7.3", "8.2", "8.4", "8.8", "9.1", "10.4", "10.11",
        "11.2", "11.9", "11.13", "12.6", "12.8", "12.10", "13.1", "13.8", "13.9",
    ];

    /// RGAAv4-tagged axe rules deliberately left unmapped, each with the reason.
    /// Empty today: #201 assigned all 67. The list exists so that an axe-core upgrade
    /// introducing a new RGAAv4 rule forces an explicit decision instead of silently
    /// discarding it.
    const DELIBERATELY_UNMAPPED: &[(&str, &str)] = &[];

    /// `axe.run()` is called with no `runOnly` filter, so every RGAAv4-tagged rule is
    /// already computed on every page. A rule that is neither mapped nor explicitly
    /// excluded is coverage being computed and thrown away (#201 AC1).
    #[test]
    fn every_rgaav4_tagged_axe_rule_is_mapped_or_explicitly_excluded() {
        #[derive(serde::Deserialize)]
        struct Rule {
            id: String,
            #[serde(default)]
            tags: Vec<String>,
        }
        let rules: Vec<Rule> =
            serde_json::from_str(AXE_RULES_JSON).expect("axe_rules.json must parse");

        let mapped: std::collections::HashSet<&str> = AxeMapper::rgaa_to_axe_map()
            .values()
            .flat_map(|m| m.rules.iter().map(String::as_str))
            .collect();
        let excluded: std::collections::HashSet<&str> =
            DELIBERATELY_UNMAPPED.iter().map(|(id, _)| *id).collect();

        let discarded: Vec<&str> = rules
            .iter()
            .filter(|r| r.tags.iter().any(|t| t == "RGAAv4"))
            .map(|r| r.id.as_str())
            .filter(|id| !mapped.contains(id) && !excluded.contains(id))
            .collect();

        assert!(
            discarded.is_empty(),
            "{} RGAAv4-tagged axe rules are computed on every page and discarded: {:?}. \
             Map them to a criterion, or add them to DELIBERATELY_UNMAPPED with a reason.",
            discarded.len(),
            discarded
        );
    }

    /// Every rule #201 mapped, against the criterion it was mapped to. A synthetic
    /// violation carrying the rule id must fail that criterion (#201 AC2) — this is
    /// what proves the rule is wired, not merely written down in the data file.
    const NEWLY_MAPPED: &[(&str, &str)] = &[
        ("aria-roles", "7.1"),
        ("aria-valid-attr", "7.1"),
        ("aria-valid-attr-value", "7.1"),
        ("aria-allowed-attr", "7.1"),
        ("aria-required-attr", "7.1"),
        ("aria-required-children", "7.1"),
        ("aria-required-parent", "7.1"),
        ("aria-prohibited-attr", "7.1"),
        ("aria-conditional-attr", "7.1"),
        ("aria-deprecated-role", "7.1"),
        ("aria-hidden-body", "7.1"),
        ("nested-interactive", "7.1"),
        ("aria-command-name", "7.1"),
        ("aria-meter-name", "7.1"),
        ("aria-progressbar-name", "7.1"),
        ("aria-tab-name", "7.1"),
        ("frame-title", "2.1"),
        ("frame-title-unique", "2.1"),
        ("frame-focusable-content", "2.1"),
        ("autocomplete-valid", "11.13"),
        ("button-name", "11.1"),
        ("input-button-name", "11.1"),
        ("select-name", "11.1"),
        ("aria-input-field-name", "11.1"),
        ("aria-toggle-field-name", "11.1"),
        ("form-field-multiple-labels", "11.1"),
        ("label-content-name-mismatch", "11.2"),
        ("scrollable-region-focusable", "7.3"),
        ("server-side-image-map", "7.3"),
        ("focus-order-semantics", "7.3"),
        ("blink", "13.8"),
        ("marquee", "13.8"),
        ("meta-refresh", "13.1"),
        ("css-orientation-lock", "13.9"),
        ("meta-viewport", "10.4"),
        ("meta-viewport", "10.11"),
        ("no-autoplay-audio", "4.10"),
        ("valid-lang", "8.8"),
        ("html-xml-lang-mismatch", "8.4"),
        ("duplicate-id-aria", "8.2"),
        ("p-as-heading", "9.1"),
        ("definition-list", "9.3"),
        ("dlitem", "9.3"),
        ("table-fake-caption", "5.4"),
        ("table-duplicate-name", "5.4"),
        ("td-has-header", "5.7"),
        ("area-alt", "1.1"),
        ("object-alt", "1.1"),
        ("role-img-alt", "1.1"),
        ("svg-img-alt", "1.1"),
    ];

    #[test]
    fn each_newly_mapped_rule_fails_its_intended_criterion() {
        for (rule_id, criterion_id) in NEWLY_MAPPED {
            let axe_json = format!(
                r#"[{{"id":"{rule_id}","impact":"serious","description":"synthetic {rule_id}","nodes":[{{"html":"<x/>"}}]}}]"#
            );
            let results = AxeMapper::map(&axe_json)
                .unwrap_or_else(|e| panic!("mapping {rule_id} must not error: {e}"));
            let result = results.get(*criterion_id).unwrap_or_else(|| {
                panic!("{rule_id} must produce a result for criterion {criterion_id}")
            });
            assert_eq!(
                result.status,
                CriterionStatus::Fail,
                "{rule_id} must fail criterion {criterion_id}"
            );
            assert!(
                result.violations.iter().any(|v| v.rule_id == *rule_id),
                "{criterion_id} must carry {rule_id} as evidence, got {:?}",
                result.violations
            );
        }
    }

    /// The #201 guard: adding rules to a criterion must not create a criterion-level
    /// `Pass` the rules cannot justify. A partial-coverage criterion appears in the
    /// results only when one of its rules actually fired (#201 AC3).
    #[test]
    fn partial_coverage_criterion_is_never_passed_by_axe_silence() {
        let results = AxeMapper::map("[]").unwrap();
        for criterion_id in PARTIAL_COVERAGE_CRITERIA {
            assert!(
                !results.contains_key(*criterion_id),
                "{criterion_id} has partial axe coverage, yet axe's silence produced {:?}",
                results.get(*criterion_id).map(|r| &r.status)
            );
        }
        for criterion_id in COMPLETE_COVERAGE_CRITERIA {
            let result = results.get(*criterion_id).unwrap_or_else(|| {
                panic!("{criterion_id} has complete axe coverage and must be present")
            });
            assert_eq!(
                result.status,
                CriterionStatus::Pass,
                "{criterion_id} has complete coverage and no violation, so it must Pass"
            );
        }
    }

    /// The two regimes must partition the criteria that carry rules — a criterion in
    /// neither list would mean `axe_mapping.json` gained an entry without anyone
    /// deciding whether axe may pass it.
    #[test]
    fn coverage_lists_account_for_every_criterion_carrying_axe_rules() {
        let mapped: Vec<&str> = AxeMapper::rgaa_to_axe_map()
            .keys()
            .map(String::as_str)
            .collect();
        let declared: std::collections::HashSet<&str> = COMPLETE_COVERAGE_CRITERIA
            .iter()
            .chain(PARTIAL_COVERAGE_CRITERIA.iter())
            .copied()
            .collect();

        let undeclared: Vec<&&str> = mapped
            .iter()
            .filter(|id| !declared.contains(**id))
            .collect();
        assert!(
            undeclared.is_empty(),
            "criteria carry axe rules but declare no coverage regime: {undeclared:?}"
        );
        assert_eq!(
            mapped.len(),
            COMPLETE_COVERAGE_CRITERIA.len() + PARTIAL_COVERAGE_CRITERIA.len(),
            "coverage lists and the mapping disagree on how many criteria carry rules"
        );
    }

    /// 21 criteria gain deterministic evidence in #201, 14 of them from zero.
    #[test]
    fn the_harvest_widened_deterministic_coverage() {
        let mapping = AxeMapper::rgaa_to_axe_map();
        assert_eq!(
            mapping.len(),
            35,
            "43 criteria carried axe rules after #201; the 2026-10-05 engine plan removed 12 mappings that could not decide their criterion (1.5, 1.6, 3.3, 10.2, 10.5, 10.9, 11.4, 12.1, 12.4, 13.3-13.5) and moved document-title to 8.5; it then added partial rules for 11.9, 12.8 and 12.10"
        );
        let rule_refs: usize = mapping.values().map(|m| m.rules.len()).sum();
        assert_eq!(rule_refs, 84, "expected 84 criterion/rule pairs");
    }
}
