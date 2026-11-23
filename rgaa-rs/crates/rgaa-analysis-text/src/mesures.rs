//! Forme des `mesures` produites par P-08 (spec §5).
//!
//! Chaque variante porte un `avis_modele` optionnel : il reste `null` tant
//! qu'aucun modèle n'est branché (point d'extension). Aucune structure ne
//! contient de champ `status` ou `verdict` (R-P1).

use crate::llm::ElementLlm;
use rgaa_analysis::{Indice, Portee, Subject};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Réponse d'un modèle sur un élément : avis, jamais un statut de test.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Pertinence {
    Oui,
    Non,
    Incertain,
}

/// Avis structuré du modèle, tel que décrit en spec §4.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AvisModele {
    pub pertinent: Pertinence,
    pub explication: String,
}

/// Mesure d'un élément, avec son éventuel lot LLM.
#[derive(Debug, Clone, PartialEq)]
pub struct Mesure {
    /// Identifiant de test RGAA (`6.1.1`, `8.6.1`, …).
    pub test: &'static str,
    pub sujet: Subject,
    pub portee: Portee,
    pub indice: Option<Indice>,
    pub donnees: Mesures,
    /// Élément à envoyer au modèle ; `None` pour un cas purement déterministe.
    pub element: Option<ElementLlm>,
}

impl Mesure {
    /// Associe un avis de modèle à la mesure et à son élément.
    pub fn set_avis(&mut self, avis: AvisModele) {
        match &mut self.donnees {
            Mesures::Lien(m) => m.avis_modele = Some(avis.clone()),
            Mesures::TitrePage(m) => m.avis_modele = Some(avis.clone()),
            Mesures::Cadre(m) => m.avis_modele = Some(avis.clone()),
            Mesures::Champ(m) => m.avis_modele = Some(avis.clone()),
            Mesures::Bouton(m) => m.avis_modele = Some(avis.clone()),
            Mesures::Formulaire(m) => m.avis_modele = Some(avis.clone()),
            Mesures::Tableau(m) => m.avis_modele = Some(avis.clone()),
            Mesures::PlanSite(m) => m.avis_modele = Some(avis.clone()),
            Mesures::VersionAlternative(m) => m.avis_modele = Some(avis.clone()),
            Mesures::Alternative(m) => m.avis_modele = Some(avis.clone()),
            Mesures::Legende(m) => m.avis_modele = Some(avis.clone()),
        }
        if let Some(element) = &mut self.element {
            element.avis_modele = Some(avis);
        }
    }
}

/// Ensemble fermé des formes de `mesures` du plugin, tagué par critère RGAA.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "critere")]
pub enum Mesures {
    #[serde(rename = "6.1")]
    Lien(LienMesures),
    #[serde(rename = "8.6")]
    TitrePage(TitrePageMesures),
    #[serde(rename = "2.2")]
    Cadre(CadreMesures),
    #[serde(rename = "11.2")]
    Champ(ChampMesures),
    #[serde(rename = "11.9")]
    Bouton(BoutonMesures),
    #[serde(rename = "11.10")]
    Formulaire(FormulaireMesures),
    #[serde(rename = "5.2")]
    Tableau(TableauMesures),
    #[serde(rename = "12.3")]
    PlanSite(PlanSiteMesures),
    #[serde(rename = "13.4")]
    VersionAlternative(VersionAlternativeMesures),
    #[serde(rename = "1.5")]
    Alternative(AlternativeMesures),
    #[serde(rename = "11.7")]
    Legende(LegendeMesures),
}

/// Mesures du critère 6.1 (exemple §5 de la spec).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LienMesures {
    pub intitule: String,
    pub nom_accessible: String,
    pub cible: String,
    pub generique: bool,
    pub meme_intitule_cibles_differentes: usize,
    pub similarite_cible: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avis_modele: Option<AvisModele>,
}

/// Mesures du critère 8.6.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TitrePageMesures {
    pub url: String,
    pub titre: String,
    pub h1: String,
    pub nombre_pages_identiques: usize,
    pub nom_de_fichier: bool,
    pub vide: bool,
    pub similarite_contenu: f64,
    pub jaro_h1: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avis_modele: Option<AvisModele>,
}

/// Mesures du critère 2.2.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CadreMesures {
    pub titre: String,
    pub titre_generique: bool,
    pub texte_document: String,
    pub similarite_titre: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avis_modele: Option<AvisModele>,
}

/// Mesures des critères 11.2 et 11.13.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ChampMesures {
    pub identifiant: String,
    pub etiquette: String,
    pub placeholder: String,
    pub autocomplete: Option<String>,
    pub autocomplete_attendu: Option<String>,
    pub ecart_finalite: bool,
    pub etiquette_vide: bool,
    pub etiquette_egale_placeholder: bool,
    pub symbole_seul: bool,
    pub format_attendu: Option<String>,
    pub exemple_format: Option<String>,
    pub requis_sans_indication: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avis_modele: Option<AvisModele>,
}

/// Mesures du critère 11.9.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct BoutonMesures {
    pub identifiant: String,
    pub nom: String,
    pub nom_vide: bool,
    pub icone_seule: bool,
    pub action_attendue: String,
    pub similarite_action: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avis_modele: Option<AvisModele>,
}

/// Mesures des critères 11.10, 11.11 et 11.12.
///
/// `soumission_effectuee` vaut toujours `false` : P-08 n'exécute aucune
/// soumission de formulaire (point d'extension F-SEC).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FormulaireMesures {
    pub identifiant: String,
    pub intitule: String,
    pub sensible: bool,
    pub mots_declencheurs: Vec<String>,
    pub messages_erreur: Vec<String>,
    pub messages_generiques: Vec<String>,
    pub recapitulatif_observe: Option<bool>,
    pub soumission_effectuee: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avis_modele: Option<AvisModele>,
}

/// Mesures des critères 5.2 et 5.5.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TableauMesures {
    pub identifiant: String,
    pub complexe: bool,
    pub resume_present: bool,
    pub caption_present: bool,
    pub similarite_caption: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avis_modele: Option<AvisModele>,
}

/// Mesures du critère 12.3.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PlanSiteMesures {
    pub present: bool,
    pub couverture: f64,
    pub urls_manquantes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avis_modele: Option<AvisModele>,
}

/// Mesures du critère 13.4 (mêmes calculs que P-04 §3.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct VersionAlternativeMesures {
    pub identifiant: String,
    pub titre: String,
    pub similarite: f64,
    pub couverture: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avis_modele: Option<AvisModele>,
}

/// Mesures des critères 1.5, 7.2 et 13.6.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AlternativeMesures {
    pub identifiant: String,
    pub critere: String,
    pub present: bool,
    pub intitule: String,
    pub contexte: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avis_modele: Option<AvisModele>,
}

/// Mesures du critère 11.7 (légende de groupe de champs).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LegendeMesures {
    pub identifiant: String,
    pub groupe: String,
    pub legende: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avis_modele: Option<AvisModele>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_mesures_sont_taguees_par_critere_et_n_ont_pas_de_statut() {
        let mesure = Mesures::Lien(LienMesures {
            intitule: "En savoir plus".to_owned(),
            nom_accessible: "En savoir plus".to_owned(),
            cible: "/offres".to_owned(),
            generique: true,
            meme_intitule_cibles_differentes: 7,
            similarite_cible: 0.31,
            avis_modele: None,
        });
        let json = serde_json::to_value(&mesure).expect("sérialisation");
        assert_eq!(json["critere"], "6.1");
        assert!(json.get("status").is_none());
        assert!(json.get("verdict").is_none());
    }
}
