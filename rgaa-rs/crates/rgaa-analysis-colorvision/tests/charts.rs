//! Chart series: dHash decides confounding, not a colour-only ΔE (P-02 §4).

use image::{GrayImage, Luma};
use rgaa_analysis_colorvision::color::delta_e_2000;
use rgaa_analysis_colorvision::matrices::CvdKind;
use rgaa_analysis_colorvision::simulate::{dhash_distance, simulate_image};
use std::path::PathBuf;

fn fixture() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bar-chart.svg");
    std::fs::read_to_string(path).expect("SVG fixture")
}

fn fills(svg: &str) -> Vec<rgaa_analysis_colorvision::Rgb8> {
    let mut result = Vec::new();
    for part in svg.split("fill=\"").skip(1) {
        let hex = part.split('"').next().unwrap_or_default();
        if let Some(colour) = rgaa_analysis_colorvision::parse_color(hex) {
            result.push(colour);
        }
    }
    result
}

/// One isolated series: same geometry for both bars, only the fill varies.
fn series_bitmap(colour: rgaa_analysis_colorvision::Rgb8) -> image::RgbaImage {
    image::RgbaImage::from_fn(64, 32, |x, _| {
        image::Rgba([
            colour[0].saturating_sub(x as u8 * 2),
            colour[1].saturating_sub(x as u8 * 2),
            colour[2].saturating_sub(x as u8 * 2),
            255,
        ])
    })
}

fn grey(image: &image::RgbaImage) -> GrayImage {
    GrayImage::from_fn(image.width(), image.height(), |x, y| {
        let pixel = image.get_pixel(x, y);
        // Rec. 709 luminance on the 8-bit channels, as in a browser greyscale.
        let value = (0.2126 * f64::from(pixel[0])
            + 0.7152 * f64::from(pixel[1])
            + 0.0722 * f64::from(pixel[2])) as u8;
        Luma([value])
    })
}

#[test]
fn red_and_green_bars_are_confounded_after_deutan_and_protan() {
    let colours = fills(&fixture());
    assert_eq!(colours, vec![[255, 0, 0], [0, 255, 0]]);
    // Distinguishable for a viewer with normal colour vision.
    assert!(delta_e_2000(colours[0], colours[1]) > 5.0);

    for kind in [CvdKind::Deuteranopia, CvdKind::Protanopia] {
        let simulated: Vec<_> = colours
            .iter()
            .map(|colour| simulate_image(&series_bitmap(*colour), kind, 1.0))
            .collect();
        let distance = dhash_distance(&grey(&simulated[0]), &grey(&simulated[1]));
        assert!(
            distance <= 4,
            "{kind:?}: hamming distance {distance} exceeds the 4-bit threshold"
        );
    }
}
