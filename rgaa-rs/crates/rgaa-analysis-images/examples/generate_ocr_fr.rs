//! Génère les bancs OCR (30 phrases FR accentuées, 5 phrases EN de contrôle).
//!
//! Usage : `cargo run -p rgaa-analysis-images --example generate_ocr_fr -- [--control] [--legacy] [dossier]`
//!
//! Les images sont générées par ce dépôt (CC0). La police est une famille
//! système courante, en noir sur blanc. Aucune image tiers n'est distribuée.

use image::DynamicImage;
use rgaa_analysis_images::svg::rasteriser;
use std::fs;
use std::path::{Path, PathBuf};

const TEXTES_FR: &[&str] = &[
    "SOLDES D'ÉTÉ -50% DANS TOUS LES RAYONS",
    "ÉTÉ 2026 : VACANCES À LA MER",
    "Le café est très chaud ce matin",
    "OÙ TROUVER LE NOUVEAU CATALOGUE ?",
    "Ça va très bien, merci beaucoup",
    "L'œuvre est exposée au musée",
    "RÉDUCTION DE 20% SUR LES LIVRES",
    "Il pleut à Paris depuis ce matin",
    "Bienvenue à l'hôtel de ville",
    "FERMÉ LE LUNDI POUR TRAVAUX",
    "Le thé île de Ceylan est délicieux",
    "Où sont les clés de la voiture ?",
    "Une élève très appliquée à l'école",
    "GRAND VIDE-DANSE CE SAMEDI À 14H",
    "Le château où nous avons dormi",
    "MENU DU JOUR : ŒUFS ET fromage",
    "Formation en ligne à distance",
    "ATTENTION : TRAVAUX PUBLICS À L'ENTRÉE",
    "Elle a écrit un très beau poème",
    "Café crème et croissant à emporter",
    "L'été où tout a changé",
    "ENTRÉE GRATUITE POUR LES ENFANTS",
    "Leçons de français à domicile",
    "Un été à la campagne, près d'ici",
    "VÉHICULES ÉLECTRIQUES EN STOCK",
    "Coût de la vie : enquête annuelle",
    "À bientôt pour une nouvelle saison",
    "Rendez-vous au café du port",
    "Une œuvre d'art à découvrir",
    "DÎNER DE GALA LE 12 DÉCEMBRE",
];

/// Cinq phrases anglaises, sans aucun accent : contrôle du banc lui-même.
const TEXTES_EN: &[&str] = &[
    "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG",
    "PLEASE SEND THE REPORT BEFORE FRIDAY AFTERNOON",
    "THE MEETING ROOM IS ON THE SECOND FLOOR",
    "WE SELL FRESH COFFEE AND HOT CROISSANTS DAILY",
    "THIS PRODUCT COMES WITH A TWO YEAR WARRANTY",
];

/// Réglage de rendu unique pour les deux bancs (modifié ensemble).
struct Rendu {
    famille: &'static str,
    taille: u32,
    largeur: u32,
    hauteur: u32,
    ligne_de_base: u32,
}

const RENDU: Rendu = Rendu {
    famille: "Liberation Serif, DejaVu Serif, serif",
    taille: 40,
    largeur: 1280,
    hauteur: 128,
    ligne_de_base: 88,
};

/// Réglage d'origine du banc (sans empattement, 32 px) : conservé pour
/// reproduire la mesure de référence avant correction du rendu.
const RENDU_BASELINE: Rendu = Rendu {
    famille: "Liberation Sans, DejaVu Sans, sans-serif",
    taille: 32,
    largeur: 768,
    hauteur: 96,
    ligne_de_base: 60,
};

fn echapper(xml: &str) -> String {
    xml.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let controle = arguments.iter().any(|arg| arg == "--control");
    let rendu = if arguments.iter().any(|arg| arg == "--legacy") {
        &RENDU_BASELINE
    } else {
        &RENDU
    };
    let (textes, dossier_defaut) = if controle {
        (TEXTES_EN, "tests/fixtures/ocr-control-en")
    } else {
        (TEXTES_FR, "tests/fixtures/ocr-fr")
    };
    let dossier = arguments
        .iter()
        .find(|arg| arg != &"--control" && arg != &"--legacy")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(dossier_defaut));
    fs::create_dir_all(&dossier)?;
    let mut verite: Vec<serde_json::Value> = Vec::with_capacity(textes.len());
    for (index, texte) in textes.iter().enumerate() {
        let svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="{largeur}" height="{hauteur}">
                <rect width="{largeur}" height="{hauteur}" fill="#ffffff"/>
                <text x="16" y="{base}" font-family="{famille}"
                      font-size="{taille}" fill="#000000">{texte}</text>
            </svg>"##,
            largeur = rendu.largeur,
            hauteur = rendu.hauteur,
            base = rendu.ligne_de_base,
            famille = rendu.famille,
            taille = rendu.taille,
            texte = echapper(texte),
        );
        let image: DynamicImage = rasteriser(svg.as_bytes(), rendu.largeur)?;
        let fichier = format!("{:02}.png", index + 1);
        image.save(Path::new(&dossier).join(&fichier))?;
        verite.push(serde_json::json!({ "fichier": fichier, "texte": texte }));
    }
    fs::write(
        dossier.join("ground_truth.json"),
        serde_json::to_string_pretty(&verite)?,
    )?;
    println!(
        "{} images générées dans {}",
        textes.len(),
        dossier.display()
    );
    Ok(())
}
