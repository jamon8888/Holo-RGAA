//! Luminance relative WCAG et amplitude du rouge saturé (spec §3).
//!
//! `rgaa-analysis` n'expose aucun module `color` sur cette base (P-00 fournit
//! seulement le contrat des preuves) : la formule sRGB est donc locale, avec
//! cette note. Les constantes sont celles de WCAG 2.1 / RGAA 4.1.2.

/// Linéarisation d'un canal sRGB 8 bits.
fn linearise(canal: u8) -> f32 {
    let canal_normalise = f32::from(canal) / 255.0;
    if canal_normalise <= 0.04045 {
        canal_normalise / 12.92
    } else {
        ((canal_normalise + 0.055) / 1.055).powf(2.4)
    }
}

/// Luminance relative d'un pixel, entre 0.0 (noir) et 1.0 (blanc).
#[must_use]
pub fn relative_luminance(r: u8, g: u8, b: u8) -> f32 {
    0.2126 * linearise(r) + 0.7152 * linearise(g) + 0.0722 * linearise(b)
}

/// Amplitude « red flash » : `(R - G - B) * 320` sur canaux sRGB normalisés.
///
/// Un pixel n'est retenu que s'il est rouge saturé, c'est-à-dire si
/// `R / (R + G + B) >= 0,8` (définition W3C). Les autres pixels valent 0 : un
/// passage rouge → noir → rouge forme bien deux changements opposés.
#[must_use]
pub fn red_metric(r: u8, g: u8, b: u8) -> f32 {
    let total = u32::from(r) + u32::from(g) + u32::from(b);
    if total == 0 {
        return 0.0;
    }
    if f32::from(r) / (total as f32) < 0.8 {
        return 0.0;
    }
    let amplitude = (f32::from(r) - f32::from(g) - f32::from(b)) / 255.0 * 320.0;
    amplitude.max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_primaires_donnent_les_coefficients_wcag() {
        assert!((relative_luminance(255, 0, 0) - 0.2126).abs() < 1e-4);
        assert!((relative_luminance(0, 255, 0) - 0.7152).abs() < 1e-4);
        assert!((relative_luminance(0, 0, 255) - 0.0722).abs() < 1e-4);
    }
}
