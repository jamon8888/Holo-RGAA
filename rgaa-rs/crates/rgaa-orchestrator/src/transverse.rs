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
//! 3. le déplacement des findings reste désactivé tant que leurs cibles DOM ne
//!    sont pas conservées avec les résultats : une règle répétée ne prouve pas
//!    que les erreurs appartiennent au landmark détecté.
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

/// Déplace les findings sur une page `transverses` lorsque leur cible est prouvée.
///
/// `Violation` ne conserve pas les sélecteurs de nœuds d'axe-core : un finding
/// ne peut donc pas être relié au sous-arbre transverse sans risquer d'effacer
/// des défauts locaux. L'échantillon reste inchangé jusqu'à ce que ces cibles
/// soient transportées dans le modèle de résultat.
pub fn apply_transverse_pages(
    _pages: &mut Vec<PageResult>,
    _signatures: &[TransverseSignature],
    _audit_type: AuditType,
) {
    // Violation drops axe's target selectors, so a repeated rule cannot prove
    // that its nodes belong to a detected landmark. Keep findings page-local
    // until that provenance is carried through the result model.
}
