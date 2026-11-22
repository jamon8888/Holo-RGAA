//! Banc OCR français (P-03 §2) : 30 images générées, libres de droits,
//! avec accents. Le moteur n'est exécuté que si ses modèles sont installés :
//! un audit sans modèle ne télécharge rien et ne échoue pas ce test.

use rgaa_analysis::models;
use rgaa_analysis_images::{models as images_models, ocr::OcrBackend, ocr::OcrsBackend};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct Entree {
    fichier: String,
    texte: String,
}

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ocr-fr")
}

#[test]
fn le_corpus_francais_contient_30_images_avec_accents() {
    let dossier = corpus();
    let contenu: Vec<Entree> = serde_json::from_str(
        &std::fs::read_to_string(dossier.join("ground_truth.json")).expect("vérité terrain"),
    )
    .expect("ground_truth.json valide");
    assert_eq!(contenu.len(), 30, "la spec demande 30 images");
    let accents = [
        'é', 'è', 'à', 'ç', 'œ', 'ù', 'û', 'ô', 'î', 'É', 'È', 'À', 'Ç', 'Œ', 'Ù',
    ];
    for entree in &contenu {
        assert!(
            entree.texte.chars().any(|c| accents.contains(&c)),
            "{} doit porter un accent : {}",
            entree.fichier,
            entree.texte
        );
        let image = image::open(dossier.join(&entree.fichier)).expect("image du corpus");
        assert!(image.width() >= 300 && image.height() >= 32);
    }
}

/// Distance d'édition de Levenshtein (référence classique, indépendante d'ocrs).
fn levenshtein(a: &str, b: &str) -> usize {
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

#[test]
fn banc_ocr_si_les_modeles_sont_presents() {
    let dossier = corpus();
    let contenu: Vec<Entree> = serde_json::from_str(
        &std::fs::read_to_string(dossier.join("ground_truth.json")).expect("vérité terrain"),
    )
    .expect("ground_truth.json valide");
    if !images_models::OCR_MODELS.iter().all(models::is_present) {
        eprintln!("Banc OCR ignoré : lancez `rgaa models pull images` pour installer les modèles");
        return;
    }
    let repertoire = models::models_dir();
    let moteur = OcrsBackend::from_models_dir(&repertoire).expect("moteur ocrs");
    let mut erreurs = 0usize;
    let mut caracteres = 0usize;
    for entree in &contenu {
        let image = image::open(dossier.join(&entree.fichier)).expect("image");
        let mesure = moteur.analyze(&image).expect("OCR");
        assert!(!mesure.texte.is_empty(), "{} : OCR vide", entree.fichier);
        erreurs += levenshtein(&mesure.texte, &entree.texte);
        caracteres += entree.texte.chars().count();
    }
    let cer = if caracteres == 0 {
        0.0
    } else {
        erreurs as f64 / caracteres as f64
    };
    // Le CER est une mesure, pas un seuil de validation : ocrs est entraîné
    // surtout sur l'anglais et son alphabet par défaut n'a pas d'accents.
    // Le banc sert à comparer les moteurs dans le ticket suivant.
    eprintln!(
        "Banc OCR FR : {} images, CER ocrs = {cer:.3}",
        contenu.len()
    );
    assert!(cer.is_finite());
}
