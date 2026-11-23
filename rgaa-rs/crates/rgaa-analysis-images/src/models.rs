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
        nom: "ocrs-text-detection.onnx",
        url: "https://ocrs-models.s3.amazonaws.com/text-detection.onnx",
        sha256: "a917b23dbd9524b465df7e922641b3ff2981623df4ded5a0234004ef2fee7cfe",
        taille_octets: 2_499_479,
        licence: "CC-BY-SA-4.0",
    },
    ModelSpec {
        nom: "ocrs-text-recognition.onnx",
        url: "https://ocrs-models.s3.amazonaws.com/text-recognition.onnx",
        sha256: "86c145c2edb96c8caed5b1ebb8f44d706408922211451c309c625157dd6061c5",
        taille_octets: 9_713_177,
        licence: "CC-BY-SA-4.0",
    },
];

/// Modèles PP-OCRv5 latin (`oar-ocr`) — Apache-2.0 (PaddleOCR).
///
/// Fichiers officiels de la release v0.3.0 de `oar-ocr`, utilisés tels quels
/// (aucun téléchargement automatique : la feature `auto-download` est désactivée).
pub const OAR_MODELS: &[ModelSpec] = &[
    ModelSpec {
        nom: "pp-ocrv5_mobile_det.onnx",
        url: "https://github.com/GreatV/oar-ocr/releases/download/v0.3.0/pp-ocrv5_mobile_det.onnx",
        sha256: "1eb7b4f7ab657ebd1c66d5f79bca7497f29768a2e3c15e52daecbba1a8e4a039",
        taille_octets: 4_826_518,
        licence: "Apache-2.0",
    },
    ModelSpec {
        nom: "latin_pp-ocrv5_mobile_rec.onnx",
        url: "https://github.com/GreatV/oar-ocr/releases/download/v0.3.0/latin_pp-ocrv5_mobile_rec.onnx",
        sha256: "e3a6bfeea1c8a01d6fccfd480a0bd363fd907f8c65931e228bb2736f5c3e142f",
        taille_octets: 8_069_614,
        licence: "Apache-2.0",
    },
    ModelSpec {
        nom: "ppocrv5_latin_dict.txt",
        url: "https://github.com/GreatV/oar-ocr/releases/download/v0.3.0/ppocrv5_latin_dict.txt",
        sha256: "ccbcc45730b3fbbd9050c5bc74db6a99067141ef1035e3d14889a84a6b9b1aff",
        taille_octets: 2_616,
        licence: "Apache-2.0",
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
    let mut tout: [ModelSpec; 12] = [OCR_MODELS[0]; 12];
    let mut i = 0;
    while i < OCR_MODELS.len() {
        tout[i] = OCR_MODELS[i];
        i += 1;
    }
    let mut j = 0;
    while j < OAR_MODELS.len() {
        tout[i + j] = OAR_MODELS[j];
        j += 1;
    }
    i += OAR_MODELS.len();
    let mut k = 0;
    while k < CLIP_MODELS.len() {
        tout[i + k] = CLIP_MODELS[k];
        k += 1;
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
        assert_eq!(MODELS.len(), 12);
        for modele in MODELS {
            assert!(!modele.licence.is_empty(), "{}", modele.nom);
            assert_eq!(modele.sha256.len(), 64, "{}", modele.nom);
            assert!(modele.url.starts_with("https://"), "{}", modele.nom);
        }
    }
}
