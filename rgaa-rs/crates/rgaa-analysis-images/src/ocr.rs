//! Moteurs OCR natifs : `ocrs` (anglais) et `oar-ocr` (PaddleOCR latin).
//!
//! Les modèles sont vérifiés dans `$RGAA_MODELS_DIR`, jamais téléchargés ici.
//! `OcrBackend` est le point d'extension : le futur sidecar `ocr-tesseract`
//! implémentera le même trait. Les deux moteurs partagent la même version
//! d'`ort` que `fastembed` (vérifié avec `cargo tree -d`).

use crate::models;
use image::DynamicImage;
use oar_ocr::prelude::OAROCRBuilder;
use ocrs::{ImageSource, OcrEngine as OcrsEngine, OcrEngineParams, TextItem};
use rgaa_analysis::{analysis_error, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Résultat d'un moteur OCR sur une image.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct OcrMesures {
    pub texte: String,
    /// Surface des boîtes de texte / surface de l'image, dans [0, 1].
    pub couverture: f64,
    /// Confiance moyenne quand le moteur la fournit.
    pub confiance: Option<f64>,
    pub moteur: String,
}

/// Moteur OCR sélectionné pour le plugin (banc FR du crate).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum OcrEngine {
    /// `ocrs` + modèles `ocrs-cli` : CER FR = 0,086, CER EN = 0,009.
    #[serde(rename = "ocrs")]
    Ocrs,
    /// PP-OCRv5 latin : CER FR = 0,001, CER EN = 0,000 (défaut).
    #[serde(rename = "oar-ocr")]
    OarOcr,
}

/// CER mesurés sur les 30 images du banc FR (voir `tests/ocr_fr_bench.rs`).
pub const CER_OCRS_FR: f64 = 0.086;
pub const CER_OAR_FR: f64 = 0.001;

/// Défaut : le moteur au meilleur CER français sur les 30 images du banc.
pub const DEFAULT_OCR_ENGINE: OcrEngine = if CER_OAR_FR < CER_OCRS_FR {
    OcrEngine::OarOcr
} else {
    OcrEngine::Ocrs
};

impl Default for OcrEngine {
    fn default() -> Self {
        DEFAULT_OCR_ENGINE
    }
}

/// Point d'extension OCR (sidecars futurs compris).
pub trait OcrBackend: Send + Sync {
    fn id(&self) -> &str;
    /// # Errors
    ///
    /// Renvoie une erreur de domaine si l'inférence échoue.
    fn analyze(&self, image: &DynamicImage) -> Result<OcrMesures>;
}

/// Moteur natif pur Rust, construit une fois à partir des modèles locaux.
pub struct OcrsBackend {
    moteur: OcrsEngine,
}

impl OcrsBackend {
    /// # Errors
    ///
    /// Renvoie [`rgaa_analysis::RgaaError::Analysis`] si les modèles sont
    /// absents, corrompus ou invalides.
    pub fn from_models_dir(dir: &Path) -> Result<Self> {
        let detection = charger_modele(dir, models::OCR_MODELS[0])?;
        let recognition = charger_modele(dir, models::OCR_MODELS[1])?;
        let moteur = OcrsEngine::new(OcrEngineParams {
            detection_model: Some(detection),
            recognition_model: Some(recognition),
            ..Default::default()
        })
        .map_err(|error| analysis_error("images", format!("moteur ocrs : {error}")))?;
        Ok(Self { moteur })
    }
}

fn charger_modele(dir: &Path, spec: rgaa_analysis::ModelSpec) -> Result<rten::Model> {
    models::lire(dir, spec)?;
    rten::Model::load_file(rgaa_analysis::models::path_in(dir, &spec))
        .map_err(|error| analysis_error("images", format!("modèle {} : {error}", spec.nom)))
}

impl OcrBackend for OcrsBackend {
    fn id(&self) -> &str {
        "ocrs"
    }

    fn analyze(&self, image: &DynamicImage) -> Result<OcrMesures> {
        // `ocrs` attend des pixels RVB (HWC), comme son propre CLI.
        let rgb = image.to_rgb8();
        let (largeur, hauteur) = rgb.dimensions();
        let source = ImageSource::from_bytes(rgb.as_raw(), (largeur, hauteur))
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
        Ok(OcrMesures {
            texte: texte.join("\n"),
            couverture: couverture(surface_texte, largeur, hauteur),
            confiance: None,
            moteur: self.id().to_owned(),
        })
    }
}

/// Moteur PP-OCRv5 latin (`oar-ocr`), sans téléchargement à l'exécution.
pub struct OarOcrBackend {
    moteur: oar_ocr::prelude::OAROCR,
}

impl OarOcrBackend {
    /// Construit le pipeline avec les trois fichiers vérifiés du cache.
    ///
    /// # Errors
    ///
    /// Renvoie [`rgaa_analysis::RgaaError::Analysis`] si un modèle est absent,
    /// corrompu ou si le pipeline ne peut pas être initialisé.
    pub fn from_models_dir(dir: &Path) -> Result<Self> {
        for spec in models::OAR_MODELS {
            models::lire(dir, *spec)?;
        }
        let chemin = |index: usize| -> Result<String> {
            rgaa_analysis::models::path_in(dir, &models::OAR_MODELS[index])
                .to_str()
                .map(str::to_owned)
                .ok_or_else(|| analysis_error("images", "chemin de modèle non UTF-8"))
        };
        let (detection, reconnaissance, dictionnaire) = (chemin(0)?, chemin(1)?, chemin(2)?);
        let moteur = OAROCRBuilder::new(
            detection.as_str(),
            reconnaissance.as_str(),
            dictionnaire.as_str(),
        )
        .build()
        .map_err(|error| analysis_error("images", format!("moteur oar-ocr : {error}")))?;
        Ok(Self { moteur })
    }
}

impl OcrBackend for OarOcrBackend {
    fn id(&self) -> &str {
        "oar-ocr-v5-latin"
    }

    fn analyze(&self, image: &DynamicImage) -> Result<OcrMesures> {
        let rgb = image.to_rgb8();
        let (largeur, hauteur) = rgb.dimensions();
        let resultats = self
            .moteur
            .predict(vec![rgb])
            .map_err(|error| analysis_error("images", format!("oar-ocr : {error}")))?;
        let resultat = resultats
            .first()
            .ok_or_else(|| analysis_error("images", "oar-ocr : aucun résultat"))?;
        let mut texte = Vec::new();
        let mut surface_texte = 0.0_f64;
        let mut confiance = 0.0_f64;
        let mut confiances = 0usize;
        for region in &resultat.text_regions {
            if let Some(mot) = region.text.as_deref() {
                texte.push(mot.to_owned());
                surface_texte += f64::from(region.bounding_box.area());
            }
            if let Some(valeur) = region.confidence {
                confiance += f64::from(valeur);
                confiances += 1;
            }
        }
        Ok(OcrMesures {
            texte: texte.join(" "),
            couverture: couverture(surface_texte, largeur, hauteur),
            confiance: if confiances == 0 {
                None
            } else {
                Some(confiance / confiances as f64)
            },
            moteur: self.id().to_owned(),
        })
    }
}

fn couverture(surface_texte: f64, largeur: u32, hauteur: u32) -> f64 {
    let surface = f64::from(largeur) * f64::from(hauteur);
    if surface == 0.0 {
        0.0
    } else {
        (surface_texte / surface).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_engines_declare_onnx_model_files() {
        assert_eq!(models::OCR_MODELS.len(), 2);
        assert_eq!(models::OAR_MODELS.len(), 3);
        assert!(models::OCR_MODELS
            .iter()
            .chain(models::OAR_MODELS.iter())
            .all(|modele| modele.nom.ends_with(".onnx") || modele.nom.ends_with(".txt")));
    }
}
