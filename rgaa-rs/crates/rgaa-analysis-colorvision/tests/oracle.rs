//! Oracle values from independent publications — never recomputed by the code under test.
//!
//! * CVD outputs: the 3×3 matrices published by Machado, Oliveira & Fernandes
//!   (2009, Table 1, severity 1.0) at <https://inf.ufrgs.br/~oliveira/pubs_files/CVD_Simulation/CVD_Simulation.html>,
//!   and the Brettel/Viénot tritan projection planes published by DaltonLens
//!   at <https://daltonlens.org/cvd-simulation-svg-filters/>. The expected 8-bit
//!   values below were calculated once, by hand in an independent spreadsheet
//!   script, from those published coefficients (linear sRGB → matrix → clamp →
//!   sRGB encode), and are hard-coded here.
//! * Contrast ratios: WebAIM contrast checker reference pairs.
//! * ΔE2000: Sharma, Wu & Dalal (2005), CIEDE2000 test data set, pair 1.

use rgaa_analysis_colorvision::color::{
    contrast_ratio, delta_e_ciede2000_lab, parse_color, relative_luminance,
};
use rgaa_analysis_colorvision::matrices::{brettel_tritan, machado, CvdKind};
use rgaa_analysis_colorvision::simulate::simulate_rgb;

type Rgb = [u8; 3];

const SIXTEEN_COLOURS: [(&str, Rgb); 16] = [
    ("white", [255, 255, 255]),
    ("black", [0, 0, 0]),
    ("red", [255, 0, 0]),
    ("green", [0, 255, 0]),
    ("blue", [0, 0, 255]),
    ("yellow", [255, 255, 0]),
    ("cyan", [0, 255, 255]),
    ("magenta", [255, 0, 255]),
    ("orange", [255, 165, 0]),
    ("purple", [128, 0, 128]),
    ("grey50", [128, 128, 128]),
    ("brown", [165, 42, 42]),
    ("pink", [255, 192, 203]),
    ("lime", [50, 205, 50]),
    ("navy", [0, 0, 128]),
    ("teal", [0, 128, 128]),
];

/// Asserts each channel is within ±1/255 of the published reference.
fn assert_within_one(name: &str, expected: Rgb, actual: Rgb) {
    for channel in 0..3 {
        let delta = i16::from(expected[channel]) - i16::from(actual[channel]);
        assert!(
            delta.abs() <= 1,
            "{name}: channel {channel}: expected {} ±1, got {}",
            expected[channel],
            actual[channel]
        );
    }
}

#[test]
fn machado_matrices_are_the_published_severity_one_matrices() {
    let protan = machado(CvdKind::Protanopia, 1.0);
    let deutan = machado(CvdKind::Deuteranopia, 1.0);
    // Machado 2009, Table 1, row "1.0".
    assert_eq!(
        protan.rows,
        [
            [0.152286, 1.052583, -0.204868],
            [0.114503, 0.786281, 0.099216],
            [-0.003882, -0.048116, 1.051998],
        ]
    );
    assert_eq!(
        deutan.rows,
        [
            [0.367322, 0.860646, -0.227968],
            [0.280085, 0.672501, 0.047413],
            [-0.011820, 0.042940, 0.968881],
        ]
    );
}

#[test]
fn brettel_tritan_planes_are_the_published_planes() {
    let tritan = brettel_tritan();
    assert_eq!(
        tritan.h1.rows,
        [
            [1.01354, 0.14268, -0.15622],
            [-0.01181, 0.87561, 0.13619],
            [0.07707, 0.81208, 0.11085],
        ]
    );
    assert_eq!(
        tritan.h2.rows,
        [
            [0.93337, 0.19999, -0.13336],
            [0.05809, 0.82565, 0.11626],
            [-0.37923, 1.13825, 0.24098],
        ]
    );
}

#[test]
fn red_deuteranopia_matches_the_published_reference() {
    // Machado 2009 deutan severity 1.0 applied to #ff0000.
    assert_within_one(
        "#ff0000 deutan",
        [163, 144, 0],
        simulate_rgb([255, 0, 0], CvdKind::Deuteranopia, 1.0),
    );
}

#[test]
fn machado_outputs_match_sixteen_independent_references() {
    const EXPECTED: [Rgb; 16] = [
        [255, 255, 255], // white
        [0, 0, 0],       // black
        [109, 95, 0],    // red
        [255, 229, 0],   // green
        [0, 89, 255],    // blue
        [255, 244, 0],   // yellow
        [237, 242, 255], // cyan
        [0, 127, 255],   // magenta
        [195, 172, 0],   // orange
        [0, 61, 131],    // purple
        [128, 128, 128], // grey50
        [78, 71, 41],    // brown
        [201, 202, 203], // pink
        [209, 185, 13],  // lime
        [0, 40, 131],    // navy
        [119, 121, 128], // teal
    ];
    for ((name, rgb), expected) in SIXTEEN_COLOURS.iter().zip(EXPECTED) {
        let actual = simulate_rgb(*rgb, CvdKind::Protanopia, 1.0);
        assert_within_one(&format!("{name} protan"), expected, actual);
    }
}

#[test]
fn brettel_tritan_outputs_match_sixteen_independent_references() {
    const EXPECTED: [Rgb; 16] = [
        [255, 255, 255], // white
        [0, 0, 0],       // black
        [255, 0, 78],    // red
        [124, 234, 255], // green
        [0, 96, 135],    // blue
        [255, 239, 242], // yellow
        [73, 248, 255],  // cyan
        [238, 99, 120],  // magenta
        [255, 153, 166], // orange
        [119, 46, 57],   // purple
        [128, 128, 128], // grey50
        [166, 38, 63],   // brown
        [254, 193, 199], // pink
        [107, 189, 217], // lime
        [0, 44, 64],     // navy
        [32, 125, 148],  // teal
    ];
    for ((name, rgb), expected) in SIXTEEN_COLOURS.iter().zip(EXPECTED) {
        let actual = simulate_rgb(*rgb, CvdKind::Tritanopia, 1.0);
        assert_within_one(&format!("{name} tritan"), expected, actual);
    }
}

#[test]
fn achromatopsia_outputs_match_sixteen_independent_references() {
    const EXPECTED: [Rgb; 16] = [
        [255, 255, 255], // white
        [0, 0, 0],       // black
        [127, 127, 127], // red
        [220, 220, 220], // green
        [76, 76, 76],    // blue
        [247, 247, 247], // yellow
        [229, 229, 229], // cyan
        [145, 145, 145], // magenta
        [184, 184, 184], // orange
        [70, 70, 70],    // purple
        [128, 128, 128], // grey50
        [88, 88, 88],    // brown
        [208, 208, 208], // pink
        [178, 178, 178], // lime
        [33, 33, 33],    // navy
        [115, 115, 115], // teal
    ];
    for ((name, rgb), expected) in SIXTEEN_COLOURS.iter().zip(EXPECTED) {
        let actual = simulate_rgb(*rgb, CvdKind::Achromatopsia, 1.0);
        assert_within_one(&format!("{name} achromatopsia"), expected, actual);
    }
}

#[test]
fn wcag_ratios_match_webaim_reference_values() {
    // WebAIM contrast checker: #ccc on #fff = 1.61:1, #808080 on #666 = 1.46:1,
    // white on #7f7f7f = 4.00:1, black on white = 21:1.
    let cases: [(&str, &str, f64); 4] = [
        ("#cccccc", "#ffffff", 1.61),
        ("#808080", "#666666", 1.46),
        ("#ffffff", "#7f7f7f", 4.00),
        ("#000000", "#ffffff", 21.0),
    ];
    for (a, b, expected) in cases {
        let ratio = contrast_ratio(
            parse_color(a).expect("parse a"),
            parse_color(b).expect("parse b"),
        );
        assert!(
            (ratio - expected).abs() < 0.01,
            "{a} on {b}: expected {expected}, got {ratio}"
        );
    }
}

#[test]
fn link_in_text_ratio_is_below_the_f73_threshold() {
    // WebAIM: #0000ee on #000000 = 2.23:1 (< 3:1, F73).
    let link = parse_color("#0000ee").expect("link");
    let text = parse_color("#000000").expect("text");
    assert!((contrast_ratio(link, text) - 2.23).abs() < 0.01);
    assert!((relative_luminance(text)).abs() < f64::EPSILON);
}

#[test]
fn ciede2000_matches_sharma_reference_pair_one() {
    // Sharma et al. 2005, test data set, pair 1: ΔE00 = 2.0425.
    // Sharma's data set is for the classic CIEDE2000; P-02 uses the improved
    // variant, cross-checked separately against palette's two implementations.
    let delta = delta_e_ciede2000_lab([50.0, 2.6772, -79.7751], [50.0, 0.0, -82.7485]);
    assert!(
        (delta - 2.0425).abs() < 0.001,
        "expected 2.0425 ±0.001, got {delta}"
    );
}

#[test]
fn exported_matrices_json_matches_the_code() {
    let document: serde_json::Value =
        serde_json::from_str(rgaa_analysis_colorvision::matrices::MATRICES_JSON)
            .expect("exported matrices JSON");
    let severity = &document["machado"]["1.0"]["deuteranopia"];
    let expected = machado(CvdKind::Deuteranopia, 1.0).rows;
    for row in 0..3 {
        for col in 0..3 {
            let exported = severity[row][col].as_f64().expect("number");
            assert!((exported - expected[row][col]).abs() < 1e-6);
        }
    }
}
