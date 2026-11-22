//! Banc OCR français (P-03 §2) : 30 images générées, libres de droits,
//! avec accents. Les moteurs ne tournent que si leurs modèles sont déjà
//! installés : aucun téléchargement pendant l'audit.

mod support;

use rgaa_analysis::models;
use rgaa_analysis_images::{
    models as images_models,
    ocr::{OarOcrBackend, OcrEngine, OcrsBackend, DEFAULT_OCR_ENGINE},
};
use support::{dossier, lire, mesurer};

#[test]
fn le_corpus_francais_contient_30_images_avec_accents() {
    let dossier = dossier("tests/fixtures/ocr-fr");
    let contenu = lire(&dossier);
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

#[test]
fn banc_ocr_francais_si_les_modeles_sont_presents() {
    if !images_models::OCR_MODELS.iter().all(models::is_present) {
        eprintln!("Banc OCR ignoré : lancez `rgaa models pull images`");
        return;
    }
    let repertoire = models::models_dir();
    let dossier = dossier("tests/fixtures/ocr-fr");
    let ocrs = OcrsBackend::from_models_dir(&repertoire).expect("moteur ocrs");
    let oar = OarOcrBackend::from_models_dir(&repertoire).expect("moteur oar-ocr");
    let cer_ocrs = mesurer(&ocrs, &dossier, "ocrs (français)");
    let cer_oar = mesurer(&oar, &dossier, "oar-ocr (français)");
    let meilleur = if cer_oar < cer_ocrs {
        OcrEngine::OarOcr
    } else {
        OcrEngine::Ocrs
    };
    eprintln!("Moteur par défaut : {DEFAULT_OCR_ENGINE:?} (meilleur mesuré : {meilleur:?})");
    assert_eq!(
        DEFAULT_OCR_ENGINE, meilleur,
        "le défaut doit suivre le meilleur CER FR"
    );
}
