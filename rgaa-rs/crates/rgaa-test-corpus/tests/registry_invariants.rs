//! CI invariants of the single mechanism registry (spec §4/§5, issue #261).
//!
//! A pure file/data check: no browser, runs in the ordinary test job. The
//! classification of the fixtures under Obscura lives in the E2E job
//! (`rgaa-orchestrator/tests/fixture_classification.rs`).

use rgaa_core::catalog::AxeCoverage;
use rgaa_core::MechanismRegistry;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Authoritative axe-core rule ids shipped with the catalog.
const AXE_RULES_JSON: &str = include_str!("../../rgaa-core/data/rgaa-4.1.2/axe_rules.json");

fn known_axe_rules() -> HashSet<String> {
    #[derive(serde::Deserialize)]
    struct Rule {
        id: String,
    }
    let rules: Vec<Rule> = serde_json::from_str(AXE_RULES_JSON).expect("axe_rules.json must parse");
    assert!(!rules.is_empty(), "axe_rules.json must be non-empty");
    rules.into_iter().map(|r| r.id).collect()
}

fn criteria_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("criteria")
}

/// Covers: `pass => complete`, fixtures present for `legacy = false`, every cited axe
/// rule exists, unique ids.
#[test]
fn registry_invariants_hold() {
    let problems = MechanismRegistry::builtin().check(&criteria_dir(), &known_axe_rules());
    assert!(
        problems.is_empty(),
        "mechanism registry violates {} invariant(s):\n  {}",
        problems.len(),
        problems.join("\n  ")
    );
}

/// Every fixture a registry entry names is a real corpus file, legacy or not.
#[test]
fn every_declared_fixture_exists() {
    let dir = criteria_dir();
    let missing: Vec<String> = MechanismRegistry::builtin()
        .mechanisms()
        .iter()
        .flat_map(|m| m.fixtures.iter().map(move |f| (m.id.as_str(), f)))
        .filter(|(_, f)| !dir.join(format!("{f}.html")).is_file())
        .map(|(id, f)| format!("{id}: {f}.html"))
        .collect();
    assert!(missing.is_empty(), "missing fixture files: {missing:?}");
}

/// The 23 formerly untested routes all have an active partial mechanism. This
/// invariant prevents the report/diagram from drifting back to an uncovered list
/// and makes sure none of these heuristic probes can claim a criterion-level Pass.
#[test]
fn all_formerly_untested_criteria_have_partial_controls() {
    use rgaa_core::registry::Outcome;

    let expected = [
        "1.6", "3.3", "4.12", "4.13", "8.7", "10.9", "10.12", "10.13", "11.3", "11.8", "11.11",
        "12.1", "12.2", "12.4", "12.5", "12.8", "12.9", "12.10", "12.11", "13.3", "13.10", "13.11",
        "13.12",
    ];
    let registry = MechanismRegistry::builtin();

    for criterion in expected {
        let mechanism = registry
            .probe_for(criterion)
            .unwrap_or_else(|| panic!("criterion {criterion} has no registered probe"));
        assert_eq!(
            mechanism.coverage,
            AxeCoverage::Partial,
            "criterion {criterion} must remain explicitly partial"
        );
        assert!(
            !mechanism.outcomes.contains(&Outcome::Pass),
            "criterion {criterion} cannot pass from its partial probe"
        );
        assert!(
            !mechanism.legacy,
            "criterion {criterion} probe must be active"
        );
    }
}
