//! Configuration locale du plugin images (spec P-03 §2).
//!
//! Le moteur OCR par défaut est celui qui a le meilleur CER français mesuré
//! sur le banc du crate. `rgaa.toml` permet de le remplacer :
//!
//! ```toml
//! [analysis.images]
//! ocr_engine = "ocrs"   # ou "oar-ocr"
//! ```
//!
//! `$RGAA_CONFIG` peut désigner un autre fichier. Aucun fichier présent =
//! moteur par défaut. Cette lecture ne télécharge rien.

use crate::ocr::OcrEngine;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Deserialize)]
struct Configuration {
    #[serde(default)]
    analysis: ConfigurationAnalyse,
}

#[derive(Debug, Default, Deserialize)]
struct ConfigurationAnalyse {
    #[serde(default)]
    images: ConfigurationImages,
}

#[derive(Debug, Default, Deserialize)]
struct ConfigurationImages {
    #[serde(default)]
    ocr_engine: Option<OcrEngine>,
}

/// Lit `ocr_engine` dans `rgaa.toml` ; revient au moteur par défaut si le
/// fichier est absent, illisible ou ne contient pas de choix valide.
#[must_use]
pub fn moteur_ocr(fichier: Option<&Path>) -> OcrEngine {
    let depuis_env = std::env::var_os("RGAA_CONFIG").map(PathBuf::from);
    let chemin: PathBuf = fichier
        .map(PathBuf::from)
        .or(depuis_env)
        .unwrap_or_else(|| PathBuf::from("rgaa.toml"));
    let Ok(contenu) = fs::read_to_string(&chemin) else {
        return OcrEngine::default();
    };
    match toml::from_str::<Configuration>(&contenu) {
        Ok(configuration) => configuration.analysis.images.ocr_engine.unwrap_or_default(),
        Err(error) => {
            tracing::warn!(%error, chemin = %chemin.display(), "rgaa.toml illisible : moteur OCR par défaut");
            OcrEngine::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_config_file_selects_the_engine() {
        let dossier = tempfile::tempdir().expect("temp");
        let chemin = dossier.path().join("rgaa.toml");
        fs::write(&chemin, "[analysis.images]\nocr_engine = \"ocrs\"\n").expect("write");
        assert_eq!(moteur_ocr(Some(&chemin)), OcrEngine::Ocrs);
        fs::write(&chemin, "[analysis.images]\nocr_engine = \"oar-ocr\"\n").expect("write");
        assert_eq!(moteur_ocr(Some(&chemin)), OcrEngine::OarOcr);
    }

    #[test]
    fn a_missing_file_uses_the_default_engine() {
        assert_eq!(
            moteur_ocr(Some(Path::new("/inexistant/rgaa.toml"))),
            OcrEngine::default()
        );
    }
}
