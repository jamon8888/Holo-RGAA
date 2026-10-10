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

/// Removing, duplicating or misnaming a route must not silently drop a catalog test.
#[test]
fn every_catalog_test_has_one_valid_route() {
    let routes = rgaa_core::TestRoutePlan::builtin();
    let keys = rgaa_core::RgaaCatalog::all_test_keys();
    assert_eq!(keys.len(), 258);
    assert_eq!(routes.routes().len(), 258);
    for (criterion, key) in keys {
        let route = routes
            .for_test(&criterion, &key)
            .unwrap_or_else(|| panic!("{criterion}/{key}: missing route"));
        assert_eq!(route.fallback, "holo_estimate", "{criterion}/{key}");
        assert!(matches!(
            route.coverage,
            rgaa_core::CoverageLevel::Complete | rgaa_core::CoverageLevel::Partial
        ));
        assert!(std::ptr::eq(
            route,
            rgaa_core::EnginePlan::route_test(&criterion, &key).expect("engine plan resolves test")
        ));
    }
    let problems = routes.check(MechanismRegistry::builtin());
    assert!(problems.is_empty(), "invalid test routes: {problems:#?}");
}

fn route_data() -> serde_json::Value {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../rgaa-core/data/rgaa-4.1.2/test_routes.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("route plan exists"))
        .expect("route JSON parses")
}

fn route_problems(data: &serde_json::Value) -> Vec<String> {
    rgaa_core::TestRoutePlan::from_json_str(&data.to_string())
        .expect("test plan parses")
        .check(MechanismRegistry::builtin())
}

#[test]
fn missing_and_duplicate_routes_identify_the_exact_test() {
    let mut missing = route_data();
    missing.as_array_mut().expect("array").remove(0);
    assert!(route_problems(&missing)
        .iter()
        .any(|p| p.contains("1.1/1") && p.contains("missing")));

    let mut duplicate = route_data();
    let first = duplicate[0].clone();
    duplicate.as_array_mut().expect("array").push(first);
    assert!(route_problems(&duplicate)
        .iter()
        .any(|p| p.contains("1.1/1") && p.contains("duplicate")));
}

#[test]
fn unknown_catalog_pairs_and_mechanisms_are_rejected() {
    for (field, value, expected) in [
        (
            "criterion_id",
            serde_json::json!("99.99"),
            "unknown catalog",
        ),
        ("test_key", serde_json::json!("99"), "unknown catalog"),
        (
            "mechanisms",
            serde_json::json!(["missing-mechanism"]),
            "unknown mechanism",
        ),
        ("fallback", serde_json::json!("holo"), "fallback"),
    ] {
        let mut data = route_data();
        data[0][field] = value;
        assert!(
            route_problems(&data).iter().any(|p| p.contains(expected)),
            "{field}: {expected}"
        );
    }
}

#[test]
fn coverage_and_fallback_declarations_are_required() {
    for (field, value) in [
        ("coverage", None),
        ("coverage", Some(serde_json::json!("estimated"))),
        ("fallback", None),
    ] {
        let mut data = route_data();
        let row = data[0].as_object_mut().expect("row object");
        if let Some(value) = value {
            row.insert(field.into(), value);
        } else {
            row.remove(field);
        }
        let result = rgaa_core::TestRoutePlan::from_json_str(&data.to_string());
        assert!(
            result.is_err(),
            "{field}: invalid declaration must fail parsing"
        );
    }
}

#[test]
fn partial_mechanisms_cannot_cover_undeclared_test_keys() {
    let mut data = route_data();
    let row = data
        .as_array_mut()
        .expect("array")
        .iter_mut()
        .find(|row| row["criterion_id"] == "13.1" && row["test_key"] == "1")
        .expect("13.1/1 exists");
    row["mechanisms"] = serde_json::json!(["axe-13-1"]);
    assert!(route_problems(&data)
        .iter()
        .any(|p| p.contains("13.1/1") && p.contains("explicit")));
}

#[test]
fn a_complete_route_needs_a_complete_deciding_mechanism() {
    let mut data = route_data();
    let row = data
        .as_array_mut()
        .expect("array")
        .iter_mut()
        .find(|row| row["criterion_id"] == "13.1" && row["test_key"] == "1")
        .expect("13.1/1 exists");
    row["coverage"] = serde_json::json!("complete");
    assert!(route_problems(&data)
        .iter()
        .any(|p| p.contains("13.1/1") && p.contains("complete")));
}

#[test]
fn mechanism_routes_cannot_cross_criteria() {
    let mut data = route_data();
    data[0]["mechanisms"] = serde_json::json!(["axe-2-1"]);
    assert!(route_problems(&data)
        .iter()
        .any(|p| p.contains("1.1/1") && p.contains("criterion")));
}

fn probe_registry(coverage: &str, tests: &str, outcomes: &str) -> MechanismRegistry {
    MechanismRegistry::from_toml_str(&format!(
        "[[mechanism]]\nid='probe-test'\ncriterion='1.1'\nkind='js-static'\n\
         engine='obscura'\ncoverage='{coverage}'\ntests={tests}\noutcomes={outcomes}\nlegacy=true"
    ))
    .expect("test registry parses")
}

fn probe_route(coverage: &str) -> rgaa_core::TestRoutePlan {
    rgaa_core::TestRoutePlan::from_json_str(
        &serde_json::json!([{
            "criterion_id": "1.1", "test_key": "1", "mechanisms": ["probe-test"],
            "coverage": coverage, "fallback": "holo_estimate"
        }])
        .to_string(),
    )
    .expect("test route parses")
}

#[test]
fn explicit_test_lists_restrict_even_complete_mechanisms() {
    let registry = probe_registry("complete", "['1.1.2']", "['fail', 'pass']");
    let problems = probe_route("complete").check(&registry);
    assert!(problems
        .iter()
        .any(|p| p.contains("1.1/1") && p.contains("explicit")));
    assert!(problems
        .iter()
        .any(|p| p.contains("1.1/1") && p.contains("no complete")));
}

#[test]
fn declared_partial_tests_are_accepted_without_establishing_pass() {
    for tests in ["['1.1.1']", "['1']"] {
        let registry = probe_registry("partial", tests, "['fail', 'review']");
        let problems = probe_route("partial").check(&registry);
        assert!(
            problems.iter().all(|p| !p.starts_with("1.1/1:")),
            "{problems:?}"
        );
    }
}

#[test]
fn a_partial_mechanism_cannot_declare_pass_in_a_test_route() {
    let registry = probe_registry("partial", "['1.1.1']", "['fail', 'pass']");
    let problems = probe_route("partial").check(&registry);
    assert!(problems
        .iter()
        .any(|p| p.contains("1.1/1") && p.contains("cannot emit pass")));
}

#[test]
fn unknown_or_noncanonical_test_ids_do_not_resolve() {
    let routes = rgaa_core::TestRoutePlan::builtin();
    for (criterion, key) in [
        ("1.1", "9"),
        ("99.99", "1"),
        ("01.1", "1"),
        ("1.1", "1.1.1"),
    ] {
        assert!(routes.for_test(criterion, key).is_none());
        assert!(rgaa_core::EnginePlan::route_test(criterion, key).is_none());
    }
}

/// The registry's public invariant entry point also rejects incomplete route plans.
#[test]
fn registry_check_includes_route_invariants() {
    let registry =
        MechanismRegistry::from_toml_str("mechanism = []").expect("empty registry parses");
    let problems = registry.check(&criteria_dir(), &known_axe_rules());
    assert!(problems
        .iter()
        .any(|p| p.contains("1.1/1") && p.contains("unknown mechanism")));
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
