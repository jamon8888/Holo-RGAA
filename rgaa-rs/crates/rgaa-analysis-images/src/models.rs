//! Modèles locaux P-03 : déclaration, vérification, lecture.
//!
//! Aucun téléchargement ici. Seule la commande `rgaa models pull images`
//! (P-00 §6) écrit dans `$RGAA_MODELS_DIR`. Les Sommes de contrôle et tailles
//! proviennent de l'API Hugging Face des dépôts épinglés (relevé du
//! 2026-11-22) ; les licences figurent dans les cartes des modèles.

use rgaa_analysis::models::{ensure_present_in, path_in};
use rgaa_analysis::{analysis_error, ModelSpec, Result};
use std::fs;
use std::path::Path;

/// Modèles `ocrs` (RTen) — licence CC-BY-SA-4.0 (carte Hugging Face).
pub const OCR_MODELS: &[ModelSpec] = &[
    ModelSpec {
        nom: "ocrs-text-detection-ssfbcj81.rten",
        url: "https://huggingface.co/robertknight/ocrs/resolve/main/text-detection-ssfbcj81.rten?download=true",
        sha256: "614aafabf27c94d386f7aa036c967c2e47e4b9938fa11531ca8f5698c1ca4c36",
        taille_octets: 2_523_564,
        licence: "CC-BY-SA-4.0",
    },
    ModelSpec {
        nom: "ocrs-text-recognition-s52qdbqt.rten",
        url: "https://huggingface.co/robertknight/ocrs/resolve/main/text-rec-checkpoint-s52qdbqt.rten?download=true",
        sha256: "606d9a0414c6b73c99df75b707c11c70d1c8b12e1d4f900922e185fc37bfca65",
        taille_octets: 9_716_444,
        licence: "CC-BY-SA-4.0",
    },
];

/// Modèles CLIP ViT-B/32 (fastembed 7.7) — licence MIT.
///
/// Les deux encodeurs et leurs fichiers de preprocessing/tokenisation sont
/// déclarés séparément pour que chaque octet téléchargé soit vérifié.
pub const CLIP_MODELS: &[ModelSpec] = &[
    ModelSpec {
        nom: "clip-vit-b32-vision.onnx",
        url: "https://huggingface.co/Qdrant/clip-ViT-B-32-vision/resolve/main/model.onnx?download=true",
        sha256: "c68d3d9a200ddd2a8c8a5510b576d4c94d1ae383bf8b36dd8c084f94e1fb4d63",
        taille_octets: 351_686_194,
        licence: "MIT",
    },
    ModelSpec {
        nom: "clip-vit-b32-vision-preprocessor_config.json",
        url: "https://huggingface.co/Qdrant/clip-ViT-B-32-vision/resolve/main/preprocessor_config.json?download=true",
        sha256: "ce945ef831c9972c135b5b198a03d8eeb70478cd69c0238f24caf1903a9965e6",
        taille_octets: 780,
        licence: "MIT",
    },
    ModelSpec {
        nom: "clip-vit-b32-text.onnx",
        url: "https://huggingface.co/Qdrant/clip-ViT-B-32-text/resolve/main/model.onnx?download=true",
        sha256: "4dbe762b11e36488304471e439cde89da053ad7acaddbf9e096745d142ec8d8b",
        taille_octets: 254_102_519,
        licence: "MIT",
    },
    ModelSpec {
        nom: "clip-vit-b32-text-tokenizer.json",
        url: "https://huggingface.co/Qdrant/clip-ViT-B-32-text/resolve/main/tokenizer.json?download=true",
        sha256: "b68d571997a1f81bf521fb73806740ddb91e4ed6666cb6e996c066bb289cf55b",
        taille_octets: 2_224_147,
        licence: "MIT",
    },
    ModelSpec {
        nom: "clip-vit-b32-text-config.json",
        url: "https://huggingface.co/Qdrant/clip-ViT-B-32-text/resolve/main/config.json?download=true",
        sha256: "4d5923d94bbc4e29864de837df14c138c12b93a0b738c64f3ca41f0c539e17b7",
        taille_octets: 536,
        licence: "MIT",
    },
    ModelSpec {
        nom: "clip-vit-b32-text-special_tokens_map.json",
        url: "https://huggingface.co/Qdrant/clip-ViT-B-32-text/resolve/main/special_tokens_map.json?download=true",
        sha256: "2cdb3b8331a60c92fc1e55a13e9fd61fd2293c5a51275fdcccd62b780052530e",
        taille_octets: 588,
        licence: "MIT",
    },
    ModelSpec {
        nom: "clip-vit-b32-text-tokenizer_config.json",
        url: "https://huggingface.co/Qdrant/clip-ViT-B-32-text/resolve/main/tokenizer_config.json?download=true",
        sha256: "6bdcee9ccce2a16ca2b4c0c5ed00b42c50ea225f4472a8c4c1e963a2902c2881",
        taille_octets: 705,
        licence: "MIT",
    },
];

/// Tous les modèles requis par le plugin, dans l'ordre d'affichage.
pub const MODELS: &[ModelSpec] = &{
    let mut tout: [ModelSpec; 9] = [OCR_MODELS[0]; 9];
    let mut i = 0;
    while i < OCR_MODELS.len() {
        tout[i] = OCR_MODELS[i];
        i += 1;
    }
    let mut j = 0;
    while j < CLIP_MODELS.len() {
        tout[i + j] = CLIP_MODELS[j];
        j += 1;
    }
    tout
};

/// Lit un modèle après vérification de son empreinte. Ne télécharge jamais.
///
/// # Errors
///
/// Renvoie [`rgaa_analysis::RgaaError::Analysis`] si le modèle est absent,
/// corrompu ou illisible.
pub fn lire(dir: &Path, spec: ModelSpec) -> Result<Vec<u8>> {
    ensure_present_in("images", &spec, dir)?;
    fs::read(path_in(dir, &spec)).map_err(|error| {
        analysis_error("images", format!("modèle {} illisible : {error}", spec.nom))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_model_declares_a_licence_and_a_digest() {
        assert_eq!(MODELS.len(), 9);
        for modele in MODELS {
            assert!(!modele.licence.is_empty(), "{}", modele.nom);
            assert_eq!(modele.sha256.len(), 64, "{}", modele.nom);
            assert!(modele.url.starts_with("https://"), "{}", modele.nom);
        }
    }
}
