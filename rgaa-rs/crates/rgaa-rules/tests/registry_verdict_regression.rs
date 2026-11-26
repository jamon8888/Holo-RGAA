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
//! Les différences approuvées après la migration sont appliquées à la table gelée
//! avant de calculer les verdicts attendus, selon la couverture et les règles
//! explicitement revues pour chaque correction.
//!
//! Autres limites de la comparaison :
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

/// Critères dont la couverture complète a été retirée après la migration.
const NO_LONGER_COMPLETE: &[&str] = &[
    "1.1", "1.2", // #366
    "1.5", "1.6", // #280
    "3.2", // #372
    "5.7", // #375
    "6.1", "9.1", // #280
    "9.3", // #369
    "10.2", "10.5", // #280
    "10.6", // #373
    "10.8", // #374
    "10.9", "11.4", "12.6", // #280
    "12.7", // #377
    "13.3", "13.4", "13.5", // #280
];

/// Associations retirées par #280 et #398.
const REMOVED_RULES: &[(&str, &[&str])] = &[
    ("1.5", &["image-alt"]),
    ("1.6", &["image-alt"]),
    ("7.3", &["focus-order-semantics"]),
    ("10.2", &["color-contrast", "image-alt"]),
    ("10.5", &["color-contrast"]),
    ("10.9", &["color-contrast", "image-alt"]),
    ("11.4", &["label"]),
    ("13.3", &["document-title"]),
    ("13.4", &["document-title"]),
    ("13.5", &["image-alt"]),
];

/// Règles de la table d'avant qu'axe-core 4.9.1 ne contient pas : jamais levées, elles
/// ne décidaient rien. `aria-tab-name` a quitté 7.1 avec #261 pour cette raison.
const NOT_INJECTED: &[&str] = &["aria-tab-name"];

/// Mécanismes axe ajoutés après la migration (#263), dont le verdict est plafonné à
/// `review` par `outcome_cap` (#279) : une violation y donne `needs_review`, jamais `fail`.
const NEW_REVIEW_CAPPED: &[(&str, &[&str])] = &[("12.8", &["tabindex", "focus-order-semantics"])];

#[derive(serde::Deserialize)]
struct Frozen {
    mapping: Vec<Entry>,
}

#[derive(serde::Deserialize)]
struct Entry {
    criterion_id: String,
    coverage: String,
    axe_rules: Vec<String>,
    /// Plafond `review` (#279) : une violation donne `NeedsReview`, jamais `Fail`.
    /// Absent de la table gelée, posé seulement par [`reviewed_mapping`].
    #[serde(default)]
    review_capped: bool,
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
/// violation, `Fail` dès qu'une règle du critère a tiré, rien sinon. Une entrée
/// plafonnée à `review` (#279) donne `NeedsReview` au lieu de `Fail`.
fn legacy_verdicts(frozen: &[Entry], fired: Option<&str>) -> Verdicts {
    let mut out = Verdicts::new();
    for entry in frozen {
        if fired.is_some_and(|rule| entry.axe_rules.iter().any(|r| r == rule)) {
            let status = if entry.review_capped {
                CriterionStatus::NeedsReview
            } else {
                CriterionStatus::Fail
            };
            out.insert(entry.criterion_id.clone(), status);
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

/// Applique les corrections approuvées à la référence d'avant migration.
fn reviewed_mapping() -> Vec<Entry> {
    let mut mapping = frozen();
    for id in NO_LONGER_COMPLETE {
        let entry = mapping
            .iter_mut()
            .find(|entry| entry.criterion_id == *id)
            .expect("reviewed coverage change must target a legacy criterion");
        assert_eq!(entry.coverage, "complete", "{id}");
        entry.coverage = "partial".into();
    }
    for (id, rules) in REMOVED_RULES {
        let entry = mapping
            .iter_mut()
            .find(|entry| entry.criterion_id == *id)
            .expect("reviewed rule removal must target a legacy criterion");
        let before = entry.axe_rules.len();
        entry
            .axe_rules
            .retain(|rule| !rules.contains(&rule.as_str()));
        assert_eq!(before - entry.axe_rules.len(), rules.len(), "{id}");
    }

    // #436 : button-name et input-button-name sont tagués RGAA-11.9.1 par
    // axe-core ; ils relèvent de 11.9 et ne doivent plus figurer sous 11.1.
    let entry = mapping
        .iter_mut()
        .find(|entry| entry.criterion_id == "11.1")
        .expect("11.1 doit être présent dans la table gelée");
    let before = entry.axe_rules.len();
    entry
        .axe_rules
        .retain(|rule| rule != "button-name" && rule != "input-button-name");
    assert_eq!(before - entry.axe_rules.len(), 2, "11.1");

    // #280 : titre de page (couverture complète) et noms des boutons (partielle).
    for (id, coverage, rules) in [
        ("8.5", "complete", &["document-title"][..]),
        ("11.9", "partial", &["button-name", "input-button-name"][..]),
    ] {
        assert!(mapping.iter().all(|entry| entry.criterion_id != id), "{id}");
        mapping.push(Entry {
            criterion_id: id.into(),
            coverage: coverage.into(),
            axe_rules: rules.iter().map(|rule| (*rule).into()).collect(),
            review_capped: false,
        });
    }

    // #263 / #279 : ordre de tabulation, indice plafonné à `review`.
    assert!(mapping.iter().all(|entry| entry.criterion_id != "12.8"));
    mapping.push(Entry {
        criterion_id: "12.8".into(),
        coverage: "partial".into(),
        axe_rules: vec!["tabindex".into(), "focus-order-semantics".into()],
        review_capped: true,
    });
    mapping
}

/// Les critères dont le verdict diffère entre les deux tables.
fn differences(expected: &Verdicts, actual: &Verdicts) -> Vec<String> {
    let keys: BTreeSet<&String> = expected.keys().chain(actual.keys()).collect();
    keys.into_iter()
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
    let mapping = reviewed_mapping();
    let mut regressions = Vec::new();
    for (axe_json, fired) in probes() {
        let mut expected = legacy_verdicts(&mapping, fired.as_deref());
        // #263/#279 : mécanismes nouveaux, plafonnés à `review` (jamais `fail`).
        for (id, rules) in NEW_REVIEW_CAPPED {
            if fired.as_deref().is_some_and(|rule| rules.contains(&rule)) {
                expected.insert((*id).into(), CriterionStatus::NeedsReview);
            }
        }
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
