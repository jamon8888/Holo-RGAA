//! Forme des `mesures` du plugin (spec §6) et export de son JSON Schema.

use crate::flash::FlashResult;
use crate::limites::LimiteTemps;
use crate::mouvement::Mouvement;
use crate::{analysis_error, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Mesures d'une source animée (GIF ou vidéo).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Mesures {
    pub source: String,
    pub fps_analyse: f64,
    pub avertissement: String,
    pub note_luminance: String,
    pub flash_general: FlashResult,
    pub flash_rouge: FlashResult,
    pub mouvement: Vec<Mouvement>,
    pub limites_temps: Vec<LimiteTemps>,
    /// Second avis optionnel ; `null` quand IRIS n'est pas installé.
    pub oracle_iris: Option<FlashResult>,
}

/// Exporte le schéma de `mesures` pour `types/` (P-00 §8, critère 6).
///
/// # Errors
/// Renvoie `RgaaError::Analysis` si le schéma ne peut pas être sérialisé.
pub fn schema() -> Result<serde_json::Value> {
    serde_json::to_value(schemars::schema_for!(Mesures))
        .map_err(|erreur| analysis_error("motion", format!("schéma mesures invalide : {erreur}")))
}
