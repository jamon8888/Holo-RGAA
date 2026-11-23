//! Step B — measurement on pixels (P-01 §3 B): Otsu separation, worst-zone
//! background percentiles, CIEDE2000 sanity check and a PNG artefact.

use crate::mesures::{
    estimate_indice, is_large_text, text_tests, text_threshold, Etape, Measurement, Mesures,
};
use crate::snapshot::TextNode;
use image::{GrayImage, ImageFormat, Luma, RgbaImage};
use imageproc::region_labelling::{connected_components, Connectivity};
use rgaa_analysis::color::{
    contrast_ratio, delta_e_2000 as delta_e2000, parse_color, relative_luminance, Color,
};
use rgaa_analysis::{Artefact, PageCapture, Portee, Rect, RgbaImage as CaptureImage};
use std::io::Cursor;

/// Crops a capture to a CSS-pixel bounding box scaled by the device pixel ratio.
/// Returns `None` when the box has no area inside the image.
#[must_use]
pub fn crop(image: &CaptureImage, bbox: Rect, dpr: f32) -> Option<RgbaImage> {
    let scale = if dpr.is_finite() && dpr > 0.0 {
        f64::from(dpr)
    } else {
        1.0
    };
    let x0 = (bbox.x * scale).floor().max(0.0) as u32;
    let y0 = (bbox.y * scale).floor().max(0.0) as u32;
    let x1 = ((bbox.x + bbox.width) * scale)
        .ceil()
        .min(f64::from(image.width)) as u32;
    let y1 = ((bbox.y + bbox.height) * scale)
        .ceil()
        .min(f64::from(image.height)) as u32;
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let width = x1 - x0;
    let height = y1 - y0;
    Some(RgbaImage::from_fn(width, height, |x, y| {
        let source_x = x0 + x;
        let source_y = y0 + y;
        let offset = (source_y as usize * image.width as usize + source_x as usize) * 4;
        image::Rgba([
            image.pixels[offset],
            image.pixels[offset + 1],
            image.pixels[offset + 2],
            image.pixels[offset + 3],
        ])
    }))
}

/// Relative luminance of a pixel, rescaled to 0..=255 for Otsu's method.
fn luma_byte(color: Color) -> u8 {
    (relative_luminance(color) * 255.0)
        .round()
        .clamp(0.0, 255.0) as u8
}

fn color_at(image: &RgbaImage, x: u32, y: u32) -> Color {
    let pixel = image.get_pixel(x, y);
    Color {
        r: pixel[0],
        g: pixel[1],
        b: pixel[2],
        a: pixel[3],
    }
}

fn sorted_by_luminance(mut colors: Vec<Color>) -> Vec<Color> {
    colors.sort_by(|a, b| {
        relative_luminance(*a)
            .total_cmp(&relative_luminance(*b))
            .then_with(|| a.r.cmp(&b.r))
            .then_with(|| a.g.cmp(&b.g))
            .then_with(|| a.b.cmp(&b.b))
    });
    colors
}

#[must_use]
pub fn median(colors: &[Color]) -> Option<Color> {
    if colors.is_empty() {
        return None;
    }
    Some(sorted_by_luminance(colors.to_vec())[colors.len() / 2])
}

fn percentile(colors: &[Color], percent: usize) -> Option<Color> {
    if colors.is_empty() {
        return None;
    }
    let sorted = sorted_by_luminance(colors.to_vec());
    let index = (sorted.len() * percent / 100).min(sorted.len() - 1);
    Some(sorted[index])
}

type LabelImage = image::ImageBuffer<Luma<u32>, Vec<u32>>;

/// Label image and colours of every connected component of a binary mask
/// (label 0 is the background class).
fn component_groups(binary: &GrayImage, image: &RgbaImage) -> (LabelImage, Vec<Vec<Color>>) {
    let labels: LabelImage = connected_components(binary, Connectivity::Four, Luma([0]));
    let mut groups: Vec<Vec<Color>> = Vec::new();
    for (index, label) in labels.pixels().enumerate() {
        let label_index = label.0[0] as usize;
        if label_index == 0 {
            continue;
        }
        if groups.len() <= label_index {
            groups.resize_with(label_index + 1, Vec::new);
        }
        let x = index as u32 % binary.width();
        let y = index as u32 / binary.width();
        groups[label_index].push(color_at(image, x, y));
    }
    (labels, groups)
}

/// Largest non-empty component; ties are resolved by the lowest label index.
fn choose_component(groups: &[Vec<Color>]) -> Option<usize> {
    groups
        .iter()
        .enumerate()
        .skip(1)
        .filter(|(_, colors)| !colors.is_empty())
        .min_by(|(left_index, left), (right_index, right)| {
            right
                .len()
                .cmp(&left.len())
                .then_with(|| left_index.cmp(right_index))
        })
        .map(|(index, _)| index)
}

/// Separates the text from the background (§3 B.2).
///
/// Otsu binarises luminance and the minority class is the text. With a CSS
/// colour, pixels within a tight luminance band around it are the candidates
/// and the largest connected component is the glyph run. A tight band is
/// needed: CIEDE2000 compresses the difference between white and a light photo
/// background, which would otherwise make the whole background look like text.
/// On a gradient Otsu splits the *background* into two large classes, so the
/// CSS anchor is required to find the glyphs. CIEDE2000 is kept for the
/// coherence check in [`measure_text`], as the spec requires.
fn text_mask(image: &RgbaImage, expected: Option<Color>) -> Option<Vec<bool>> {
    let (width, height) = image.dimensions();
    let grays = GrayImage::from_fn(width, height, |x, y| {
        Luma([luma_byte(color_at(image, x, y))])
    });
    let level = imageproc::contrast::otsu_level(&grays);

    if let Some(css) = expected {
        let target = i32::from(luma_byte(css));
        let binary = GrayImage::from_fn(width, height, |x, y| {
            let luma = i32::from(grays.get_pixel(x, y).0[0]);
            Luma([if (luma - target).abs() <= 3 { 255 } else { 0 }])
        });
        let (labels, groups) = component_groups(&binary, image);
        if let Some(chosen) = choose_component(&groups) {
            return Some(
                labels
                    .pixels()
                    .map(|label| label.0[0] == chosen as u32)
                    .collect(),
            );
        }
    }

    let dark_count = grays.pixels().filter(|pixel| pixel.0[0] <= level).count();
    let light_count = width as usize * height as usize - dark_count;
    if dark_count == 0 || light_count == 0 {
        return None;
    }
    let dark_is_text = dark_count < light_count;
    let binary = GrayImage::from_fn(width, height, |x, y| {
        Luma([if (grays.get_pixel(x, y).0[0] <= level) == dark_is_text {
            255
        } else {
            0
        }])
    });
    let (labels, groups) = component_groups(&binary, image);
    let chosen = choose_component(&groups)?;
    Some(
        labels
            .pixels()
            .map(|label| label.0[0] == chosen as u32)
            .collect(),
    )
}

/// Encodes the crop with the text mask overlaid in red (§3 B.4).
fn artefact(crop: &RgbaImage, mask: &[bool]) -> Option<Artefact> {
    let mut overlay = crop.clone();
    for (x, y) in (0..crop.width()).flat_map(|x| (0..crop.height()).map(move |y| (x, y))) {
        if mask[y as usize * crop.width() as usize + x as usize] {
            let pixel = overlay.get_pixel_mut(x, y);
            pixel.0 = [pixel.0[0] / 2 + 127, pixel.0[1] / 2, pixel.0[2] / 2, 255];
        }
    }
    let mut bytes = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(overlay)
        .write_to(&mut bytes, ImageFormat::Png)
        .ok()?;
    let data = bytes.into_inner();
    let sha256 = rgaa_analysis::sha256_hex(&data);
    Some(Artefact {
        nom: format!("{sha256}.png"),
        media_type: "image/png".to_owned(),
        chemin_bundle: format!("analysis/contrast/{sha256}.png"),
        sha256,
    })
}

/// Runs step B for one text node.
#[must_use]
pub fn measure_text(
    node: &TextNode,
    etat: crate::mesures::Etat,
    page: &PageCapture<'_>,
    size_px: f64,
    weight: u16,
    expected_color: Option<Color>,
) -> Measurement {
    let large = is_large_text(size_px, weight);
    let threshold = text_threshold(large);
    let mut mesures = Mesures {
        etape: Etape::B,
        etat,
        ratio: None,
        seuil: Some(threshold),
        texte_grand: large,
        couleur_avant: expected_color.map(Color::to_hex),
        couleur_fond: None,
        fond_p5: None,
        fond_p95: None,
        taille_px: Some(size_px),
        graisse: Some(weight),
        raison: None,
    };
    let mut measurement = Measurement::new(
        mesures.clone(),
        Portee::Estimee,
        text_tests(size_px, weight),
    );
    let (Some(screenshot), Some(bbox)) = (page.screenshot, node.bbox) else {
        mesures.raison = Some("capture ou bbox indisponible".to_owned());
        measurement.mesures = mesures;
        return measurement.with_indice(estimate_indice(0.0, threshold, "capture indisponible"));
    };
    let Some(crop) = crop(screenshot, bbox, page.dpr) else {
        mesures.raison = Some("bbox hors de la capture".to_owned());
        measurement.mesures = mesures;
        return measurement.with_indice(estimate_indice(0.0, threshold, "bbox hors de la capture"));
    };
    let Some(mask) = text_mask(&crop, expected_color) else {
        mesures.raison = Some("séparation texte/fond incertaine".to_owned());
        measurement.mesures = mesures;
        return measurement.with_indice(estimate_indice(
            0.0,
            threshold,
            "séparation texte/fond incertaine",
        ));
    };
    let mut text_colors = Vec::new();
    let mut background_colors = Vec::new();
    for y in 0..crop.height() {
        for x in 0..crop.width() {
            if mask[y as usize * crop.width() as usize + x as usize] {
                text_colors.push(color_at(&crop, x, y));
            } else {
                background_colors.push(color_at(&crop, x, y));
            }
        }
    }
    let Some(text_color) = median(&text_colors) else {
        mesures.raison = Some("séparation texte/fond incertaine".to_owned());
        measurement.mesures = mesures;
        return measurement.with_indice(estimate_indice(
            0.0,
            threshold,
            "séparation texte/fond incertaine",
        ));
    };
    let (Some(p5), Some(p95)) = (
        percentile(&background_colors, 5),
        percentile(&background_colors, 95),
    ) else {
        mesures.raison = Some("fond absent de la vignette".to_owned());
        measurement.mesures = mesures;
        return measurement.with_indice(estimate_indice(
            0.0,
            threshold,
            "fond absent de la vignette",
        ));
    };
    // Coherence check first (§3 B.2). When the sampled median agrees with the
    // computed CSS colour, the CSS colour is the exact text colour and is used
    // for the luminance; the pixels still decide the background percentiles.
    let separation_ok = expected_color.is_none_or(|css| delta_e2000(css, text_color) < 10.0);
    let text_color = if separation_ok {
        expected_color.unwrap_or(text_color)
    } else {
        text_color
    };
    // "Pire zone" practice: keep the least favourable of the two extremes.
    let ratio = contrast_ratio(text_color, p5).min(contrast_ratio(text_color, p95));
    mesures.ratio = Some(ratio);
    mesures.couleur_avant = Some(text_color.to_hex());
    mesures.couleur_fond = Some(p95.to_hex());
    mesures.fond_p5 = Some(p5.to_hex());
    mesures.fond_p95 = Some(p95.to_hex());
    if !separation_ok {
        mesures.raison = Some("séparation texte/fond incertaine".to_owned());
    }
    let indice = if !separation_ok {
        estimate_indice(0.0, threshold, "séparation texte/fond incertaine")
    } else {
        estimate_indice(ratio, threshold, "pire zone sous le seuil")
    };
    measurement.mesures = mesures;
    if let Some(artefact) = artefact(&crop, &mask) {
        measurement = measurement.with_artefact(artefact);
    }
    measurement.with_indice(indice)
}

/// Parses a CSS colour string, kept here so the colour module stays the single
/// place where colour strings are interpreted.
#[must_use]
pub fn css_color(raw: Option<&str>) -> Option<Color> {
    raw.and_then(parse_color)
}
