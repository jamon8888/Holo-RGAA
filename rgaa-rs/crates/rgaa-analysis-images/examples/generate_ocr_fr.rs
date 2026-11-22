//! Génère le banc OCR français (30 PNG, texte accentué) avec resvg.
//!
//! Usage : `cargo run -p rgaa-analysis-images --example generate_ocr_fr -- <dossier>`
//!
//! Les images sont générées par ce dépôt (CC0) ; la police est Liberation
//! Sans (SIL OFL 1.1), présente sur la machine de génération. Aucune image
//! tiers n'est redistribuée.

use image::DynamicImage;
use rgaa_analysis_images::svg::rasteriser;
use std::fs;
use std::path::{Path, PathBuf};

const TEXTES: &[&str] = &[
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

fn echapper(xml: &str) -> String {
    xml.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let dossier = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ocr-fr"));
    fs::create_dir_all(&dossier)?;
    let mut verite: Vec<serde_json::Value> = Vec::with_capacity(TEXTES.len());
    for (index, texte) in TEXTES.iter().enumerate() {
        let svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="768" height="96">
                <rect width="768" height="96" fill="#ffffff"/>
                <text x="14" y="60" font-family="Liberation Sans, DejaVu Sans, sans-serif"
                      font-size="32" fill="#000000">{}</text>
            </svg>"##,
            echapper(texte)
        );
        let image: DynamicImage = rasteriser(svg.as_bytes(), 768)?;
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
        TEXTES.len(),
        dossier.display()
    );
    Ok(())
}
