//! The single colour module of the repository (P-00 contract, used by P-01/P-02).
//!
//! Relative luminance uses the WCAG 2.1 sRGB linearisation threshold `0.04045`
//! (not the older `0.03928`). Contrast ratios composite a translucent
//! foreground over its background before comparing, which is what the #439
//! Obscura measurement and the P-01 plugin both need. Colour differences use
//! `palette`'s CIEDE2000; the classic variant is checked against the Sharma,
//! Wu & Dalal (2005) reference data, and the improved variant is exposed for
//! P-02's colour-vision work.
//!
//! There must be no second copy of these formulae in another crate. PR #456
//! (`rgaa-obscura/src/contrast.rs`) will be migrated after it is merged.

use palette::color_difference::{Ciede2000, ImprovedCiede2000};
use palette::{IntoColor, Lab, Srgb};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An 8-bit sRGB colour with an 8-bit alpha channel (255 = opaque).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    /// An opaque colour.
    #[must_use]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Lowercase `#rrggbb`, or `#rrggbbaa` when translucent.
    #[must_use]
    pub fn to_hex(self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }

    /// The three colour channels, for callers that do not use alpha.
    #[must_use]
    pub const fn channels(self) -> [u8; 3] {
        [self.r, self.g, self.b]
    }
}

/// Parses the CSS colour notations a computed style can expose: `#rgb`,
/// `#rgba`, `#rrggbb`, `#rrggbbaa`, `rgb()`/`rgba()` with either comma or
/// whitespace separation, and `transparent`.
#[must_use]
pub fn parse_color(raw: &str) -> Option<Color> {
    let raw = raw.trim();
    if raw.eq_ignore_ascii_case("transparent") {
        return Some(Color {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        });
    }
    if let Some(hex) = raw.strip_prefix('#') {
        let digits = hex.as_bytes();
        let expand = |byte: &u8| -> Option<u8> {
            (byte.to_ascii_lowercase() as char)
                .to_digit(16)
                .map(|digit| (digit * 17) as u8)
        };
        let pair = |high: &u8, low: &u8| -> Option<u8> {
            let high = (high.to_ascii_lowercase() as char).to_digit(16)?;
            let low = (low.to_ascii_lowercase() as char).to_digit(16)?;
            Some(((high << 4) | low) as u8)
        };
        return match digits.len() {
            3 => Some(Color::rgb(
                expand(&digits[0])?,
                expand(&digits[1])?,
                expand(&digits[2])?,
            )),
            4 => Some(Color {
                r: expand(&digits[0])?,
                g: expand(&digits[1])?,
                b: expand(&digits[2])?,
                a: expand(&digits[3])?,
            }),
            6 => Some(Color::rgb(
                pair(&digits[0], &digits[1])?,
                pair(&digits[2], &digits[3])?,
                pair(&digits[4], &digits[5])?,
            )),
            8 => Some(Color {
                r: pair(&digits[0], &digits[1])?,
                g: pair(&digits[2], &digits[3])?,
                b: pair(&digits[4], &digits[5])?,
                a: pair(&digits[6], &digits[7])?,
            }),
            _ => None,
        };
    }
    let inner = raw
        .strip_prefix("rgba(")
        .or_else(|| raw.strip_prefix("rgb("))?
        .strip_suffix(')')?;
    let channels: Vec<&str> = inner
        .split(|character: char| character == ',' || character.is_whitespace())
        .filter(|part| !part.is_empty())
        .collect();
    if channels.len() != 3 && channels.len() != 4 {
        return None;
    }
    let channel = |value: &str| -> Option<u8> {
        let (number, percent) = match value.strip_suffix('%') {
            Some(number) => (number, true),
            None => (value, false),
        };
        let scalar = number.trim().parse::<f64>().ok()?;
        let scaled = if percent {
            scalar * 255.0 / 100.0
        } else {
            scalar
        };
        Some(scaled.clamp(0.0, 255.0).round() as u8)
    };
    let alpha = match channels.get(3) {
        Some(value) => {
            let scalar = value.trim_end_matches('%').parse::<f64>().ok()?;
            let fraction = if value.ends_with('%') {
                scalar / 100.0
            } else {
                scalar
            };
            (fraction.clamp(0.0, 1.0) * 255.0).round() as u8
        }
        None => 255,
    };
    Some(Color {
        r: channel(channels[0])?,
        g: channel(channels[1])?,
        b: channel(channels[2])?,
        a: alpha,
    })
}

/// Linearises one 8-bit sRGB channel (WCAG 2.1 threshold 0.04045).
#[must_use]
pub fn linearize_channel(channel: u8) -> f64 {
    let value = f64::from(channel) / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

/// Encodes a linear sRGB channel back to 8 bits, clamping out-of-gamut values.
/// Colour-vision matrices routinely produce negative components (P-02).
#[must_use]
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
/// Alpha is ignored: callers composite first with [`composite`].
#[must_use]
pub fn relative_luminance(color: Color) -> f64 {
    0.2126 * linearize_channel(color.r)
        + 0.7152 * linearize_channel(color.g)
        + 0.0722 * linearize_channel(color.b)
}

/// Alpha-composites `foreground` over `background`.
#[must_use]
pub fn composite(foreground: Color, background: Color) -> Color {
    let alpha = f64::from(foreground.a) / 255.0;
    let blend = |over: u8, under: u8| -> u8 {
        (f64::from(over) * alpha + f64::from(under) * (1.0 - alpha)).round() as u8
    };
    Color {
        r: blend(foreground.r, background.r),
        g: blend(foreground.g, background.g),
        b: blend(foreground.b, background.b),
        a: 255,
    }
}

/// Composites background layers listed from the text outwards, stopping at
/// the first opaque one. `None` means unknown: callers must turn that into a
/// review, never into an assumed white background.
#[must_use]
pub fn compose_background(layers: &[Color]) -> Option<Color> {
    let opaque = layers.iter().rposition(|color| color.a == 255)?;
    let mut base = layers[opaque];
    for top in layers[..opaque].iter().rev() {
        base = composite(*top, base);
    }
    Some(base)
}

/// WCAG 2.1 contrast ratio `(L1 + 0.05) / (L2 + 0.05)`, translucent foreground
/// composited over the background first.
#[must_use]
pub fn contrast_ratio(foreground: Color, background: Color) -> f64 {
    let seen = composite(foreground, background);
    let light = relative_luminance(seen).max(relative_luminance(background));
    let dark = relative_luminance(seen).min(relative_luminance(background));
    (light + 0.05) / (dark + 0.05)
}

fn to_lab(color: Color) -> Lab {
    Srgb::new(
        f32::from(color.r) / 255.0,
        f32::from(color.g) / 255.0,
        f32::from(color.b) / 255.0,
    )
    .into_linear()
    .into_color()
}

fn labs(a: [f64; 3], b: [f64; 3]) -> (Lab, Lab) {
    (
        Lab::new(a[0] as f32, a[1] as f32, a[2] as f32),
        Lab::new(b[0] as f32, b[1] as f32, b[2] as f32),
    )
}

/// Classic CIEDE2000 between two sRGB colours. Used by P-01's pixel/text
/// coherence check and validated against the Sharma data set below.
#[must_use]
pub fn delta_e_2000(a: Color, b: Color) -> f64 {
    f64::from(to_lab(a).difference(to_lab(b)))
}

/// Classic CIEDE2000 between two CIE L\*a\*b\* colours.
#[must_use]
pub fn delta_e_2000_lab(a: [f64; 3], b: [f64; 3]) -> f64 {
    let (lab_a, lab_b) = labs(a, b);
    f64::from(lab_a.difference(lab_b))
}

/// Improved CIEDE2000 (Huang et al.) between two sRGB colours, for P-02.
#[must_use]
pub fn delta_e_improved_2000(a: Color, b: Color) -> f64 {
    f64::from(to_lab(a).improved_difference(to_lab(b)))
}

/// Improved CIEDE2000 between two CIE L\*a\*b\* colours, for P-02.
#[must_use]
pub fn delta_e_improved_2000_lab(a: [f64; 3], b: [f64; 3]) -> f64 {
    let (lab_a, lab_b) = labs(a, b);
    f64::from(lab_a.improved_difference(lab_b))
}

/// Parses a computed `font-size` in CSS pixels.
#[must_use]
pub fn parse_px(raw: &str) -> Option<f64> {
    raw.trim().strip_suffix("px")?.trim().parse::<f64>().ok()
}

/// Parses a computed `font-weight` (`normal`, `bold` or a numeric value).
#[must_use]
pub fn parse_weight(raw: &str) -> u16 {
    match raw.trim() {
        "bold" | "bolder" => 700,
        "normal" | "lighter" | "" => 400,
        other => other.parse::<u16>().unwrap_or(400),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WebAIM Contrast Checker values, read 2026-11-21. Hardcoded on purpose:
    /// they are an external oracle, never recomputed from the code under test.
    #[test]
    fn wcag_ratios_match_webaim_reference_values() {
        let cases = [
            ("#000000", "#ffffff", 21.0),
            ("#ffffff", "#000000", 21.0),
            ("#767676", "#ffffff", 4.54),
            ("#777777", "#ffffff", 4.48),
            ("#777777", "#000000", 4.69),
            ("#cccccc", "#ffffff", 1.61),
            ("#ffffff", "#7f7f7f", 4.0),
            ("#808080", "#666666", 1.46),
        ];
        for (foreground, background, expected) in cases {
            let actual = contrast_ratio(
                parse_color(foreground).expect("foreground"),
                parse_color(background).expect("background"),
            );
            assert!(
                (actual - expected).abs() < 0.011,
                "{foreground} on {background}: WebAIM {expected}, got {actual:.3}"
            );
        }
    }

    #[test]
    fn alpha_layers_compose_over_the_first_opaque_ancestor() {
        let layers = [
            parse_color("rgba(0, 0, 0, 0.5019608)").expect("layer"),
            parse_color("rgb(255, 255, 255)").expect("ancestor"),
        ];
        assert_eq!(compose_background(&layers), Some(Color::rgb(127, 127, 127)));
        assert_eq!(compose_background(&layers[..1]), None);
    }

    #[test]
    fn parses_the_css_colour_forms_a_computed_style_uses() {
        assert_eq!(parse_color("#fff"), Some(Color::rgb(255, 255, 255)));
        assert_eq!(parse_color("rgb(0 0 238)"), Some(Color::rgb(0, 0, 238)));
        assert_eq!(
            parse_color("rgb(100%, 0%, 0%)"),
            Some(Color::rgb(255, 0, 0))
        );
        assert_eq!(
            parse_color("transparent"),
            Some(Color {
                r: 0,
                g: 0,
                b: 0,
                a: 0
            })
        );
        assert_eq!(parse_color("nope"), None);
    }

    /// Sharma, Wu & Dalal (2005), CIEDE2000 test data set, pairs 1–4.
    #[test]
    fn ciede2000_matches_sharma_reference_values() {
        let pairs: [([f64; 3], [f64; 3], f64); 4] = [
            ([50.0, 2.6772, -79.7751], [50.0, 0.0, -82.7485], 2.0425),
            ([50.0, 3.1571, -77.2803], [50.0, 0.0, -82.7485], 2.8615),
            ([50.0, 2.8361, -74.0200], [50.0, 0.0, -82.7485], 3.4412),
            ([50.0, -1.3802, -84.2814], [50.0, 0.0, -82.7485], 1.0000),
        ];
        for (a, b, expected) in pairs {
            let actual = delta_e_2000_lab(a, b);
            assert!(
                (actual - expected).abs() < 0.0005,
                "Sharma {a:?}/{b:?}: {expected}, got {actual:.4}"
            );
        }
    }

    #[test]
    fn font_metrics_are_parsed_from_computed_styles() {
        assert_eq!(parse_px("18.66px"), Some(18.66));
        assert_eq!(parse_px("1.5em"), None);
        assert_eq!(parse_weight("700"), 700);
        assert_eq!(parse_weight("bold"), 700);
        assert_eq!(parse_weight("normal"), 400);
    }
}
