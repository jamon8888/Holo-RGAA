//! Cache de décision par pHash (P-03 §5).
//!
//! Clé = `(pHash, alt, contexte_hash)`. Une décision de revue portée sur une
//! clé est proposée en lot aux copies, mais c'est toujours le relecteur qui
//! confirme (R5) : ce module ne pose jamais de statut.

use crate::phash;
use indexmap::IndexMap;

/// Une entrée du cache, avec les pages où l'image a été vue.
#[derive(Debug, Clone, PartialEq)]
pub struct EntreeDecision {
    pub p_hash: u64,
    pub alt: String,
    pub contexte: String,
    pub pages: Vec<String>,
    pub decision: Option<String>,
    pub deja_analyse: bool,
}

/// Cache déterministe : l'ordre d'insertion est conservé (IndexMap).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DecisionCache {
    entrees: IndexMap<String, EntreeDecision>,
}

impl DecisionCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clé stable `(pHash, alt, contexte_hash)`.
    #[must_use]
    pub fn cle(p_hash: u64, alt: &str, contexte: &str) -> String {
        let contexte_hash = rgaa_analysis::sha256_hex(format!("{alt}\0{contexte}").as_bytes());
        format!("{p_hash:016x}:{contexte_hash}")
    }

    /// Enregistre une occurrence et indique si cette clé avait déjà été vue.
    pub fn enregistrer(&mut self, p_hash: u64, alt: &str, contexte: &str, page: &str) -> bool {
        let cle = Self::cle(p_hash, alt, contexte);
        match self.entrees.get_mut(&cle) {
            Some(entree) => {
                if !entree.pages.contains(&page.to_owned()) {
                    entree.pages.push(page.to_owned());
                }
                let deja = entree.deja_analyse;
                entree.deja_analyse = true;
                deja
            }
            None => {
                self.entrees.insert(
                    cle,
                    EntreeDecision {
                        p_hash,
                        alt: alt.to_owned(),
                        contexte: contexte.to_owned(),
                        pages: vec![page.to_owned()],
                        decision: None,
                        deja_analyse: true,
                    },
                );
                false
            }
        }
    }

    /// Nombre de pages distinctes où ce pHash (ou une copie) apparaît.
    #[must_use]
    pub fn occurrences(&self, p_hash: u64) -> usize {
        let mut pages = Vec::new();
        for entree in self.entrees.values() {
            if phash::sont_des_copies(entree.p_hash, p_hash) {
                for page in &entree.pages {
                    if !pages.contains(page) {
                        pages.push(page.clone());
                    }
                }
            }
        }
        pages.len()
    }

    /// Vrai si une clé identique a déjà été analysée pendant cet audit.
    #[must_use]
    pub fn deja_analyse(&self, p_hash: u64, alt: &str, contexte: &str) -> bool {
        self.entrees
            .get(&Self::cle(p_hash, alt, contexte))
            .is_some_and(|entree| entree.deja_analyse)
    }

    /// Enregistre une décision de revue (jamais posée par la machine).
    pub fn enregistrer_decision(&mut self, cle: &str, decision: &str) {
        if let Some(entree) = self.entrees.get_mut(cle) {
            entree.decision = Some(decision.to_owned());
        }
    }

    #[must_use]
    pub fn decision(&self, cle: &str) -> Option<&str> {
        self.entrees.get(cle).and_then(|e| e.decision.as_deref())
    }

    /// Clés des copies d'une image, à proposer en lot au relecteur.
    #[must_use]
    pub fn copies(&self, cle: &str) -> Vec<String> {
        let Some(entree) = self.entrees.get(cle) else {
            return Vec::new();
        };
        self.entrees
            .iter()
            .filter(|(_, autre)| phash::sont_des_copies(autre.p_hash, entree.p_hash))
            .map(|(autre_cle, _)| autre_cle.clone())
            .collect()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entrees.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entrees.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_decision_is_only_a_proposal_for_copies() {
        let mut cache = DecisionCache::new();
        cache.enregistrer(1, "logo", "c", "p1");
        cache.enregistrer(2, "logo", "c", "p2");
        let cle = DecisionCache::cle(1, "logo", "c");
        cache.enregistrer_decision(&cle, "conforme");
        assert_eq!(cache.copies(&cle).len(), 2);
        assert_eq!(cache.decision(&DecisionCache::cle(2, "logo", "c")), None);
    }

    #[test]
    fn the_same_key_is_analysed_once() {
        let mut cache = DecisionCache::new();
        assert!(!cache.enregistrer(1, "alt", "c", "p1"));
        assert!(cache.enregistrer(1, "alt", "c", "p2"));
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.occurrences(1), 2);
    }
}
