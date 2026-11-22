//! Inférence CLIP réelle, exécutée seulement quand les modèles sont déjà
//! installés (jamais téléchargés pendant l'audit).

use image::{DynamicImage, Rgba, RgbaImage};
use rgaa_analysis::models;
use rgaa_analysis_images::clip::{ClipBackend, ClipEmbeddings, CLASSES};
use rgaa_analysis_images::models as images_models;

fn image_uniforme() -> DynamicImage {
    DynamicImage::ImageRgba8(RgbaImage::from_pixel(224, 224, Rgba([180, 120, 60, 255])))
}

#[test]
fn clip_zero_shot_et_ancre_de_hasard_si_les_modeles_sont_presents() {
    if !images_models::CLIP_MODELS.iter().all(models::is_present) {
        eprintln!("CLIP ignoré : lancez `rgaa models pull images` pour installer les modèles");
        return;
    }
    let moteur = ClipEmbeddings::from_models_dir(&models::models_dir()).expect("CLIP local");
    let autres = vec![
        "Rapport annuel".to_owned(),
        "Recette de cuisine".to_owned(),
        "Itinéraire de métro".to_owned(),
        "Bulletin météo".to_owned(),
        "Facture d'électricité".to_owned(),
    ];
    let mesure = moteur
        .analyze(&image_uniforme(), "une photo", &autres)
        .expect("inférence CLIP");
    eprintln!("CLIP : {mesure:?}");
    assert!(CLASSES.iter().any(|(fr, _)| *fr == mesure.classe));
    assert!((-1.0..=1.0).contains(&mesure.score));
    assert!((-1.0..=1.0).contains(&mesure.similarite_alt));
    assert!((-1.0..=1.0).contains(&mesure.ancre_hasard));
    assert!(mesure.ancre_hasard.is_finite());
}
