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
//! Ce qui n'entre pas dans la comparaison telle quelle, et pourquoi :
//! * les corrections de couverture **postérieures** à la migration, chacune décidée par
//!   sa propre issue : listées dans [`REVIEWED_CHANGES`], chacune comme un écart exact
//!   — verdict sur le silence d'axe avant → après, règles qui font échouer le critère
//!   gagnées et perdues. La table attendue est la table gelée **plus ces seuls écarts**,
//!   et elle est comparée à `AxeMapper` sur chaque sonde, sans exemption : un autre
//!   écart sur un critère listé (une règle de plus perdue par 8.5, par exemple) échoue
//!   comme partout ailleurs. Un écart déclaré dont le « avant » ne correspond plus à la
//!   table gelée, ou qui ne change rien, fait échouer
//!   `every_reviewed_change_starts_from_the_frozen_table` ;
//! * les règles absentes d'axe-core 4.9.1 (la version injectée) : elles ne lèvent
//!   jamais de violation, donc ne décident aucun verdict. Les sondes ne tirent que les
//!   règles injectées ; [`NOT_INJECTED`] épingle l'ensemble exact de règles de la table
//!   d'avant que ce choix laisse de côté
//!   (`only_rules_absent_from_the_injected_axe_are_left_out`) ;
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

/// Ce que vaut un critère quand axe ne lève rien : `Pass` (couverture complète) ou
/// aucun verdict (couverture partielle, ou critère absent de la table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Silence {
    Pass,
    Nothing,
}

/// Un changement de verdict assumé depuis la migration, décrit exactement.
///
/// Le verdict d'`AxeMapper` sur un critère est entièrement fixé par deux données : ce
/// qu'il vaut sur le silence d'axe, et l'ensemble des règles dont une violation le fait
/// échouer. Un écart se décrit donc par ces deux données avant → après ; il porte, sonde
/// par sonde, exactement les paires (critère × sonde × verdict avant/après) qu'il
/// change, et aucune autre.
struct ReviewedChange {
    criterion: &'static str,
    /// Verdict sur le silence d'axe : (table gelée, registre).
    silence: (Silence, Silence),
    /// Règles (injectées) qui font désormais échouer le critère.
    gains: &'static [&'static str],
    /// Règles (injectées) qui ne le font plus échouer.
    loses: &'static [&'static str],
    why: &'static str,
}

const fn change(
    criterion: &'static str,
    silence: (Silence, Silence),
    gains: &'static [&'static str],
    loses: &'static [&'static str],
    why: &'static str,
) -> ReviewedChange {
    ReviewedChange {
        criterion,
        silence,
        gains,
        loses,
        why,
    }
}

const COMPLETE_TO_PARTIAL: (Silence, Silence) = (Silence::Pass, Silence::Nothing);
const UNCHANGED_NOTHING: (Silence, Silence) = (Silence::Nothing, Silence::Nothing);

/// Changements de verdict assumés depuis la migration, chacun par une issue revue.
/// Aucun ne vient de la migration elle-même : à `9b819a3`, la table dérivée du registre
/// était identique à `axe_mapping.json`.
const REVIEWED_CHANGES: &[ReviewedChange] = &[
    change(
        "1.1",
        COMPLETE_TO_PARTIAL,
        &[],
        &[],
        "#366 : partielle, aucune règle ne décide 1.1.4, 1.1.7, 1.1.8",
    ),
    change(
        "1.2",
        COMPLETE_TO_PARTIAL,
        &[],
        &[],
        "#280 puis #366 : partielle, le décoratif ne se lit pas dans le balisage",
    ),
    change(
        "1.5",
        COMPLETE_TO_PARTIAL,
        &[],
        &["image-alt"],
        "#280 : image-alt ne décide pas 1.5, mapping retiré",
    ),
    change(
        "1.6",
        COMPLETE_TO_PARTIAL,
        &[],
        &["image-alt"],
        "#280 : image-alt ne décide pas 1.6, mapping retiré",
    ),
    change(
        "3.2",
        COMPLETE_TO_PARTIAL,
        &[],
        &[],
        "#372 : partielle (3.2.5, color-contrast inerte sous ce pipeline, #371)",
    ),
    change(
        "5.7",
        COMPLETE_TO_PARTIAL,
        &[],
        &[],
        "#375 : partielle, cohérence scope/orientation non vérifiée par axe",
    ),
    change("6.1", COMPLETE_TO_PARTIAL, &[], &[], "#280 : partielle"),
    change(
        "8.5",
        (Silence::Nothing, Silence::Pass),
        &["document-title"],
        &[],
        "#280 : document-title déplacée ici depuis 13.3/13.4",
    ),
    change("9.1", COMPLETE_TO_PARTIAL, &[], &[], "#280 : partielle"),
    change(
        "9.3",
        COMPLETE_TO_PARTIAL,
        &[],
        &[],
        "#369 : partielle, une liste non balisée échappe à axe",
    ),
    change(
        "10.2",
        COMPLETE_TO_PARTIAL,
        &[],
        &["color-contrast", "image-alt"],
        "#280 : mapping retiré",
    ),
    change(
        "10.5",
        COMPLETE_TO_PARTIAL,
        &[],
        &["color-contrast"],
        "#280 : mapping retiré",
    ),
    change(
        "10.6",
        COMPLETE_TO_PARTIAL,
        &[],
        &[],
        "#373 : partielle, link-in-text-block faussement verte (#371)",
    ),
    change(
        "10.8",
        COMPLETE_TO_PARTIAL,
        &[],
        &[],
        "#374 : partielle, 10.8.1 est un jugement d'intention",
    ),
    change(
        "10.9",
        COMPLETE_TO_PARTIAL,
        &[],
        &["color-contrast", "image-alt"],
        "#280 : mapping retiré",
    ),
    change(
        "11.4",
        COMPLETE_TO_PARTIAL,
        &[],
        &["label"],
        "#280 : mapping retiré",
    ),
    change(
        "11.9",
        UNCHANGED_NOTHING,
        &["button-name", "input-button-name"],
        &[],
        "#280 : button-name et input-button-name ajoutées, partielle",
    ),
    change("12.6", COMPLETE_TO_PARTIAL, &[], &[], "#280 : partielle"),
    change(
        "12.7",
        COMPLETE_TO_PARTIAL,
        &[],
        &[],
        "#377 : partielle, 12.7.2 porte sur un ensemble de pages",
    ),
    change(
        "13.3",
        COMPLETE_TO_PARTIAL,
        &[],
        &["document-title"],
        "#280 : document-title ne décide pas 13.3, mapping retiré",
    ),
    change(
        "13.4",
        COMPLETE_TO_PARTIAL,
        &[],
        &["document-title"],
        "#280 : document-title ne décide pas 13.4, mapping retiré",
    ),
    change(
        "13.5",
        COMPLETE_TO_PARTIAL,
        &[],
        &["image-alt"],
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

/// Ce qu'une table dit d'un critère : son verdict sur le silence, et les règles qui le
/// font échouer.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Behaviour {
    silence: Silence,
    fails_on: BTreeSet<String>,
}

type Table = BTreeMap<String, Behaviour>;

/// Nombre d'entrées de `pre_registry_axe_mapping.json` : les critères qui portaient des
/// règles axe avant la migration. Ce fichier est gelé ; le régénérer est un changement
/// de la référence elle-même, qui doit mettre ce nombre à jour en connaissance de cause.
const FROZEN_ENTRIES: usize = 43;

fn frozen() -> Vec<Entry> {
    let frozen: Frozen =
        serde_json::from_str(PRE_REGISTRY).expect("pre_registry_axe_mapping.json must parse");
    assert_eq!(
        frozen.mapping.len(),
        FROZEN_ENTRIES,
        "pre_registry_axe_mapping.json est la table gelée d'avant la migration (43 critères \
         portant des règles axe) : elle ne doit pas changer. Si elle a été régénérée \
         délibérément, mettre à jour FROZEN_ENTRIES — et revoir REVIEWED_CHANGES, dont \
         chaque « avant » est lu dans cette table"
    );
    frozen.mapping
}

fn injected_rules() -> Vec<String> {
    let injected: Injected =
        serde_json::from_str(INJECTED).expect("axe_rules_injected.json must parse");
    injected.rules
}

/// La table d'avant la migration, restreinte aux règles que la version injectée peut
/// lever (les autres ne décident rien).
fn legacy_table(frozen: &[Entry], injected: &BTreeSet<String>) -> Table {
    frozen
        .iter()
        .map(|entry| {
            (
                entry.criterion_id.clone(),
                Behaviour {
                    silence: if entry.coverage == "complete" {
                        Silence::Pass
                    } else {
                        Silence::Nothing
                    },
                    fails_on: entry
                        .axe_rules
                        .iter()
                        .filter(|rule| injected.contains(*rule))
                        .cloned()
                        .collect(),
                },
            )
        })
        .collect()
}

fn absent() -> Behaviour {
    Behaviour {
        silence: Silence::Nothing,
        fails_on: BTreeSet::new(),
    }
}

/// La table gelée avec les seuls écarts revus appliqués. Panique si un écart ne part pas
/// exactement de la table gelée : son « avant » doit être vrai.
fn expected_table(legacy: &Table) -> Table {
    let mut table = legacy.clone();
    for change in REVIEWED_CHANGES {
        let before = legacy.get(change.criterion).cloned().unwrap_or_else(absent);
        let (silence_before, silence_after) = change.silence;
        assert_eq!(
            before.silence, silence_before,
            "{} ({}) : la table gelée vaut {:?} sur le silence, l'écart déclaré part de {:?}",
            change.criterion, change.why, before.silence, silence_before
        );
        let mut after = before.clone();
        after.silence = silence_after;
        for rule in change.loses {
            assert!(
                after.fails_on.remove(*rule),
                "{} ({}) : déclarée perdue, {rule} ne faisait pas échouer le critère avant",
                change.criterion,
                change.why
            );
        }
        for rule in change.gains {
            assert!(
                after.fails_on.insert((*rule).to_string()),
                "{} ({}) : déclarée gagnée, {rule} faisait déjà échouer le critère avant",
                change.criterion,
                change.why
            );
        }
        assert_ne!(
            before, after,
            "{} ({}) : l'écart déclaré ne change rien — exemption morte, à retirer",
            change.criterion, change.why
        );
        if after == absent() {
            table.remove(change.criterion);
        } else {
            table.insert(change.criterion.to_string(), after);
        }
    }
    table
}

/// Le verdict qu'une table donne : `Fail` dès qu'une règle du critère a tiré, sinon son
/// verdict sur le silence (`Pass` ou rien).
fn table_verdicts(table: &Table, fired: Option<&str>) -> Verdicts {
    let mut out = Verdicts::new();
    for (id, behaviour) in table {
        if fired.is_some_and(|rule| behaviour.fails_on.contains(rule)) {
            out.insert(id.clone(), CriterionStatus::Fail);
        } else if behaviour.silence == Silence::Pass {
            out.insert(id.clone(), CriterionStatus::Pass);
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

/// Sur chaque sonde, le registre donne exactement le verdict de la table gelée modifiée
/// des seuls écarts revus — critère par critère, sans exemption.
#[test]
fn verdicts_are_unchanged_except_reviewed_changes() {
    let injected: BTreeSet<String> = injected_rules().into_iter().collect();
    let expected = expected_table(&legacy_table(&frozen(), &injected));
    let mut regressions = Vec::new();
    for (axe_json, fired) in probes() {
        let want = table_verdicts(&expected, fired.as_deref());
        let actual = current_verdicts(&axe_json);
        for difference in differences(&want, &actual) {
            regressions.push(format!(
                "[{}] {difference}",
                fired.as_deref().unwrap_or("aucune violation")
            ));
        }
    }
    assert!(
        regressions.is_empty(),
        "{} verdict(s) changed by the registry beyond the reviewed changes:\n  {}",
        regressions.len(),
        regressions.join("\n  ")
    );
}

/// Chaque écart revu part de la table gelée et change quelque chose ; aucun critère
/// n'est listé deux fois.
#[test]
fn every_reviewed_change_starts_from_the_frozen_table() {
    let injected: BTreeSet<String> = injected_rules().into_iter().collect();
    // `expected_table` vérifie le « avant » et l'effet de chaque écart.
    let _ = expected_table(&legacy_table(&frozen(), &injected));
    let mut seen = BTreeSet::new();
    for change in REVIEWED_CHANGES {
        assert!(
            seen.insert(change.criterion),
            "{} listé deux fois",
            change.criterion
        );
        for rule in change.gains.iter().chain(change.loses) {
            assert!(
                injected.contains(*rule),
                "{} : {rule} n'est pas injectée, elle ne décide aucun verdict",
                change.criterion
            );
        }
    }
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
