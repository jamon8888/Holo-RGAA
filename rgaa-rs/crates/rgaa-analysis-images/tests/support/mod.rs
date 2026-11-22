//! Utilitaires partagés par les bancs OCR (contrôle anglais et français).

use rgaa_analysis_images::ocr::OcrBackend;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct Entree {
    pub fichier: String,
    pub texte: String,
}

/// `RGAA_OCR_CORPUS` permet de mesurer un dossier temporaire (baseline).
pub fn dossier(defaut: &str) -> PathBuf {
    std::env::var_os("RGAA_OCR_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(defaut))
}

pub fn lire(dossier: &Path) -> Vec<Entree> {
    let chemin = dossier.join("ground_truth.json");
    let contenu = std::fs::read_to_string(&chemin)
        .unwrap_or_else(|error| panic!("{} : {error}", chemin.display()));
    serde_json::from_str(&contenu).expect("ground_truth.json valide")
}

/// Les retours à la ligne et les espaces répétés d'un OCR ne sont pas des
/// erreurs de caractères. Les accents, eux, sont conservés et comptent.
pub fn normaliser(texte: &str) -> String {
    texte.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Distance d'édition de Levenshtein (référence classique, hors moteur OCR).
pub fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut ligne: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut precedent = ligne[0];
        ligne[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let ancien = ligne[j + 1];
            ligne[j + 1] = (ligne[j + 1] + 1)
                .min(ligne[j] + 1)
                .min(precedent + usize::from(ca != cb));
            precedent = ancien;
        }
    }
    ligne[b.len()]
}

/// Mesure le CER d'un moteur sur un corpus et l'affiche.
pub fn mesurer(backend: &dyn OcrBackend, dossier: &Path, moteur: &str) -> f64 {
    let entrees = lire(dossier);
    let mut erreurs = 0usize;
    let mut caracteres = 0usize;
    for entree in &entrees {
        let image = image::open(dossier.join(&entree.fichier))
            .unwrap_or_else(|error| panic!("{} : {error}", entree.fichier));
        let mesure = backend
            .analyze(&image)
            .unwrap_or_else(|error| panic!("{moteur} sur {}: {error}", entree.fichier));
        assert!(!mesure.texte.is_empty(), "{} : OCR vide", entree.fichier);
        if std::env::var_os("RGAA_OCR_DEBUG").is_some() {
            eprintln!(
                "{} : {:?} / attendu {:?}",
                entree.fichier, mesure.texte, entree.texte
            );
        }
        erreurs += levenshtein(&normaliser(&mesure.texte), &normaliser(&entree.texte));
        caracteres += normaliser(&entree.texte).chars().count();
    }
    let cer = if caracteres == 0 {
        0.0
    } else {
        erreurs as f64 / caracteres as f64
    };
    eprintln!("CER {moteur} : {cer:.3} ({} images)", entrees.len());
    cer
}
