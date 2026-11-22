//! CLIP zéro-shot et similarité `alt` ↔ image avec ancre de hasard (P-03 §3).
//!
//! `fastembed` 7.7 fournit les encodeurs image et texte. Les fichiers ONNX
//! sont lus depuis `$RGAA_MODELS_DIR` : `try_new_from_user_defined` évite
//! tout appel réseau pendant l'audit.

use crate::models;
use fastembed::similarity::cosine_similarity;
use fastembed::{
    ImageEmbedding, ImageInitOptionsUserDefined, InitOptionsUserDefined, Pooling, TextEmbedding,
    TokenizerFiles, UserDefinedEmbeddingModel, UserDefinedImageEmbeddingModel,
};
use image::DynamicImage;
use rgaa_analysis::{analysis_error, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

/// Classes zéro-shot de la spec, avec leur variante anglaise.
pub const CLASSES: &[(&str, &str)] = &[
    ("une photo", "a photograph"),
    ("une icône", "an icon"),
    ("un logo", "a logo"),
    ("un graphique", "a chart"),
    ("une capture d'écran de texte", "a screenshot of text"),
    (
        "un motif décoratif, un séparateur",
        "a decorative pattern, a separator",
    ),
];

/// Longueur de contexte CLIP ViT-B/32 (CLIP paper, 2021).
const CONTEXTE_CLIP: usize = 77;

/// Résultat CLIP attaché à une image.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ClipMesures {
    pub classe: String,
    pub score: f64,
    pub similarite_alt: f64,
    pub ancre_hasard: f64,
}

/// Point d'extension (un autre encodeur local pourrait implémenter ce trait).
pub trait ClipBackend: Send + Sync {
    fn id(&self) -> &str;
    /// # Errors
    ///
    /// Renvoie une erreur de domaine si l'inférence échoue.
    fn analyze(
        &self,
        image: &DynamicImage,
        alt: &str,
        alts_melanges: &[String],
    ) -> Result<ClipMesures>;
}

/// Ancre de hasard : moyenne des similarités des alt mélangées de la page.
/// Sans alt mélangée, l'ancre est nulle (aucune preuve de hasard disponible).
#[must_use]
pub fn ancre_hasard(similarites: &[f64]) -> f64 {
    if similarites.is_empty() {
        return 0.0;
    }
    similarites.iter().sum::<f64>() / similarites.len() as f64
}

/// Moteur CLIP pu Rust, construit à partir des modèles vérifiés.
pub struct ClipEmbeddings {
    vision: Mutex<ImageEmbedding>,
    texte: Mutex<TextEmbedding>,
}

impl ClipEmbeddings {
    /// # Errors
    ///
    /// Renvoie [`rgaa_analysis::RgaaError::Analysis`] si un modèle est absent,
    /// corrompu, ou si fastembed ne peut pas créer une session.
    pub fn from_models_dir(dir: &Path) -> Result<Self> {
        let vision_onnx = models::lire(dir, models::CLIP_MODELS[0])?;
        let preprocesseur = models::lire(dir, models::CLIP_MODELS[1])?;
        let vision = ImageEmbedding::try_new_from_user_defined(
            UserDefinedImageEmbeddingModel::new(vision_onnx, preprocesseur),
            ImageInitOptionsUserDefined::new().with_intra_threads(2),
        )
        .map_err(|error| analysis_error("images", format!("CLIP vision : {error}")))?;

        let texte_onnx = models::lire(dir, models::CLIP_MODELS[2])?;
        let fichiers = TokenizerFiles {
            tokenizer_file: models::lire(dir, models::CLIP_MODELS[3])?,
            config_file: models::lire(dir, models::CLIP_MODELS[4])?,
            special_tokens_map_file: models::lire(dir, models::CLIP_MODELS[5])?,
            tokenizer_config_file: models::lire(dir, models::CLIP_MODELS[6])?,
        };
        let texte = TextEmbedding::try_new_from_user_defined(
            UserDefinedEmbeddingModel::new(texte_onnx, fichiers).with_pooling(Pooling::Mean),
            InitOptionsUserDefined::new()
                .with_max_length(CONTEXTE_CLIP)
                .with_intra_threads(2),
        )
        .map_err(|error| analysis_error("images", format!("CLIP texte : {error}")))?;

        Ok(Self {
            vision: Mutex::new(vision),
            texte: Mutex::new(texte),
        })
    }
}

impl ClipBackend for ClipEmbeddings {
    fn id(&self) -> &str {
        "clip-vit-b32"
    }

    fn analyze(
        &self,
        image: &DynamicImage,
        alt: &str,
        alts_melanges: &[String],
    ) -> Result<ClipMesures> {
        let mut vision = self
            .vision
            .lock()
            .map_err(|_| analysis_error("images", "verrou CLIP vision empoisonné"))?;
        let vecteur_image = vision
            .embed_images(vec![image.clone()])
            .map_err(|error| analysis_error("images", format!("CLIP image : {error}")))?
            .into_iter()
            .next()
            .ok_or_else(|| analysis_error("images", "CLIP image : aucun plongement"))?;
        drop(vision);

        let prompts: Vec<String> = CLASSES
            .iter()
            .flat_map(|(fr, en)| [(*fr).to_owned(), (*en).to_owned()])
            .collect();
        let mut texte = self
            .texte
            .lock()
            .map_err(|_| analysis_error("images", "verrou CLIP texte empoisonné"))?;
        let vecteurs_prompts = texte
            .embed(&prompts, Some(8))
            .map_err(|error| analysis_error("images", format!("CLIP prompts : {error}")))?;
        let (mut meilleur, mut score_max) = (0_usize, f64::NEG_INFINITY);
        for (index, vecteur) in vecteurs_prompts.iter().enumerate() {
            let score = f64::from(cosine_similarity(&vecteur_image, vecteur));
            if score > score_max {
                meilleur = index / 2;
                score_max = score;
            }
        }

        let mut textes = Vec::with_capacity(alts_melanges.len() + 1);
        textes.push(alt.to_owned());
        textes.extend_from_slice(alts_melanges);
        let vecteurs = texte
            .embed(&textes, Some(8))
            .map_err(|error| analysis_error("images", format!("CLIP alt : {error}")))?;
        let similarite_alt = vecteurs
            .first()
            .map(|vecteur| f64::from(cosine_similarity(&vecteur_image, vecteur)))
            .unwrap_or(0.0);
        let scores_hasard: Vec<f64> = vecteurs
            .iter()
            .skip(1)
            .map(|vecteur| f64::from(cosine_similarity(&vecteur_image, vecteur)))
            .collect();
        drop(texte);

        Ok(ClipMesures {
            classe: CLASSES[meilleur].0.to_owned(),
            score: score_max,
            similarite_alt,
            ancre_hasard: ancre_hasard(&scores_hasard),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chance_anchor_is_the_mean_of_shuffled_alts() {
        // Valeurs arithmétiques indépendantes du modèle.
        assert!((ancre_hasard(&[0.1, 0.3, 0.2, 0.4, 0.5]) - 0.3).abs() < 1e-12);
        assert_eq!(ancre_hasard(&[]), 0.0);
    }

    #[test]
    fn every_class_has_a_french_and_an_english_prompt() {
        assert_eq!(CLASSES.len(), 6);
        assert!(CLASSES
            .iter()
            .all(|(fr, en)| !fr.is_empty() && !en.is_empty()));
    }
}
