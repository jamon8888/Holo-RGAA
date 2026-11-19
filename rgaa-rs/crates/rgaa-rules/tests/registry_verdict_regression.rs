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
//!   sa propre issue : décrites exactement dans [`INTENTIONAL_CHANGES`] et appliquées à
//!   la table gelée avant la comparaison. Seul le changement décrit est admis : toute
//!   autre différence, même sur un critère listé, reste une régression. La liste ne
//!   peut pas pourrir : un critère listé qui ne diffère plus fait échouer
//!   `every_listed_change_really_differs` ;
//! * les règles absentes d'axe-core 4.9.1 (la version injectée) : elles ne lèvent
//!   jamais de violation, donc ne décident aucun verdict ([`NOT_INJECTED`]) ;
//! * les entrées `incomplete` d'axe, prises en compte après la migration (prérequis
//!   bloquant 1 de `human-criteria-automation.md`) : la table d'avant ne les lisait pas.
//!
//! La moitié gap-fix de la migration (`COMPLETE_COVERAGE` → registre) est gardée par
//! `migrated_probe_coverage_matches_the_former_constant` dans `src/gap_fix.rs`, et
//! l'invariant « aucune sonde active sans entrée au registre » (spec §5) par
//! `snippets_and_registry_probes_are_the_same_set`, dans le même fichier.

use rgaa_core::CriterionStatus;
use rgaa_rules::AxeMapper;
use std::collections::{BTreeMap, BTreeSet};

const PRE_REGISTRY: &str = include_str!("data/pre_registry_axe_mapping.json");
const INJECTED: &str = include_str!("../../rgaa-core/data/rgaa-4.1.2/axe_rules_injected.json");

/// Ce qu'un changement assumé fait à l'entrée de la table d'avant. Le changement est
/// décrit exactement : la comparaison porte ensuite sur **tous** les critères, listés ou
/// non, donc une différence de plus sur un critère listé reste une régression.
#[derive(Clone, Copy)]
enum Change {
    /// La couverture passe de `complete` à `partial`, les règles restent les mêmes.
    Partial,
    /// Le critère ne porte plus aucune règle axe.
    Removed,
    /// Le critère, absent de la table d'avant, porte désormais ces règles.
    Added {
        coverage: &'static str,
        rules: &'static [&'static str],
    },
}

/// Changements de verdict assumés depuis la migration, chacun par une issue revue.
/// Aucun ne vient de la migration elle-même : à `9b819a3`, la table dérivée du registre
/// était identique à `axe_mapping.json`.
const INTENTIONAL_CHANGES: &[(&str, Change, &str)] = &[
    (
        "1.1",
        Change::Partial,
        "#366 : partielle, aucune règle ne décide 1.1.4, 1.1.7, 1.1.8",
    ),
    (
        "1.2",
        Change::Partial,
        "#280 puis #366 : partielle, le décoratif ne se lit pas dans le balisage",
    ),
    (
        "1.5",
        Change::Removed,
        "#280 : image-alt ne décide pas 1.5, mapping retiré",
    ),
    (
        "1.6",
        Change::Removed,
        "#280 : image-alt ne décide pas 1.6, mapping retiré",
    ),
    (
        "3.2",
        Change::Partial,
        "#372 : partielle (3.2.5, color-contrast inerte sous ce pipeline, #371)",
    ),
    (
        "5.7",
        Change::Partial,
        "#375 : partielle, cohérence scope/orientation non vérifiée par axe",
    ),
    ("6.1", Change::Partial, "#280 : partielle"),
    (
        "8.5",
        Change::Added {
            coverage: "complete",
            rules: &["document-title"],
        },
        "#280 : document-title déplacée ici depuis 13.3/13.4",
    ),
    ("9.1", Change::Partial, "#280 : partielle"),
    (
        "9.3",
        Change::Partial,
        "#369 : partielle, une liste non balisée échappe à axe",
    ),
    ("10.2", Change::Removed, "#280 : mapping retiré"),
    ("10.5", Change::Removed, "#280 : mapping retiré"),
    (
        "10.6",
        Change::Partial,
        "#373 : partielle, link-in-text-block faussement verte (#371)",
    ),
    (
        "10.8",
        Change::Partial,
        "#374 : partielle, 10.8.1 est un jugement d'intention",
    ),
    ("10.9", Change::Removed, "#280 : mapping retiré"),
    ("11.4", Change::Removed, "#280 : mapping retiré"),
    (
        "11.9",
        Change::Added {
            coverage: "partial",
            rules: &["button-name", "input-button-name"],
        },
        "#280 : button-name et input-button-name ajoutées, partielle",
    ),
    ("12.6", Change::Partial, "#280 : partielle"),
    (
        "12.7",
        Change::Partial,
        "#377 : partielle, 12.7.2 porte sur un ensemble de pages",
    ),
    (
        "13.3",
        Change::Removed,
        "#280 : document-title ne décide pas 13.3, mapping retiré",
    ),
    (
        "13.4",
        Change::Removed,
        "#280 : document-title ne décide pas 13.4, mapping retiré",
    ),
    (
        "13.5",
        Change::Removed,
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

#[derive(serde::Deserialize, Clone)]
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

/// La table d'avant à laquelle on applique exactement les changements assumés : c'est
/// la seule table que le registre a le droit de reproduire.
fn reviewed(frozen: &[Entry]) -> Vec<Entry> {
    let mut out: Vec<Entry> = Vec::new();
    for entry in frozen {
        match change_for(&entry.criterion_id) {
            None => out.push(entry.clone()),
            Some(Change::Partial) => {
                assert_eq!(
                    entry.coverage, "complete",
                    "{}: listed as made partial, but it already was",
                    entry.criterion_id
                );
                out.push(Entry {
                    coverage: "partial".to_string(),
                    ..entry.clone()
                });
            }
            Some(Change::Removed) => {}
            Some(Change::Added { .. }) => panic!(
                "{}: listed as added, but the pre-migration table already mapped it",
                entry.criterion_id
            ),
        }
    }
    for (criterion, change, _) in INTENTIONAL_CHANGES {
        match change {
            Change::Added { coverage, rules } => out.push(Entry {
                criterion_id: (*criterion).to_string(),
                coverage: (*coverage).to_string(),
                axe_rules: rules.iter().map(|r| (*r).to_string()).collect(),
            }),
            Change::Partial | Change::Removed => assert!(
                frozen.iter().any(|e| e.criterion_id == *criterion),
                "{criterion}: listed as changed, but the pre-migration table never mapped it"
            ),
        }
    }
    out
}

fn change_for(criterion: &str) -> Option<Change> {
    INTENTIONAL_CHANGES
        .iter()
        .find(|(c, _, _)| *c == criterion)
        .map(|(_, change, _)| *change)
}

/// Les critères dont le verdict diffère entre les deux tables.
fn differences(expected: &Verdicts, actual: &Verdicts) -> Vec<String> {
    let keys: BTreeSet<&String> = expected.keys().chain(actual.keys()).collect();
    keys.into_iter()
        .filter(|id| expected.get(*id) != actual.get(*id))
        .map(|id| {
            format!(
                "{id}: attendu {:?}, registre {:?}",
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
    let reviewed = reviewed(&frozen());
    let mut regressions = Vec::new();
    for (axe_json, fired) in probes() {
        let expected = legacy_verdicts(&reviewed, fired.as_deref());
        let actual = current_verdicts(&axe_json);
        for difference in differences(&expected, &actual) {
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
        .map(|(c, _, _)| *c)
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
