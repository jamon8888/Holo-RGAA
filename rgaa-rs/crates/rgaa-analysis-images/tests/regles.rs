//! Règles déterministes P-03 — valeurs attendues issues de la spec
//! `docs/specs/plugins-analyse/03-images-alternatives.md` §3 et §6.

use rgaa_analysis_images::regles;

#[test]
fn un_alt_egal_au_nom_de_fichier_est_detected() {
    // Exemples littéraux de la spec : `IMG_2034.jpg`, `banner-final-v2`.
    assert!(regles::est_alt_nom_fichier(
        "IMG_2034.jpg",
        "/img/IMG_2034.jpg"
    ));
    assert!(regles::est_alt_nom_fichier(
        "DSC_0042.JPG",
        "images/DSC_0042.JPG"
    ));
    assert!(regles::est_alt_nom_fichier(
        "banner-final-v2",
        "/img/banner-final-v2.png"
    ));
    assert!(!regles::est_alt_nom_fichier(
        "chat roux sur un canapé",
        "/img/chat.jpg"
    ));
}

#[test]
fn un_alt_generique_est_detected() {
    // La spec cite « image », « photo », « logo » seuls.
    for alt in ["image", "Image", "une image", "photo", "LOGO"] {
        assert!(regles::est_alt_generique(alt), "{alt}");
    }
    // Un logo nommé par son contexte n'est pas une alternative générique.
    assert!(!regles::est_alt_generique("Logo de l'association"));
    assert!(!regles::est_alt_generique("photo du chat"));
}

#[test]
fn la_limite_de_150_caracteres_est_stricte() {
    // Spec : « un alt de plus de 150 caractères » -> 150 acceptable, 151 suspect.
    let cent_cinquante = "é".repeat(150);
    let cent_cinquante_et_un = "é".repeat(151);
    assert!(!regles::est_alt_trop_long(&cent_cinquante));
    assert!(regles::est_alt_trop_long(&cent_cinquante_et_un));
}

#[test]
fn un_alt_identique_au_texte_du_lien_est_redondant() {
    assert!(regles::est_alt_redondant(
        "Voir le détail",
        Some("  voir le détail  ")
    ));
    assert!(!regles::est_alt_redondant(
        "Voir le détail",
        Some("En savoir plus")
    ));
    assert!(!regles::est_alt_redondant("Voir le détail", None));
}

#[test]
fn les_libelles_de_regles_sont_stables_et_ordonnes() {
    // Libellés attendus : ceux de la spec (§3, §6).
    let node = rgaa_analysis_images::inventory::ImageNode {
        type_noeud: "img".to_owned(),
        src: Some("DSC_0042.JPG".to_owned()),
        alt: Some("DSC_0042.JPG".to_owned()),
        texte_lien: Some("DSC_0042.JPG".to_owned()),
        ..Default::default()
    };
    assert_eq!(
        regles::appliquer(&node),
        vec!["alt-nom-fichier", "alt-redondant"]
    );
}
