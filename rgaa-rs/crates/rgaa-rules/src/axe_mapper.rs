use indexmap::IndexMap;
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

        // Initialize all axe-mapped criteria as PASS
        for rgaa_id in mapping.keys() {
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
                },
            );
        }

        // Map violations to criteria
        for violation in &violations {
            for (rgaa_id, axe_rules) in mapping {
                if axe_rules.iter().any(|rule| rule == &violation.id) {
                    if let Some(result) = results.get_mut(rgaa_id) {
                        result.status = CriterionStatus::Fail;
                        result.violations.push(Violation {
                            rule_id: violation.id.clone(),
                            impact: violation.impact.clone(),
                            description: violation.description.clone(),
                            nodes_affected: violation.nodes.len(),
                        });
                    }
                }
            }
        }

        Ok(results)
    }

    /// Built once per process and shared read-only — this ~77-entry table (with its
    /// ~150 string allocations) no longer gets rebuilt on every `map()` call.
    fn rgaa_to_axe_map() -> &'static IndexMap<String, Vec<String>> {
        static MAPPING: OnceLock<IndexMap<String, Vec<String>>> = OnceLock::new();
        MAPPING.get_or_init(Self::build_rgaa_to_axe_map)
    }

    /// Derived from the catalog's `axe_mapping.json`, which is the single source of
    /// truth for RGAA → axe-core rule assignments.
    ///
    /// Criteria whose mapping carries no axe rule are **omitted**: initializing them
    /// to `Pass` would assert conformance no axe rule could ever contradict. They are
    /// left to the pipeline's declared fallback instead (#199).
    fn build_rgaa_to_axe_map() -> IndexMap<String, Vec<String>> {
        let mut m: IndexMap<String, Vec<String>> = IndexMap::new();
        for theme in RgaaCatalog::all() {
            for wrapper in &theme.criteria {
                let criterion = &wrapper.criterium;
                if criterion.axe_rules.is_empty() {
                    continue;
                }
                m.insert(
                    criterion.id_for_theme(theme.number),
                    criterion.axe_rules.clone(),
                );
            }
        }
        m
    }
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
        for (criterion_id, rules) in AxeMapper::rgaa_to_axe_map() {
            for rule in rules {
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

    /// 12.9 (keyboard trap) has no axe-core rule. It must not be reported as a
    /// deterministic Pass just because axe found nothing: axe was never asked.
    #[test]
    fn criterion_without_a_real_axe_rule_is_absent_from_the_results() {
        let results = AxeMapper::map("[]").unwrap();
        for criterion_id in ["12.9", "10.4", "13.8", "13.1", "7.1", "11.13"] {
            assert!(
                !results.contains_key(criterion_id),
                "criterion {criterion_id} has no real axe rule, yet the mapper emitted {:?}",
                results.get(criterion_id).map(|r| &r.status)
            );
        }
    }

    #[test]
    fn empty_json_initializes_all_mapped_criteria_as_pass() {
        let result = AxeMapper::map("[]").unwrap();
        // 29, not the mapping file's 77 entries: only criteria carrying at least one
        // real axe rule can be asserted Pass by axe (#199).
        assert_eq!(result.len(), 29);
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
        // color-contrast maps to 3.3
        let r = results.get("3.3").expect("3.3 should be present");
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
        let r = results.get("3.3").expect("3.3 should be present");
        assert_eq!(r.status, CriterionStatus::Fail);
        // Both violations should be present
        assert_eq!(r.violations.len(), 2);
    }

    #[test]
    fn all_mapped_criteria_initialized_as_pass() {
        let axe_json = "[]";
        let results = AxeMapper::map(axe_json).unwrap();
        // Only the 29 criteria with at least one real axe rule (#199).
        assert_eq!(results.len(), 29);
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
}
