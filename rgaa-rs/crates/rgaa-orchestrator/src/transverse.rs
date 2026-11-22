//! Détection et évaluation unique des éléments transverses (ticket #429).
//!
//! Ara porte les éléments répétés (en-tête, menu, pied) sur une page virtuelle
//! `transverses`, évaluée une fois pour tout l'échantillon. Le présent module
//! rejoue ce modèle sur les résultats de l'orchestrateur :
//!
//! 1. la sonde DOM d'Obscura décrit les points de repère `banner`,
//!    `navigation` et `contentinfo` par la structure normalisée de leur sous-arbre ;
//! 2. [`detect_transverse_landmarks`] retient les signatures présentes sur
//!    **plus de 80 %** des pages (le seuil est strict, et il faut au moins deux pages) ;
//! 3. [`apply_transverse_pages`] ne laisse qu'un exemplaire de chaque finding
//!    répété sur la page `transverses`. Les pages de l'échantillon conservent
//!    leur statut de critère (le sites en reste NC), mais plus aucune preuve ni
//!    décision locale : une seule décision nominative vaut pour toutes.
//!
//! La normalisation ignore les attributs volatils (`id`, classes générées,
//! états d'interface) : elle ne retient que les balises, rôles, libellés et
//! textes en minuscules et à espaces réduits. Un contenu dynamique change donc
//! la signature et n'est **pas** déclaré transverse — choix conservateur qui
//! évite de masquer un défaut propre à une page.

use indexmap::IndexMap;
use rgaa_core::{AuditType, CriterionResult, PageResult};
use serde::Deserialize;
use std::collections::BTreeSet;

pub use rgaa_core::{TRANSVERSE_PAGE_URL, TRANSVERSE_REFERENCE_SOURCE};

/// Seuil de répétition : strictement plus de 80 % des pages de l'échantillon.
pub const TRANSVERSE_THRESHOLD: f64 = 0.8;
/// Équivalent entier exact du seuil, pour éviter les comparaisons flottantes.
const THRESHOLD_NUMERATOR: usize = 4;
const THRESHOLD_DENOMINATOR: usize = 5;

/// Titre lisible de la page virtuelle dans le rapport.
const TRANSVERSE_TITLE: &str = "Éléments transverses";
/// Raison affichée sur la page virtuelle : une seule décision pour l'échantillon.
const TRANSVERSE_DECISION_REASON: &str =
    "Une seule décision vaut pour toutes les pages de l'échantillon";

/// Un nœud du sous-arbre d'un point de repère, tel que la sonde DOM le décrit.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LandmarkNode {
    pub tag: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub text: String,
}

/// Le sous-arbre normalisé d'un point de repère `banner`, `navigation` ou
/// `contentinfo`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LandmarkSubtree {
    pub kind: String,
    #[serde(default)]
    pub nodes: Vec<LandmarkNode>,
}

/// Forme brute du champ `transverse_landmarks` du contexte Obscura.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PageTransverseContext {
    #[serde(default)]
    pub transverse_landmarks: Vec<LandmarkSubtree>,
}

/// Points de repère observés sur une page de l'échantillon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageLandmarks {
    pub url: String,
    pub landmarks: Vec<LandmarkSubtree>,
}

/// Identité d'un bloc répété : son genre de point de repère et le hash de son
/// sous-arbre normalisé.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TransverseSignature {
    pub kind: String,
    pub hash: String,
}

impl TransverseSignature {
    #[must_use]
    pub fn new(kind: impl Into<String>, hash: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            hash: hash.into(),
        }
    }
}

/// FNV-1a 64 bits, déterministe et sans dépendance : le hash sert à comparer
/// des sous-arbres entre pages, pas à authentifier du contenu.
fn fnv1a64(input: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

/// Hash d'un sous-arbre normalisé. La forme canonique est
/// `balise\trole\tlibellé\ttexte` par nœud, les nœuds étant joints par `\n`.
#[must_use]
pub fn normalized_subtree_hash(subtree: &LandmarkSubtree) -> String {
    // La sonde fournit déjà des minuscules et des espaces réduits ; la
    // normalisation est répétée ici pour que la signature soit stable même si
    // une sonde plus ancienne renvoie la casse d'origine du DOM.
    let normalize = |value: &str| {
        value
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    };
    let canonical = subtree
        .nodes
        .iter()
        .map(|node| {
            format!(
                "{}\t{}\t{}\t{}",
                normalize(&node.tag),
                normalize(&node.role),
                normalize(&node.label),
                normalize(&node.text)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("fnv1a64:{:016x}", fnv1a64(&canonical))
}

fn is_transverse_kind(kind: &str) -> bool {
    matches!(kind, "banner" | "navigation" | "contentinfo")
}

fn above_threshold(count: usize, total: usize) -> bool {
    count * THRESHOLD_DENOMINATOR > total * THRESHOLD_NUMERATOR
}

/// Signatures présentes sur plus de 80 % des pages, dans l'ordre où elles ont
/// été vues pour la première fois (ordre déterministe, indépendant de HashMap).
#[must_use]
pub fn detect_transverse_landmarks(pages: &[PageLandmarks]) -> Vec<TransverseSignature> {
    if pages.len() < 2 {
        return Vec::new();
    }

    // Un même bloc présent deux fois sur une page ne compte qu'une fois pour
    // cette page : c'est la couverture de l'échantillon qui est mesurée.
    let mut counts: IndexMap<TransverseSignature, usize> = IndexMap::new();
    for page in pages {
        let on_this_page: BTreeSet<TransverseSignature> = page
            .landmarks
            .iter()
            .filter(|landmark| is_transverse_kind(&landmark.kind))
            .map(|landmark| {
                TransverseSignature::new(landmark.kind.as_str(), normalized_subtree_hash(landmark))
            })
            .collect();
        for signature in on_this_page {
            *counts.entry(signature).or_insert(0) += 1;
        }
    }

    counts
        .into_iter()
        .filter(|(_, count)| above_threshold(*count, pages.len()))
        .map(|(signature, _)| signature)
        .collect()
}

/// Page virtuelle portant, pour un critère donné, l'unique finding transverse.
///
/// Ne modifie pas l'échantillon : l'appelant décide ensuite quand l'ajouter.
#[must_use]
pub fn transverse_page_from(pages: &[PageResult], criterion_id: &str) -> Option<PageResult> {
    let mut source = pages
        .iter()
        .flat_map(|page| page.criteria.iter())
        .find(|criterion| {
            criterion.criterion_id == criterion_id && !criterion.violations.is_empty()
        })?
        .clone();
    source.violations.truncate(1);
    source.review_required = true;
    source.review_reason = Some(TRANSVERSE_DECISION_REASON.to_string());
    source.verified_status = None;
    source.review_events.clear();

    Some(PageResult {
        url: TRANSVERSE_PAGE_URL.to_string(),
        title: Some(TRANSVERSE_TITLE.to_string()),
        compliance_rate: rgaa_report::compliance_rate(std::slice::from_ref(&source)),
        criteria: vec![source],
        crawl_depth: 0,
    })
}

fn repeated_pairs(pages: &[PageResult]) -> IndexMap<(String, String), usize> {
    let mut counts: IndexMap<(String, String), usize> = IndexMap::new();
    for page in pages {
        let on_this_page: BTreeSet<(String, String)> = page
            .criteria
            .iter()
            .flat_map(|criterion| {
                criterion.violations.iter().map(move |violation| {
                    (criterion.criterion_id.clone(), violation.rule_id.clone())
                })
            })
            .collect();
        for pair in on_this_page {
            *counts.entry(pair).or_insert(0) += 1;
        }
    }
    counts
}

fn prepare_transverse_criterion(criterion: &mut CriterionResult, pairs: &[(String, String)]) {
    criterion.violations.retain(|violation| {
        !pairs.iter().any(|(criterion_id, rule_id)| {
            *criterion_id == criterion.criterion_id && *rule_id == violation.rule_id
        })
    });
    if criterion.violations.is_empty() {
        // Plus de preuve locale : la page renvoie à la page transverses, qui
        // porte l'unique finding et l'unique décision (R5).
        criterion.source = TRANSVERSE_REFERENCE_SOURCE.to_string();
        criterion.review_required = false;
        criterion.review_reason = None;
        criterion.verified_status = None;
        criterion.review_events.clear();
    }
}

/// Déplace les findings répétés sur une page `transverses` unique.
///
/// Les signatures de points de repère sont le préalable : sans élément transverse
/// détecté, aucun finding n'est déplacé. Seuls les findings eux-mêmes répétés
/// sur plus de 80 % des pages le sont, ce qui évite de masquer un défaut local.
pub fn apply_transverse_pages(
    pages: &mut Vec<PageResult>,
    signatures: &[TransverseSignature],
    audit_type: AuditType,
) {
    if signatures.is_empty() || pages.len() < 2 {
        return;
    }

    let repeated: Vec<(String, String)> = repeated_pairs(pages)
        .into_iter()
        .filter(|(_, count)| above_threshold(*count, pages.len()))
        .map(|(pair, _)| pair)
        .collect();
    if repeated.is_empty() {
        return;
    }

    // La page transverses hérite de tous les critères d'une page témoin, pour
    // que le garde-fou de couverture automatique reste satisfait, mais ne porte
    // comme preuves que les findings transverses.
    let mut transverse = pages[0].clone();
    transverse.url = TRANSVERSE_PAGE_URL.to_string();
    transverse.title = Some(TRANSVERSE_TITLE.to_string());
    transverse.crawl_depth = 0;
    for criterion in &mut transverse.criteria {
        if !audit_type.contains(&criterion.criterion_id) {
            continue;
        }
        let matching: Vec<&str> = repeated
            .iter()
            .filter(|(id, _)| id == &criterion.criterion_id)
            .map(|(_, rule)| rule.as_str())
            .collect();
        if matching.is_empty() {
            criterion.violations.clear();
            continue;
        }
        // La page témoin ne porte pas forcément le finding (il suffit qu'il
        // soit présent sur plus de 80 % des pages) : le reprendre à la première
        // page qui l'a observé, une seule fois par règle.
        let mut kept = Vec::with_capacity(matching.len());
        for rule in matching {
            if let Some(violation) = criterion
                .violations
                .iter()
                .find(|violation| violation.rule_id == rule)
                .cloned()
                .or_else(|| {
                    pages
                        .iter()
                        .flat_map(|page| page.criteria.iter())
                        .filter(|candidate| candidate.criterion_id == criterion.criterion_id)
                        .find_map(|candidate| {
                            candidate
                                .violations
                                .iter()
                                .find(|violation| violation.rule_id == rule)
                        })
                        .cloned()
                })
            {
                kept.push(violation);
            }
        }
        criterion.violations = kept;
        criterion.review_required = true;
        criterion.review_reason = Some(TRANSVERSE_DECISION_REASON.to_string());
        criterion.verified_status = None;
        criterion.review_events.clear();
    }
    transverse
        .criteria
        .retain(|criterion| audit_type.contains(&criterion.criterion_id));
    transverse.compliance_rate = rgaa_report::compliance_rate(&transverse.criteria);

    for page in pages.iter_mut() {
        for criterion in &mut page.criteria {
            prepare_transverse_criterion(criterion, &repeated);
        }
        page.compliance_rate = rgaa_report::compliance_rate(&page.criteria);
    }
    pages.push(transverse);
}
