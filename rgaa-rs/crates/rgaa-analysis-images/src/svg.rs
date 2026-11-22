//! Rasterisation des SVG inline et `.svg` avant OCR et CLIP (P-03 §3).

use image::{DynamicImage, ImageBuffer, RgbaImage};
use resvg::tiny_skia::{Pixmap, Transform};
use rgaa_analysis::{analysis_error, Result};
use std::sync::{Arc, OnceLock};
use usvg::Tree;

/// `usvg::Options::default()` a une base de polices vide : sans cela, les
/// `<text>` disparaissent silencieusement à la rasterisation (banc OCR FR).
fn options() -> usvg::Options<'static> {
    static FONTES: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    let fontes = FONTES.get_or_init(|| {
        let mut base = usvg::fontdb::Database::new();
        base.load_system_fonts();
        Arc::new(base)
    });
    usvg::Options {
        fontdb: fontes.clone(),
        ..usvg::Options::default()
    }
}

/// Rasterise un SVG en RGBA, en bornant la plus grande dimension à `max_size`.
///
/// # Errors
///
/// Renvoie [`rgaa_analysis::RgaaError::Analysis`] si le SVG est invalide ou si
/// la pixmap ne peut pas être allouée.
pub fn rasteriser(svg: &[u8], max_size: u32) -> Result<DynamicImage> {
    let arbre = Tree::from_data(svg, &options())
        .map_err(|error| analysis_error("images", format!("SVG illisible : {error}")))?;
    let taille = arbre.size();
    let (largeur, hauteur) = adapter_taille(taille.width(), taille.height(), max_size);
    let mut pixmap = Pixmap::new(largeur, hauteur).ok_or_else(|| {
        analysis_error(
            "images",
            format!("pixmap SVG impossible : {largeur}x{hauteur}"),
        )
    })?;
    resvg::render(
        &arbre,
        Transform::from_scale(
            largeur as f32 / taille.width(),
            hauteur as f32 / taille.height(),
        ),
        &mut pixmap.as_mut(),
    );
    let pixels: Vec<u8> = pixmap.take().to_vec();
    let image: RgbaImage = ImageBuffer::from_raw(largeur, hauteur, pixels)
        .ok_or_else(|| analysis_error("images", "pixels SVG de taille incohérente"))?;
    Ok(DynamicImage::ImageRgba8(image))
}

/// Conserve le ratio en bornant la plus grande dimension.
fn adapter_taille(largeur: f32, hauteur: f32, max_size: u32) -> (u32, u32) {
    let facteur = (f64::from(max_size) / f64::from(largeur.max(hauteur))).min(1.0);
    (
        (f64::from(largeur) * facteur).ceil().max(1.0) as u32,
        (f64::from(hauteur) * facteur).ceil().max(1.0) as u32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aspect_ratio_is_preserved_when_scaling_down() {
        assert_eq!(adapter_taille(1024.0, 512.0, 768), (768, 384));
        assert_eq!(adapter_taille(64.0, 32.0, 768), (64, 32));
    }
}
