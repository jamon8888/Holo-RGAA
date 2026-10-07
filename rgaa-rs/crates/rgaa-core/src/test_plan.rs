//! Per-test routes for the 258 RGAA 4.1.2 catalog tests.
//!
//! A complete mechanism declares coverage of every test of its criterion unless
//! its explicit test list restricts that declaration. Partial mechanisms need
//! explicit test keys: their silence never establishes conformance. The model
//! fallback is always an estimate, including for routes with no mechanism.

use crate::catalog::AxeCoverage;
use crate::registry::Outcome;
use crate::{MechanismRegistry, RgaaCatalog, RgaaError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::sync::OnceLock;

const TEST_ROUTES_JSON: &str = include_str!("../data/rgaa-4.1.2/test_routes.json");

/// Whether non-model evidence can fully decide a catalog test.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CoverageLevel {
    /// A named complete mechanism can decide the test and may establish `Pass`.
    Complete,
    /// Observed failures are usable, but silence cannot establish `Pass`.
    Partial,
}

/// The evaluator contract for one canonical criterion and local catalog test key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRoute {
    /// Canonical criterion ID, such as `"1.1"`.
    pub criterion_id: String,
    /// Local test key, such as `"1"`.
    pub test_key: String,
    /// Registered non-model mechanisms that cover this test, in execution order.
    pub mechanisms: Vec<String>,
    /// Required declaration; omitted coverage cannot silently become complete.
    pub coverage: CoverageLevel,
    /// Estimate fallback; the only admitted value is `"holo_estimate"`.
    pub fallback: String,
}

/// Parsed routes with a borrowed-key lookup; raw rows are kept to detect duplicates.
#[derive(Debug, Clone)]
pub struct TestRoutePlan {
    routes: Vec<TestRoute>,
    index: HashMap<String, HashMap<String, usize>>,
}

impl TestRoutePlan {
    /// Parse the route contract without hiding duplicate rows.
    ///
    /// Use [`Self::check`] to validate catalog identities and mechanism references.
    ///
    /// # Errors
    /// Returns an error for malformed JSON or missing/invalid required declarations.
    pub fn from_json_str(text: &str) -> Result<Self, RgaaError> {
        let routes: Vec<TestRoute> = serde_json::from_str(text)
            .map_err(|error| RgaaError::MissingField(format!("test route plan: {error}")))?;
        let mut index: HashMap<String, HashMap<String, usize>> = HashMap::new();
        for (position, route) in routes.iter().enumerate() {
            index
                .entry(route.criterion_id.clone())
                .or_default()
                .entry(route.test_key.clone())
                .or_insert(position);
        }
        Ok(Self { routes, index })
    }

    /// The embedded plan, parsed once per process and validated by corpus invariants.
    ///
    /// # Panics
    /// Panics only if the embedded JSON is malformed, indicating a build defect.
    pub fn builtin() -> &'static Self {
        static PLAN: OnceLock<TestRoutePlan> = OnceLock::new();
        PLAN.get_or_init(|| {
            Self::from_json_str(TEST_ROUTES_JSON).expect("test_routes.json must parse")
        })
    }

    /// All declared rows in file order, including duplicates in an invalid plan.
    pub fn routes(&self) -> &[TestRoute] {
        &self.routes
    }

    /// Look up a canonical criterion and local test key without allocating.
    pub fn for_test(&self, criterion_id: &str, test_key: &str) -> Option<&TestRoute> {
        let position = self.index.get(criterion_id)?.get(test_key)?;
        self.routes.get(*position)
    }

    /// Every violated route invariant, naming the exact criterion and local key.
    ///
    /// A complete route requires at least one complete mechanism allowed to emit
    /// `Pass`. Partial mechanisms may only appear for explicitly declared tests
    /// and may never declare `Pass`, even within a complete route.
    pub fn check(&self, registry: &MechanismRegistry) -> Vec<String> {
        let catalog_keys = RgaaCatalog::all_test_keys();
        let catalog: BTreeSet<_> = catalog_keys
            .iter()
            .map(|(criterion, key)| (criterion.as_str(), key.as_str()))
            .collect();
        let mechanisms: HashMap<_, _> = registry
            .mechanisms()
            .iter()
            .map(|mechanism| (mechanism.id.as_str(), mechanism))
            .collect();
        let mut seen = BTreeSet::new();
        let mut problems = Vec::new();
        for route in &self.routes {
            let pair = (route.criterion_id.as_str(), route.test_key.as_str());
            let identity = format!("{}/{}", pair.0, pair.1);
            if !seen.insert(pair) {
                problems.push(format!("{identity}: duplicate test route"));
            }
            if !catalog.contains(&pair) {
                problems.push(format!("{identity}: unknown catalog test pair"));
            }
            if route.fallback != "holo_estimate" {
                problems.push(format!("{identity}: invalid fallback {:?}", route.fallback));
            }
            let mut deciding = false;
            let mut mechanism_ids = BTreeSet::new();
            for id in &route.mechanisms {
                if !mechanism_ids.insert(id.as_str()) {
                    problems.push(format!("{identity}: duplicate mechanism {id}"));
                }
                let Some(mechanism) = mechanisms.get(id.as_str()) else {
                    problems.push(format!("{identity}: unknown mechanism {id}"));
                    continue;
                };
                if mechanism.criterion != route.criterion_id {
                    problems.push(format!(
                        "{identity}: mechanism {id} belongs to criterion {}",
                        mechanism.criterion
                    ));
                    continue;
                }
                let explicit = mechanism.tests.iter().any(|test| {
                    test == &route.test_key
                        || test
                            .strip_prefix(&route.criterion_id)
                            .and_then(|suffix| suffix.strip_prefix('.'))
                            == Some(route.test_key.as_str())
                });
                let covered = explicit
                    || (mechanism.tests.is_empty() && mechanism.coverage == AxeCoverage::Complete);
                if !covered {
                    problems.push(format!(
                        "{identity}: mechanism {id} has no explicit coverage of this test"
                    ));
                }
                if mechanism.coverage == AxeCoverage::Partial
                    && mechanism.outcomes.contains(&Outcome::Pass)
                {
                    problems.push(format!(
                        "{identity}: partial mechanism {id} cannot emit pass"
                    ));
                }
                deciding |= covered
                    && mechanism.coverage == AxeCoverage::Complete
                    && mechanism.outcomes.contains(&Outcome::Pass);
            }
            if route.coverage == CoverageLevel::Complete && !deciding {
                problems.push(format!(
                    "{identity}: complete route has no complete deciding mechanism"
                ));
            }
        }
        for (criterion, key) in catalog_keys {
            if !seen.contains(&(criterion.as_str(), key.as_str())) {
                problems.push(format!("{criterion}/{key}: missing test route"));
            }
        }
        problems
    }
}
