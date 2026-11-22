//! Step A — CSS measurement on a reliably known background (P-01 §3 A).
//!
//! When the background chain is unknown (image, gradient, opacity, filter,
//! blend mode, overlap) the node is handed to step B instead of receiving an
//! invented white background. This is the same non-negotiable rule as PR #456.

use crate::mesures::{
    is_large_text, text_tests, text_threshold, Etape, Etat, Measurement, Mesures,
};
use crate::pixels;
use crate::snapshot::TextNode;
use rgaa_analysis::color::{
    compose_background, contrast_ratio, parse_color, parse_px, parse_weight, Color,
};
use rgaa_analysis::{Indice, PageCapture, Portee};

/// Default computed font size when the snapshot omits it, in CSS pixels.
const DEFAULT_SIZE_PX: f64 = 16.0;

#[must_use]
fn metrics(node: &TextNode) -> (f64, u16) {
    let size = node
        .font_size
        .as_deref()
        .and_then(parse_px)
        .unwrap_or(DEFAULT_SIZE_PX);
    let weight = node.font_weight.as_deref().map_or(400, parse_weight);
    (size, weight)
}

/// Builds the `Neutre` proof for a text with no applicable threshold (§3 A).
fn neutral(node: &TextNode, etat: Etat, size: f64, weight: u16, reason: &str) -> Measurement {
    let layers: Vec<Color> = node
        .layers
        .iter()
        .filter_map(|layer| parse_color(layer))
        .collect();
    let background = compose_background(&layers);
    let foreground = node.color.as_deref().and_then(parse_color);
    let ratio = match (foreground, background) {
        (Some(text), Some(background)) => Some(contrast_ratio(text, background)),
        _ => None,
    };
    let mesures = Mesures {
        etape: Etape::A,
        etat,
        ratio,
        seuil: None,
        texte_grand: is_large_text(size, weight),
        couleur_avant: foreground.map(Color::to_hex),
        couleur_fond: background.map(Color::to_hex),
        fond_p5: None,
        fond_p95: None,
        taille_px: Some(size),
        graisse: Some(weight),
        raison: Some(reason.to_owned()),
    };
    Measurement::new(mesures, Portee::Mesuree, text_tests(size, weight)).with_indice(Indice::Neutre)
}

/// Measures one text node, falling back to pixels when CSS is not reliable.
#[must_use]
pub fn measure_text(node: &TextNode, etat: Etat, page: &PageCapture<'_>) -> Measurement {
    let (size, weight) = metrics(node);
    let tests = text_tests(size, weight);
    let large = is_large_text(size, weight);
    let threshold = text_threshold(large);

    if node.disabled {
        return neutral(node, etat, size, weight, "désactivé");
    }
    if node.logo {
        return neutral(node, etat, size, weight, "logotype");
    }
    if node.decorative {
        return neutral(node, etat, size, weight, "décoratif");
    }

    let foreground = node.color.as_deref().and_then(parse_color);
    let layers: Vec<Color> = node
        .layers
        .iter()
        .filter_map(|layer| parse_color(layer))
        .collect();
    let background = compose_background(&layers);

    if node.issues.is_empty() {
        if let (Some(foreground), Some(background)) = (foreground, background) {
            let ratio = contrast_ratio(foreground, background);
            let mesures = Mesures {
                etape: Etape::A,
                etat,
                ratio: Some(ratio),
                seuil: Some(threshold),
                texte_grand: large,
                couleur_avant: Some(foreground.to_hex()),
                couleur_fond: Some(background.to_hex()),
                fond_p5: None,
                fond_p95: None,
                taille_px: Some(size),
                graisse: Some(weight),
                raison: None,
            };
            return Measurement::new(mesures, Portee::Mesuree, tests);
        }
    }
    pixels::measure_text(node, etat, page, size, weight, foreground)
}
