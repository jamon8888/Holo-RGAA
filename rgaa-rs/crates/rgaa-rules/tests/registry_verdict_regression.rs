//! Aucune régression de verdict à la migration vers le registre unique (#261).
//!
//! `mechanisms.toml` a remplacé `axe_mapping.json`. Le critère d'acceptation de #261 :
//! les résultats d'audit sont identiques avant et après. Ce test le prouve sur le seul
//! étage que la migration a touché, `AxeMapper`, en le confrontant à la table d'avant
//! la migration, gelée dans `tests/data/pre_registry_axe_mapping.json`.
//!
//! Le verdict d'`AxeMapper` est une fonction de la table et de la sortie d'axe, et il se
//! compose règle par règle : un critère à couverture complète vaut `Pass` sans violation,
//! et toute violation d'une de ses règles le fait passer à `Fail`. Le confronter à la
//! table gelée sur la sortie vide et sur une violation isolée de **chaque** règle que la
//! version injectée d'axe-core peut lever couvre donc tous les verdicts possibles.
//!
//! Ce qui n'entre pas dans la comparaison, et pourquoi :
//! * les corrections de couverture **postérieures** à la migration, chacune décidée par
//!   sa propre issue : listées dans [`INTENTIONAL_CHANGES`]. La liste ne peut pas
//!   pourrir : un critère listé qui ne diffère plus fait échouer
//!   `every_listed_change_really_differs` ;
//! * les règles absentes d'axe-core 4.9.1 (la version injectée) : elles ne lèvent
//!   jamais de violation, donc ne décident aucun verdict ([`NOT_INJECTED`]) ;
//! * les entrées `incomplete` d'axe, prises en compte après la migration (prérequis
//!   bloquant 1 de `human-criteria-automation.md`) : la table d'avant ne les lisait pas.
//!
//! La moitié gap-fix de la migration (`COMPLETE_COVERAGE` → registre) est gardée par
//! `migrated_probe_coverage_matches_the_former_constant` dans `src/gap_fix.rs`.

use rgaa_core::CriterionStatus;
use rgaa_rules::AxeMapper;
use std::collections::{BTreeMap, BTreeSet};

const PRE_REGISTRY: &str = include_str!("data/pre_registry_axe_mapping.json");
const INJECTED: &str = include_str!("../../rgaa-core/data/rgaa-4.1.2/axe_rules_injected.json");

/// Changements de verdict assumés depuis la migration, chacun par une issue revue.
/// Aucun ne vient de la migration elle-même : à `9b819a3`, la table dérivée du registre
/// était identique à `axe_mapping.json`.
const INTENTIONAL_CHANGES: &[(&str, &str)] = &[
    (
        "1.1",
        "#366 : partielle, aucune règle ne décide 1.1.4, 1.1.7, 1.1.8",
    ),
    (
        "1.2",
        "#280 puis #366 : partielle, le décoratif ne se lit pas dans le balisage",
    ),
    ("1.5", "#280 : image-alt ne décide pas 1.5, mapping retiré"),
    ("1.6", "#280 : image-alt ne décide pas 1.6, mapping retiré"),
    (
        "3.2",
        "#372 : partielle (3.2.5, color-contrast inerte sous ce pipeline, #371)",
    ),
    (
        "5.7",
        "#375 : partielle, cohérence scope/orientation non vérifiée par axe",
    ),
    ("6.1", "#280 : partielle"),
    ("8.5", "#280 : document-title déplacée ici depuis 13.3/13.4"),
    ("9.1", "#280 : partielle"),
    (
        "9.3",
        "#369 : partielle, une liste non balisée échappe à axe",
    ),
    ("10.2", "#280 : mapping retiré"),
    ("10.5", "#280 : mapping retiré"),
    (
        "10.6",
        "#373 : partielle, link-in-text-block faussement verte (#371)",
    ),
    (
        "10.8",
        "#374 : partielle, 10.8.1 est un jugement d'intention",
    ),
    ("10.9", "#280 : mapping retiré"),
    ("11.4", "#280 : mapping retiré"),
    (
        "11.9",
        "#280 : button-name et input-button-name ajoutées, partielle",
    ),
    ("12.6", "#280 : partielle"),
    (
        "12.7",
        "#377 : partielle, 12.7.2 porte sur un ensemble de pages",
    ),
    (
        "13.3",
        "#280 : document-title ne décide pas 13.3, mapping retiré",
    ),
    (
        "13.4",
        "#280 : document-title ne décide pas 13.4, mapping retiré",
    ),
    (
        "13.5",
        "#280 : image-alt ne décide pas 13.5, mapping retiré",
    ),
];

/// Règles de la table d'avant qu'axe-core 4.9.1 ne contient pas : jamais levées, elles
/// ne décidaient rien. `aria-tab-name` a quitté 7.1 avec #261 pour cette raison.
const NOT_INJECTED: &[&str] = &["aria-tab-name"];

#[derive(serde::Deserialize)]
struct Frozen {
    mapping: Vec<Entry>,
}

#[derive(serde::Deserialize)]
struct Entry {
    criterion_id: String,
    coverage: String,
    axe_rules: Vec<String>,
}

#[derive(serde::Deserialize)]
struct Injected {
    rules: Vec<String>,
}

type Verdicts = BTreeMap<String, CriterionStatus>;

fn frozen() -> Vec<Entry> {
    let frozen: Frozen =
        serde_json::from_str(PRE_REGISTRY).expect("pre_registry_axe_mapping.json must parse");
    assert_eq!(
        frozen.mapping.len(),
        43,
        "43 criteria carried axe rules before the migration"
    );
    frozen.mapping
}

fn injected_rules() -> Vec<String> {
    let injected: Injected =
        serde_json::from_str(INJECTED).expect("axe_rules_injected.json must parse");
    injected.rules
}

/// Le verdict que la table d'avant donnait : `Pass` pour une couverture complète sans
/// violation, `Fail` dès qu'une règle du critère a tiré, rien sinon.
fn legacy_verdicts(frozen: &[Entry], fired: Option<&str>) -> Verdicts {
    let mut out = Verdicts::new();
    for entry in frozen {
        if fired.is_some_and(|rule| entry.axe_rules.iter().any(|r| r == rule)) {
            out.insert(entry.criterion_id.clone(), CriterionStatus::Fail);
        } else if entry.coverage == "complete" {
            out.insert(entry.criterion_id.clone(), CriterionStatus::Pass);
        }
    }
    out
}

fn current_verdicts(axe_json: &str) -> Verdicts {
    AxeMapper::map(axe_json)
        .expect("synthetic axe output must map")
        .into_iter()
        .map(|(id, result)| (id, result.status))
        .collect()
}

fn violation(rule: &str) -> String {
    format!(
        r#"[{{"id":"{rule}","impact":"serious","description":"synthetic {rule}","nodes":[{{"html":"<x/>"}}]}}]"#
    )
}

fn is_intentional(criterion: &str) -> bool {
    INTENTIONAL_CHANGES.iter().any(|(c, _)| *c == criterion)
}

/// Les critères dont le verdict diffère entre les deux tables, hors liste assumée si
/// `skip_intentional`.
fn differences(expected: &Verdicts, actual: &Verdicts, skip_intentional: bool) -> Vec<String> {
    let keys: BTreeSet<&String> = expected.keys().chain(actual.keys()).collect();
    keys.into_iter()
        .filter(|id| !(skip_intentional && is_intentional(id)))
        .filter(|id| expected.get(*id) != actual.get(*id))
        .map(|id| {
            format!(
                "{id}: avant {:?}, registre {:?}",
                expected.get(id),
                actual.get(id)
            )
        })
        .collect()
}

/// Les sorties d'axe sondées : aucune violation, puis une violation isolée de chaque
/// règle de la version injectée.
fn probes() -> Vec<(String, Option<String>)> {
    let mut probes = vec![("[]".to_string(), None)];
    for rule in injected_rules() {
        probes.push((violation(&rule), Some(rule)));
    }
    probes
}

#[test]
fn verdicts_are_unchanged_except_reviewed_changes() {
    let frozen = frozen();
    let mut regressions = Vec::new();
    for (axe_json, fired) in probes() {
        let expected = legacy_verdicts(&frozen, fired.as_deref());
        let actual = current_verdicts(&axe_json);
        for difference in differences(&expected, &actual, true) {
            regressions.push(format!(
                "[{}] {difference}",
                fired.as_deref().unwrap_or("aucune violation")
            ));
        }
    }
    assert!(
        regressions.is_empty(),
        "{} verdict(s) changed by the registry without a reviewed reason:\n  {}",
        regressions.len(),
        regressions.join("\n  ")
    );
}

/// Un changement assumé qui ne change plus rien est une exemption morte : elle
/// masquerait une régression future sur ce critère.
#[test]
fn every_listed_change_really_differs() {
    let frozen = frozen();
    let mut changed: BTreeSet<String> = BTreeSet::new();
    for (axe_json, fired) in probes() {
        let expected = legacy_verdicts(&frozen, fired.as_deref());
        let actual = current_verdicts(&axe_json);
        let keys: BTreeSet<&String> = expected.keys().chain(actual.keys()).collect();
        for id in keys {
            if expected.get(id) != actual.get(id) {
                changed.insert(id.clone());
            }
        }
    }
    let stale: Vec<&str> = INTENTIONAL_CHANGES
        .iter()
        .map(|(c, _)| *c)
        .filter(|c| !changed.contains(*c))
        .collect();
    assert!(
        stale.is_empty(),
        "listed as intentional but no verdict differs any more, remove them: {stale:?}"
    );
}

/// Les règles exclues de la comparaison sont exactement celles de la table d'avant
/// qu'axe-core 4.9.1 ne contient pas : aucune autre ne peut s'y glisser en silence.
#[test]
fn only_rules_absent_from_the_injected_axe_are_left_out() {
    let injected: BTreeSet<String> = injected_rules().into_iter().collect();
    let left_out: Vec<String> = frozen()
        .iter()
        .flat_map(|e| e.axe_rules.iter())
        .filter(|rule| !injected.contains(*rule))
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    assert_eq!(left_out, NOT_INJECTED);
}
