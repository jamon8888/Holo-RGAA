//! Forme du champ `mesures` des preuves P-03 (export JSON Schema).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Mesures d'une image, telles que documentées dans la spec P-03 §4.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Mesures {
    pub type_noeud: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nom_accessible: Option<String>,
    #[serde(default)]
    pub dans_lien: bool,
    #[serde(default)]
    pub occurrences_site: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regles: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ocr: Option<MesureOcr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clip: Option<MesureClip>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captcha: Option<MesureCaptcha>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preuve_manquante: Option<String>,
    /// Clé du cache réutilisée : cette image a déjà été analysée cet audit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reutilisation_cache: Option<String>,
}

/// Résultat OCR, ou raison de l'absence de preuve.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MesureOcr {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texte: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub couverture: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confiance: Option<f64>,
    pub moteur: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preuve_manquante: Option<String>,
}

/// Résultat CLIP, ou raison de l'absence de preuve.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MesureClip {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classe: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similarite_alt: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ancre_hasard: Option<f64>,
    pub moteur: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preuve_manquante: Option<String>,
}

/// Un CAPTCHA n'est jamais OCRisé (critère 1.4).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MesureCaptcha {
    pub ocr: bool,
    pub raison: String,
}
