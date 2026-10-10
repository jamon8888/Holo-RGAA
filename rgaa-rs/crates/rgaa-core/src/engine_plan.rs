//! Which engine owns the verdict for each of the 106 RGAA 4.1.2 criteria.
//!
//! Source: `data/rgaa-4.1.2/engine_plan.json`, derived from
//! `docs/research/couverture-repartition-106.md`. Every criterion has exactly one
//! primary engine; the other layers (axe rules, deterministic checks, Holo role,
//! human residual) are the supporting detail.

use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

const ENGINE_PLAN_JSON: &str = include_str!("../data/rgaa-4.1.2/engine_plan.json");

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Hash)]
pub enum PlanEngine {
    /// axe-core rules decide.
    AxeCore,
    /// In-house DOM/CSS/HTTP checks (Playwright, crawler, linter) decide.
    Deterministic,
    /// Holo vision/semantic judgement decides.
    Holo,
    /// A human must conclude; automation only pre-sorts.
    Human,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EnginePlanEntry {
    pub criterion_id: String,
    pub primary: PlanEngine,
    pub axe_rules: Vec<String>,
    pub deterministic: String,
    pub holo: String,
    pub human_residual: String,
}

pub struct EnginePlan;

static PLAN: OnceLock<HashMap<String, EnginePlanEntry>> = OnceLock::new();

impl EnginePlan {
    /// The executable route for a canonical catalog test, including its estimate fallback.
    pub fn route_test(
        criterion_id: &str,
        test_key: &str,
    ) -> Option<&'static crate::test_plan::TestRoute> {
        crate::test_plan::TestRoutePlan::builtin().for_test(criterion_id, test_key)
    }

    fn map() -> &'static HashMap<String, EnginePlanEntry> {
        PLAN.get_or_init(|| {
            let entries: Vec<EnginePlanEntry> = serde_json::from_str(ENGINE_PLAN_JSON)
                .expect("engine_plan.json is embedded and validated by tests");
            entries
                .into_iter()
                .map(|e| (e.criterion_id.clone(), e))
                .collect()
        })
    }

    pub fn get(id: &str) -> Option<&'static EnginePlanEntry> {
        Self::map().get(id)
    }

    pub fn primary(id: &str) -> Option<PlanEngine> {
        Self::get(id).map(|e| e.primary)
    }

    /// Criterion ids owned by `engine`, in catalog order.
    pub fn owned_by(engine: PlanEngine) -> Vec<&'static str> {
        crate::RgaaCriteria::all()
            .iter()
            .filter(|c| Self::primary(c.id) == Some(engine))
            .map(|c| c.id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RgaaCriteria;

    #[test]
    fn plan_covers_all_106_criteria() {
        assert_eq!(EnginePlan::map().len(), 106);
        for c in RgaaCriteria::all() {
            assert!(EnginePlan::get(c.id).is_some(), "{} has no engine", c.id);
        }
    }

    #[test]
    fn engines_partition_the_catalog() {
        let total: usize = [
            PlanEngine::AxeCore,
            PlanEngine::Deterministic,
            PlanEngine::Holo,
            PlanEngine::Human,
        ]
        .into_iter()
        .map(|e| EnginePlan::owned_by(e).len())
        .sum();
        assert_eq!(total, 106);
        assert_eq!(EnginePlan::owned_by(PlanEngine::Human).len(), 8);
    }

    #[test]
    fn engine_assignments_match_the_audited_criterion_map() {
        assert_eq!(EnginePlan::owned_by(PlanEngine::AxeCore).len(), 13);
        assert_eq!(EnginePlan::owned_by(PlanEngine::Deterministic).len(), 53);
        assert_eq!(EnginePlan::owned_by(PlanEngine::Holo).len(), 32);

        let human: std::collections::HashSet<&str> = EnginePlan::owned_by(PlanEngine::Human)
            .into_iter()
            .collect();
        let expected: std::collections::HashSet<&str> =
            ["4.2", "4.4", "4.6", "7.5", "11.12", "13.1", "13.4", "13.7"]
                .into_iter()
                .collect();
        assert_eq!(human, expected);
    }

    #[test]
    fn raw_plan_has_exactly_one_route_for_each_catalog_criterion() {
        let entries: Vec<EnginePlanEntry> =
            serde_json::from_str(ENGINE_PLAN_JSON).expect("embedded engine plan must parse");
        let route_ids: std::collections::HashSet<&str> = entries
            .iter()
            .map(|entry| entry.criterion_id.as_str())
            .collect();
        let catalog_ids: std::collections::HashSet<&str> = RgaaCriteria::all()
            .iter()
            .map(|criterion| criterion.id)
            .collect();

        assert_eq!(entries.len(), 106, "the raw plan must contain 106 rows");
        assert_eq!(route_ids.len(), entries.len(), "route IDs must be unique");
        assert_eq!(route_ids, catalog_ids, "routes must match the catalog IDs");
    }

    /// axe-core may only own a criterion it decides completely: with partial coverage its
    /// silence proves nothing, so the verdict must belong to a deterministic probe or Holo.
    #[test]
    fn axe_owns_only_criteria_it_covers_completely() {
        let registry = crate::MechanismRegistry::builtin();
        for id in EnginePlan::owned_by(PlanEngine::AxeCore) {
            let covered = registry
                .axe_for(id)
                .is_some_and(|m| m.coverage == crate::catalog::AxeCoverage::Complete);
            assert!(
                covered,
                "{id} is owned by axe-core without complete axe coverage"
            );
        }
    }

    #[test]
    fn axe_owned_criteria_name_axe_rules() {
        for id in EnginePlan::owned_by(PlanEngine::AxeCore) {
            assert!(!EnginePlan::get(id).unwrap().axe_rules.is_empty(), "{id}");
        }
    }

    #[test]
    fn planned_axe_rules_exist_in_axe_core() {
        let known: Vec<serde_json::Value> =
            serde_json::from_str(include_str!("../data/rgaa-4.1.2/axe_rules.json")).unwrap();
        let ids: std::collections::HashSet<&str> =
            known.iter().filter_map(|r| r["id"].as_str()).collect();
        let unknown: Vec<_> = EnginePlan::map()
            .values()
            .flat_map(|e| e.axe_rules.iter())
            .filter(|r| !ids.contains(r.as_str()))
            .cloned()
            .collect();
        assert!(
            unknown.is_empty(),
            "rules not in axe_rules.json: {unknown:?}"
        );
    }
}
