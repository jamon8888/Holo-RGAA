//! pHash et rasterisation SVG — propriétés attendues des algorithmes,
//! indépendantes de l'implémentation retenue.

use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use rgaa_analysis_images::{phash, svg};

/// Dégradé continu : un pHash y est stable, contrairement à un damier dont
/// le seuil médian tombe presque à zéro.
fn degrade(taille: u32, horizontal: bool) -> DynamicImage {
    let mut image = RgbaImage::new(taille, taille);
    for y in 0..taille {
        for x in 0..taille {
            let valeur = if horizontal { x } else { y } as u8;
            image.put_pixel(x, y, Rgba([valeur, valeur, valeur, 255]));
        }
    }
    DynamicImage::ImageRgba8(image)
}

#[test]
fn phash_est_stable_pour_une_meme_image_et_ses_copies() {
    let original = degrade(64, true);
    let copie = original.clone();
    assert_eq!(
        phash::calculer(&original).expect("pHash"),
        phash::calculer(&copie).expect("pHash")
    );
}

#[test]
fn une_legerement_modification_reste_sous_le_seuil_de_repetition() {
    // Seuil de la spec : distance <= 6 compte comme la même image.
    let original = degrade(64, true);
    let mut modifiee = original.clone().to_rgba8();
    modifiee.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
    let distance = phash::distance(
        phash::calculer(&original).expect("pHash"),
        phash::calculer(&DynamicImage::ImageRgba8(modifiee)).expect("pHash"),
    );
    assert!(
        distance <= phash::DISTANCE_MAX_REPETITION,
        "distance = {distance}"
    );
}

#[test]
fn deux_images_sans_rapport_sont_loignes() {
    let horizontal = degrade(64, true);
    let vertical = degrade(64, false);
    let distance = phash::distance(
        phash::calculer(&horizontal).expect("pHash"),
        phash::calculer(&vertical).expect("pHash"),
    );
    assert!(
        distance > phash::DISTANCE_MAX_REPETITION,
        "distance = {distance}"
    );
}

#[test]
fn un_svg_est_rasterise_a_ses_dimensions_declarees() {
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="32">
        <rect width="64" height="32" fill="#ff0000"/>
    </svg> "##;
    let image = svg::rasteriser(svg, 768).expect("rasterisation");
    assert_eq!(image.width(), 64);
    assert_eq!(image.height(), 32);
    let pixel = image.get_pixel(32, 16).0;
    assert_eq!(pixel[0], 255);
    assert!(pixel[1] < 10 && pixel[2] < 10, "pixel = {pixel:?}");
}

#[test]
fn un_svg_tres_large_est_limite_a_la_vignette() {
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="512">
        <rect width="1024" height="512" fill="#0000ff"/>
    </svg> "##;
    let image = svg::rasteriser(svg, 768).expect("rasterisation");
    assert!(image.width() <= 768 && image.height() <= 768);
}
