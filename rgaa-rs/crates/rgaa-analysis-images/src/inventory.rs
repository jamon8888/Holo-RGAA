//! Inventaire DOM des images (P-03 §3).
//!
//! Le collecteur CDP n'existe pas encore : le plugin lit un `DomSnapshot`
//! sérialisé dont le format est défini ici. Il couvre `img`,
//! `input[type=image]`, `svg`, `area`, `object`, `embed`, `canvas`,
//! `role="img"` et les images CSS porteuses. L'ordre du document est conservé.

use rgaa_analysis::{analysis_error, Rect, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Un nœud d'image inventorié, avec le contexte utile aux règles.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ImageNode {
    /// `img`, `input-image`, `svg`, `area`, `object`, `embed`, `canvas`,
    /// `role-img` ou `css-background`.
    pub type_noeud: String,
    #[serde(default)]
    pub selecteur: String,
    #[serde(default)]
    pub xpath: String,
    #[serde(default)]
    pub bbox: Option<Rect>,
    /// URL de la ressource ou données SVG inline.
    #[serde(default)]
    pub src: Option<String>,
    #[serde(default)]
    pub alt: Option<String>,
    /// Nom accessible relevé dans l'arbre AX.
    #[serde(default)]
    pub nom_accessible: Option<String>,
    #[serde(default)]
    pub dans_lien: bool,
    #[serde(default)]
    pub dans_bouton: bool,
    #[serde(default)]
    pub dans_figure: bool,
    /// Texte voisin (~300 caractères) et légende `figcaption`.
    #[serde(default)]
    pub texte_adjacent: Option<String>,
    #[serde(default)]
    pub legende: Option<String>,
    /// Texte du lien englobant, pour la règle d'alt redondant.
    #[serde(default)]
    pub texte_lien: Option<String>,
    /// Classes et URL connues d'un CAPTCHA, ou proximité d'un champ « recopiez ».
    #[serde(default)]
    pub captcha: bool,
    #[serde(default)]
    pub classes: Option<String>,
}

/// Snapshot attendu : `{"page": "...", "images": [ ... ]}`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Snapshot {
    #[serde(default)]
    pub page: String,
    #[serde(default)]
    pub images: Vec<ImageNode>,
}

impl Snapshot {
    /// Parse un snapshot. Une entrée illisible est une erreur de domaine,
    /// jamais un `unwrap` (CLAUDE.md).
    ///
    /// # Errors
    ///
    /// Renvoie [`rgaa_analysis::RgaaError::Analysis`] si le JSON est invalide.
    pub fn parse(json: &str) -> Result<Self> {
        // Le chemin de convenance P-00 (`run_plugin`) n'a pas encore de
        // navigateur : un snapshot vide est un inventaire vide, pas une erreur.
        if json.trim().is_empty() {
            return Ok(Self::default());
        }
        serde_json::from_str(json).map_err(|error| {
            analysis_error("images", format!("inventaire DOM illisible : {error}"))
        })
    }
}

/// Un CAPTCHA est repéré par ses classes/URL connues ou par un champ
/// « recopiez » voisin (spec §3). Aucun OCR n'est appliqué à ces images.
#[must_use]
pub fn est_captcha(node: &ImageNode) -> bool {
    if node.captcha {
        return true;
    }
    let classes = node.classes.as_deref().unwrap_or_default().to_lowercase();
    let source = node.src.as_deref().unwrap_or_default().to_lowercase();
    let voisin = node
        .texte_adjacent
        .as_deref()
        .unwrap_or_default()
        .to_lowercase();
    ["recaptcha", "hcaptcha", "captcha"]
        .iter()
        .any(|marqueur| classes.contains(marqueur) || source.contains(marqueur))
        || voisin.contains("recopiez")
        || voisin.contains("reportez")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_declared_node_types_are_inventoried() {
        let json = r#"{"page":"https://exemple.test","images":[
            {"type_noeud":"img"},{"type_noeud":"input-image"},{"type_noeud":"svg"},
            {"type_noeud":"area"},{"type_noeud":"object"},{"type_noeud":"embed"},
            {"type_noeud":"canvas"},{"type_noeud":"role-img"},{"type_noeud":"css-background"}
        ]}"#;
        let snapshot = Snapshot::parse(json).expect("snapshot");
        assert_eq!(snapshot.images.len(), 9);
        assert_eq!(snapshot.page, "https://exemple.test");
        assert_eq!(snapshot.images[0].type_noeud, "img");
    }

    #[test]
    fn captcha_is_detected_from_classes_and_nearby_text() {
        let mut node = ImageNode {
            classes: Some("g-recaptcha".to_owned()),
            ..Default::default()
        };
        assert!(est_captcha(&node));
        node.classes = None;
        node.texte_adjacent = Some("Recopiez le texte affiché".to_owned());
        assert!(est_captcha(&node));
        node.texte_adjacent = Some("Chat roux sur un canapé".to_owned());
        assert!(!est_captcha(&node));
    }

    #[test]
    fn unreadable_snapshot_is_a_domain_error() {
        let error = Snapshot::parse("pas du json").expect_err("invalid JSON");
        assert!(error.to_string().contains("inventaire DOM illisible"));
    }
}
