//! OCR natif `ocrs` / `rten` (P-03 §2).
//!
//! Les deux modèles sont vérifiés dans `$RGAA_MODELS_DIR`, jamais téléchargés
//! ici. `OcrBackend` est le point d'extension : les futurs sidecars
//! `ocr-tesseract` et `oar-ocr` implémenteront le même trait.

use crate::models;
use image::DynamicImage;
use ocrs::{ImageSource, OcrEngine, OcrEngineParams, TextItem};
use rgaa_analysis::{analysis_error, Result};
use rten::Model;
use std::path::Path;

/// Résultat d'un moteur OCR sur une image.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct OcrMesures {
    pub texte: String,
    /// Surface des boîtes de mots / surface de l'image, dans [0, 1].
    pub couverture: f64,
    /// `None` : `ocrs` n'expose pas de confiance par mot.
    pub confiance: Option<f64>,
    pub moteur: String,
}

use serde::{Deserialize, Serialize};

/// Point d'extension OCR (sidecars futurs compris).
pub trait OcrBackend: Send + Sync {
    fn id(&self) -> &str;
    /// # Errors
    ///
    /// Renvoie une erreur de domaine si l'inférence échoue.
    fn analyze(&self, image: &DynamicImage) -> Result<OcrMesures>;
}

/// Moteur natif pur Rust, construit une fois à partir des modèlocaux.
pub struct OcrsBackend {
    moteur: OcrEngine,
}

impl OcrsBackend {
    /// Construit le moteur ; échoue avec « preuve manquante » si un modèle
    /// n'est pas installé ou si son empreinte ne correspond pas.
    ///
    /// # Errors
    ///
    /// Renvoie [`rgaa_analysis::RgaaError::Analysis`] si les modèles sont
    /// absents, corrompus ou invalides.
    pub fn from_models_dir(dir: &Path) -> Result<Self> {
        let detection = charger_modele(dir, models::OCR_MODELS[0])?;
        let recognition = charger_modele(dir, models::OCR_MODELS[1])?;
        let moteur = OcrEngine::new(OcrEngineParams {
            detection_model: Some(detection),
            recognition_model: Some(recognition),
            ..Default::default()
        })
        .map_err(|error| analysis_error("images", format!("moteur ocrs : {error}")))?;
        Ok(Self { moteur })
    }
}

fn charger_modele(dir: &Path, spec: rgaa_analysis::ModelSpec) -> Result<Model> {
    models::lire(dir, spec)?;
    Model::load_file(rgaa_analysis::models::path_in(dir, &spec))
        .map_err(|error| analysis_error("images", format!("modèle {} : {error}", spec.nom)))
}

impl OcrBackend for OcrsBackend {
    fn id(&self) -> &str {
        "ocrs"
    }

    fn analyze(&self, image: &DynamicImage) -> Result<OcrMesures> {
        let rgba = image.to_rgba8();
        let (largeur, hauteur) = rgba.dimensions();
        let source = ImageSource::from_bytes(rgba.as_raw(), (largeur, hauteur))
            .map_err(|error| analysis_error("images", format!("image OCR : {error}")))?;
        let entre = self
            .moteur
            .prepare_input(source)
            .map_err(|error| analysis_error("images", format!("préparation OCR : {error}")))?;
        let mots = self
            .moteur
            .detect_words(&entre)
            .map_err(|error| analysis_error("images", format!("détection OCR : {error}")))?;
        let lignes = self.moteur.find_text_lines(&entre, &mots);
        let reconnues = self
            .moteur
            .recognize_text(&entre, &lignes)
            .map_err(|error| analysis_error("images", format!("reconnaissance OCR : {error}")))?;

        let mut surface_texte = 0.0_f64;
        let mut texte = Vec::with_capacity(lignes.len());
        for ligne in reconnues.into_iter().flatten() {
            texte.push(ligne.to_string());
            for mot in ligne.words() {
                let boite = mot.bounding_rect();
                surface_texte += f64::from(boite.width()) * f64::from(boite.height());
            }
        }
        let surface_image = f64::from(largeur) * f64::from(hauteur);
        let couverture = if surface_image > 0.0 {
            (surface_texte / surface_image).clamp(0.0, 1.0)
        } else {
            0.0
        };
        Ok(OcrMesures {
            texte: texte.join("\n"),
            couverture,
            confiance: None,
            moteur: self.id().to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_declaration_matches_the_model_specs() {
        assert_eq!(models::OCR_MODELS.len(), 2);
        assert!(models::OCR_MODELS
            .iter()
            .all(|modele| modele.nom.ends_with(".rten")));
    }
}
