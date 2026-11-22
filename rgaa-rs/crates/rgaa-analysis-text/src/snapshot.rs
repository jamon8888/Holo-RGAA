//! Entrée du plugin P-08 : ce que l'orchestrateur a déjà capturé, rien de plus.
//!
//! Le contrat P-00 fournit une `PageCapture` par page. Les règles de P-08
//! portent aussi sur plusieurs pages (8.6, 12.3) : l'adaptateur sérialise donc
//! ce `TextSnapshot` dans `PageCapture::dom_snapshot.json`. C'est le pendant de
//! `rgaa_analysis_layout::snapshot::Snapshot` pour P-07. Aucun champ ne
//! déclenche de requête réseau : cibles, titres cibles, textes et messages
//! d'erreur sont des observations déjà capturées.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Entrée complète, éventuellement sur plusieurs pages du même site.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct TextSnapshot {
    /// Page courante (celle que `PageCapture::url` décrit).
    pub page: PageText,
    /// Autres pages explorées, titres et textes déjà capturés (8.6).
    pub pages: Vec<PageText>,
    /// URLs découvertes par `rgaa-spider`, pour la couverture du plan (12.3).
    pub urls_spider: Vec<String>,
    /// Plan du site, s'il a été trouvé (12.3).
    pub plan_du_site: Option<PlanSite>,
}

/// Contenu textuel d'une page.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct PageText {
    pub url: String,
    pub titre: String,
    pub h1: String,
    pub contenu_principal: String,
    pub liens: Vec<Lien>,
    pub cadres: Vec<Cadre>,
    pub champs: Vec<Champ>,
    pub boutons: Vec<Bouton>,
    pub tableaux: Vec<Tableau>,
    pub formulaires: Vec<Formulaire>,
    pub alternatives: Vec<Alternative>,
    pub versions: Vec<VersionAlternative>,
}

/// Lien à évaluer au regard du critère 6.1.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Lien {
    pub id: String,
    pub intitule: String,
    /// `aria-label`, si présent.
    pub nom_accessible: String,
    /// Texte référencé par `aria-describedby`, si présent.
    pub description_accessible: String,
    /// Titre de la liste ou du bloc contenant le lien.
    pub titre_liste: String,
    pub titre_section: String,
    pub extrait: String,
    /// Cible normalisée (URL absolue ou chemin), sans requête ni fragment.
    pub cible: String,
    /// Titre de la page cible, si elle a été explorée.
    pub titre_cible: Option<String>,
}

/// Cadre à évaluer au regard du critère 2.2.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Cadre {
    pub id: String,
    pub titre: String,
    pub texte_document: String,
    pub titre_section: String,
    pub extrait: String,
}

/// Champ de formulaire : 11.2, 11.13 et signaux de 11.10/11.11.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Champ {
    pub id: String,
    pub etiquette: String,
    pub placeholder: String,
    pub autocomplete: Option<String>,
    pub type_champ: String,
    pub requis: bool,
    pub indication_requise_visible: bool,
    pub indication_requise_ax: bool,
    pub format_attendu: Option<String>,
    pub exemple_format: Option<String>,
    /// Groupe de champs (`fieldset`), pour la légende de 11.7.
    pub groupe: Option<String>,
    pub legende: Option<String>,
    pub titre_section: String,
    pub extrait: String,
    pub formulaire: Option<String>,
}

/// Bouton à évaluer au regard du critère 11.9.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Bouton {
    pub id: String,
    pub nom: String,
    /// Le nom visible se réduit à une icône (glyphe, emoji, image sans texte).
    pub icone_seule: bool,
    pub texte_proche: String,
    pub formulaire: Option<String>,
    pub titre_section: String,
    pub extrait: String,
}

/// Formulaire, pour 11.10 à 11.12.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Formulaire {
    pub id: String,
    pub intitule: String,
    /// Messages déjà capturés par une action simulée sans effet de bord (P-07).
    pub messages_erreur: Vec<String>,
    /// Présence d'une étape de récapitulatif, observée sans soumettre de paiement.
    pub recapitulatif_detecte: Option<bool>,
    pub titre_section: String,
    pub extrait: String,
}

/// Tableau, pour 5.2 (résumé) et 5.5 (titre).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Tableau {
    pub id: String,
    /// Complexe au sens du glossaire : en-têtes hors 1re ligne/colonne.
    pub complexe: bool,
    pub resume: String,
    pub caption: String,
    pub en_tetes: Vec<String>,
    pub contenu: String,
    pub titre_section: String,
    pub extrait: String,
}

/// Alternative textuelle des critères 1.5, 7.2 et 13.6.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Alternative {
    pub id: String,
    /// Critère RGAA servi : `1.5`, `7.2` ou `13.6`.
    pub critere: String,
    pub present: bool,
    pub intitule: String,
    pub contexte: String,
}

/// Document et sa version alternative équivalente (13.4, calculs P-04 §3.4).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct VersionAlternative {
    pub id: String,
    pub original: String,
    pub alternative: String,
    pub titre: String,
}

/// Page « plan du site » (12.3).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct PlanSite {
    pub url: String,
    pub liens: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_snapshot_minimal_est_valide_et_deterministe() {
        let snapshot = TextSnapshot::default();
        let json = serde_json::to_string(&snapshot).expect("sérialisation");
        let relu: TextSnapshot = serde_json::from_str(&json).expect("désérialisation");
        assert_eq!(relu, snapshot);
    }
}
