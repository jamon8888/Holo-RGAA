//! Colour primitives shared by the plugin and by P-01.
//!
//! [`contrast_ratio`] is the single WCAG 2.1 relative-luminance formula of the
//! repository. It is deliberately byte-for-byte compatible with the formula
//! written for #439 in `rgaa-obscura/src/contrast.rs`; see the P-02 PR for the
//! migration note once PR #456 is merged (R-P1: a plugin never renders a status).

use palette::color_difference::{Ciede2000, ImprovedCiede2000};
use palette::{IntoColor, Lab, Srgb};

/// Converts 8-bit sRGB to CIE Lab via linear sRGB.
fn to_lab(colour: Rgb8) -> Lab {
    let srgb = Srgb::new(
        f32::from(colour[0]) / 255.0,
        f32::from(colour[1]) / 255.0,
        f32::from(colour[2]) / 255.0,
    );
    srgb.into_color()
}

/// An 8-bit sRGB colour, in `R, G, B` order.
pub type Rgb8 = [u8; 3];

/// Parses the CSS colour notations needed by computed styles: `#rgb`,
/// `#rrggbb`, `rgb(r, g, b)` and `rgb(r g b)`. Alpha, when present, is ignored:
/// the caller is responsible for compositing semi-transparent layers.
pub fn parse_color(input: &str) -> Option<Rgb8> {
    let raw = input.trim();
    if let Some(hex) = raw.strip_prefix('#') {
        let digit = |part: &str| u8::from_str_radix(part, 16).ok();
        return match hex.len() {
            3 => Some([
                digit(hex.get(0..1)?)? * 17,
                digit(hex.get(1..2)?)? * 17,
                digit(hex.get(2..3)?)? * 17,
            ]),
            6 => Some([
                digit(hex.get(0..2)?)?,
                digit(hex.get(2..4)?)?,
                digit(hex.get(4..6)?)?,
            ]),
            _ => None,
        };
    }
    let inside = raw.strip_prefix("rgb(")?.strip_suffix(')')?;
    let parts: Vec<&str> = inside
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|part| !part.is_empty())
        .take(4)
        .collect();
    if parts.len() != 3 && parts.len() != 4 {
        return None;
    }
    let channel = |part: &str| -> Option<u8> {
        let value = part.trim_end_matches('%').parse::<f64>().ok()?;
        if part.ends_with('%') {
            let scaled = value * 255.0 / 100.0;
            Some(scaled.round().clamp(0.0, 255.0) as u8)
        } else {
            Some(value.round().clamp(0.0, 255.0) as u8)
        }
    };
    Some([channel(parts[0])?, channel(parts[1])?, channel(parts[2])?])
}

/// Linearises one 8-bit sRGB channel (WCAG 2.1, threshold 0.04045).
pub fn linearize_channel(channel: u8) -> f64 {
    let value = f64::from(channel) / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

/// Encodes one linear sRGB channel back to an 8-bit value, clamping out-of-
/// gamut results first. CVD matrices routinely produce negative components.
pub fn encode_channel(linear: f64) -> u8 {
    let linear = linear.clamp(0.0, 1.0);
    let encoded = if linear <= 0.0031308 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0).round().clamp(0.0, 255.0) as u8
}

/// WCAG 2.1 relative luminance (Rec. 709 coefficients on linearised sRGB).
pub fn relative_luminance(colour: Rgb8) -> f64 {
    0.2126 * linearize_channel(colour[0])
        + 0.7152 * linearize_channel(colour[1])
        + 0.0722 * linearize_channel(colour[2])
}

/// WCAG 2.1 contrast ratio, from 1.0 to 21.0.
///
/// This is the one WCAG formula used by the repository: P-01 calls it through
/// the plugin crate instead of keeping a second copy in `rgaa-obscura`.
pub fn contrast_ratio(a: Rgb8, b: Rgb8) -> f64 {
    let light = relative_luminance(a).max(relative_luminance(b));
    let dark = relative_luminance(a).min(relative_luminance(b));
    (light + 0.05) / (dark + 0.05)
}

/// ΔE2000 (`ImprovedCiede2000`, Huang et al.) between two sRGB colours.
pub fn delta_e_2000(a: Rgb8, b: Rgb8) -> f64 {
    delta_e_srgb(a, b)
}

fn delta_e_srgb(a: Rgb8, b: Rgb8) -> f64 {
    f64::from(to_lab(a).improved_difference(to_lab(b)))
}

/// ΔE2000 (`ImprovedCiede2000`, Huang et al.) between two CIE L\*a\*b\* colours.
pub fn delta_e_2000_lab(a: [f64; 3], b: [f64; 3]) -> f64 {
    let (lab_a, lab_b) = labs(a, b);
    f64::from(lab_a.improved_difference(lab_b))
}

/// Classic CIEDE2000, used to check the implementation against the published
/// Sharma, Wu & Dalal (2005) data set. P-02 §2 selects the improved variant for
/// the plugin itself; this function exists only as an independent cross-check
/// of the Lab conversion.
pub fn delta_e_ciede2000_lab(a: [f64; 3], b: [f64; 3]) -> f64 {
    let (lab_a, lab_b) = labs(a, b);
    f64::from(lab_a.difference(lab_b))
}

fn labs(a: [f64; 3], b: [f64; 3]) -> (Lab, Lab) {
    (
        Lab::new(a[0] as f32, a[1] as f32, a[2] as f32),
        Lab::new(b[0] as f32, b[1] as f32, b[2] as f32),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_supported_css_notation() {
        assert_eq!(parse_color("#fff"), Some([255, 255, 255]));
        assert_eq!(parse_color("#0000ee"), Some([0, 0, 238]));
        assert_eq!(parse_color("rgb(0, 0, 238)"), Some([0, 0, 238]));
        assert_eq!(parse_color("rgb(0 0 238)"), Some([0, 0, 238]));
        assert_eq!(parse_color("nope"), None);
    }

    #[test]
    fn black_and_white_have_the_extreme_ratio() {
        assert_eq!(contrast_ratio([0, 0, 0], [255, 255, 255]), 21.0);
    }
}
