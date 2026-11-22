//! P-01 oracle — reference ratios come from the WebAIM Contrast Checker and
//! are hardcoded here. They are never recomputed from the implementation.

use rgaa_analysis_contrast::{contrast_ratio, parse_color};

fn ratio(foreground: &str, background: &str) -> f64 {
    let foreground = parse_color(foreground).expect("foreground colour");
    let background = parse_color(background).expect("background colour");
    contrast_ratio(foreground, background)
}

#[test]
fn wcag_ratios_match_webaim_reference_values() {
    // WebAIM Contrast Checker, read 2026-11-21.
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
        let actual = ratio(foreground, background);
        assert!(
            (actual - expected).abs() < 0.011,
            "{foreground} on {background}: WebAIM {expected}, got {actual:.3}"
        );
    }
}
