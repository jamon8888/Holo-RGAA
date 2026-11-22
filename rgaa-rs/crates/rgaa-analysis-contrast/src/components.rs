//! Step C — non-textual components (P-01 §3 C): 3 px adjacent-pixel ring,
//! 3:1 threshold, on an estimated proof since the sample can be misleading.

use crate::mesures::{estimate_indice, Etape, Etat, Measurement, Mesures, COMPONENT_THRESHOLD};
use crate::pixels;
use crate::snapshot::{ComponentKind, ComponentNode};
use rgaa_analysis::color::{contrast_ratio, parse_color, Color};
use rgaa_analysis::{PageCapture, Portee, Rect, RgbaImage};

/// Width of the adjacent-pixel ring, in CSS pixels (§3 C).
const RING_CSS_PX: f64 = 3.0;

fn pixel_at(image: &RgbaImage, x: u32, y: u32) -> Color {
    let offset = (y as usize * image.width as usize + x as usize) * 4;
    Color {
        r: image.pixels[offset],
        g: image.pixels[offset + 1],
        b: image.pixels[offset + 2],
        a: image.pixels[offset + 3],
    }
}

fn scale(rect: Rect, dpr: f32) -> Rect {
    let factor = if dpr.is_finite() && dpr > 0.0 {
        f64::from(dpr)
    } else {
        1.0
    };
    Rect {
        x: rect.x * factor,
        y: rect.y * factor,
        width: rect.width * factor,
        height: rect.height * factor,
    }
}

/// Returns the colours of the 3 px CSS ring around a bounding box, clamped to
/// the capture. Coordinates are deterministic: row-major from the top-left.
#[must_use]
pub fn adjacent_pixels(image: &RgbaImage, bbox: Rect, dpr: f32) -> Vec<Color> {
    let inner = scale(bbox, dpr);
    let x0 = (inner.x - RING_CSS_PX * f64::from(dpr)).floor().max(0.0) as u32;
    let y0 = (inner.y - RING_CSS_PX * f64::from(dpr)).floor().max(0.0) as u32;
    let x1 = ((inner.x + inner.width) + RING_CSS_PX * f64::from(dpr))
        .ceil()
        .min(f64::from(image.width)) as u32;
    let y1 = ((inner.y + inner.height) + RING_CSS_PX * f64::from(dpr))
        .ceil()
        .min(f64::from(image.height)) as u32;
    let ix0 = inner.x.floor().max(0.0) as u32;
    let iy0 = inner.y.floor().max(0.0) as u32;
    let ix1 = (inner.x + inner.width).ceil().min(f64::from(image.width)) as u32;
    let iy1 = (inner.y + inner.height).ceil().min(f64::from(image.height)) as u32;
    let mut colors = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            let inside = x >= ix0 && x < ix1 && y >= iy0 && y < iy1;
            if !inside {
                colors.push(pixel_at(image, x, y));
            }
        }
    }
    colors
}

fn empty_measurement(
    node: &ComponentNode,
    etat: Etat,
    reason: &str,
    ratio: f64,
    color: Option<Color>,
) -> Measurement {
    let mesures = Mesures {
        etape: Etape::C,
        etat,
        ratio: Some(ratio),
        seuil: Some(COMPONENT_THRESHOLD),
        texte_grand: false,
        couleur_avant: color.map(Color::to_hex),
        couleur_fond: color.map(Color::to_hex),
        fond_p5: None,
        fond_p95: None,
        taille_px: None,
        graisse: None,
        raison: Some(reason.to_owned()),
    };
    Measurement::new(
        mesures,
        Portee::Estimee,
        node.kind
            .tests()
            .iter()
            .map(|test| (*test).to_owned())
            .collect(),
    )
    .with_indice(estimate_indice(ratio, COMPONENT_THRESHOLD, reason))
}

/// Measures one non-textual component against its adjacent pixels.
#[must_use]
pub fn measure_component(node: &ComponentNode, etat: Etat, page: &PageCapture<'_>) -> Measurement {
    let css_color = node.color.as_deref().and_then(parse_color);
    if node.kind == ComponentKind::FocusRing && !node.present {
        return empty_measurement(node, etat, "contour de focus absent", 1.0, css_color);
    }
    if !node.issues.is_empty() {
        let reason = format!("mesure non fiable : {}", node.issues.join(" ; "));
        return empty_measurement(node, etat, &reason, 0.0, css_color);
    }
    let (Some(screenshot), Some(bbox)) = (page.screenshot, node.bbox) else {
        return empty_measurement(node, etat, "capture ou bbox indisponible", 0.0, css_color);
    };
    let element_color = match css_color {
        Some(color) => color,
        None => {
            let colors: Vec<Color> = pixels::crop(screenshot, bbox, page.dpr)
                .map(|crop| {
                    crop.pixels()
                        .map(|pixel| Color {
                            r: pixel[0],
                            g: pixel[1],
                            b: pixel[2],
                            a: pixel[3],
                        })
                        .collect()
                })
                .unwrap_or_default();
            match pixels::median(&colors) {
                Some(color) => color,
                None => {
                    return empty_measurement(
                        node,
                        etat,
                        "couleur d'élément indisponible",
                        0.0,
                        None,
                    )
                }
            }
        }
    };
    let adjacent = adjacent_pixels(screenshot, bbox, page.dpr);
    let Some((worst_ratio, worst_color)) = adjacent
        .into_iter()
        .map(|color| (contrast_ratio(element_color, color), color))
        .min_by(|a, b| a.0.total_cmp(&b.0))
    else {
        return empty_measurement(node, etat, "aucun pixel adjacent", 0.0, Some(element_color));
    };
    let mesures = Mesures {
        etape: Etape::C,
        etat,
        ratio: Some(worst_ratio),
        seuil: Some(COMPONENT_THRESHOLD),
        texte_grand: false,
        couleur_avant: Some(element_color.to_hex()),
        couleur_fond: Some(worst_color.to_hex()),
        fond_p5: None,
        fond_p95: None,
        taille_px: None,
        graisse: None,
        raison: None,
    };
    Measurement::new(
        mesures,
        Portee::Estimee,
        node.kind
            .tests()
            .iter()
            .map(|test| (*test).to_owned())
            .collect(),
    )
    .with_indice(estimate_indice(
        worst_ratio,
        COMPONENT_THRESHOLD,
        "contraste insuffisant avec les pixels adjacents",
    ))
}
