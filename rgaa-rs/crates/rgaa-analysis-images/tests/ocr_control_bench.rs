//! Contrôle du banc OCR : cinq phrases anglaises sans accent, générées par
//! le même pipeline que le corpus français. Un CER élevé ici incrimine le
//! rendu des fixtures, pas la langue du texte.

mod support;

use rgaa_analysis::models;
use rgaa_analysis_images::{models as images_models, ocr::OarOcrBackend, ocr::OcrsBackend};
use support::{dossier, lire, mesurer};

#[test]
fn banc_de_controle_anglais_est_lisible() {
    let dossier = dossier("tests/fixtures/ocr-control-en");
    let entrees = lire(&dossier);
    assert_eq!(entrees.len(), 5);
    for entree in &entrees {
        assert!(
            entree.texte.is_ascii(),
            "le contrôle ne doit pas contenir d'accent : {}",
            entree.texte
        );
        let image = image::open(dossier.join(&entree.fichier)).expect("image de contrôle");
        assert!(image.width() >= 300 && image.height() >= 32);
    }
}

#[test]
fn cer_anglais_doit_rester_sous_5_pour_cent() {
    if !images_models::OCR_MODELS.iter().all(models::is_present) {
        eprintln!("Contrôle OCR ignoré : modèles absents");
        return;
    }
    let repertoire = models::models_dir();
    let dossier = dossier("tests/fixtures/ocr-control-en");
    let ocrs = OcrsBackend::from_models_dir(&repertoire).expect("moteur ocrs");
    let oar = OarOcrBackend::from_models_dir(&repertoire).expect("moteur oar-ocr");
    let cer_ocrs = mesurer(&ocrs, &dossier, "ocrs (contrôle anglais)");
    let cer_oar = mesurer(&oar, &dossier, "oar-ocr (contrôle anglais)");
    assert!(
        cer_ocrs < 0.05,
        "CER de contrôle ocrs = {cer_ocrs:.3} : le rendu est en cause"
    );
    assert!(
        cer_oar < 0.05,
        "CER de contrôle oar-ocr = {cer_oar:.3} : le rendu est en cause"
    );
}
