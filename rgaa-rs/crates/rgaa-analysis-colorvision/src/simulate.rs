//! Applying the simulations to pixels, and perceptual hashes for chart series (P-02 §4).

use image::{GrayImage, RgbaImage};
use image_hasher::{HashAlg, HasherConfig};

use crate::color::{encode_channel, linearize_channel, Rgb8};
use crate::matrices::{simulate_linear, CvdKind};

/// Simulates one 8-bit colour. Thin wrapper kept next to the image code so
/// callers only need one module for pixel work.
pub fn simulate_rgb(colour: Rgb8, kind: CvdKind, severity: f64) -> Rgb8 {
    crate::matrices::simulate_rgb(colour, kind, severity)
}

/// Simulates a full RGBA image in linear sRGB, preserving alpha.
///
/// Channel linearisation is tabulated once per call: a 1280×4000 capture needs
/// 5 million pixels, and a `powf` per channel per pixel would dominate the
/// runtime. The matrix multiplication itself stays in `simulate_linear`, so
/// there is still a single definition of each CVD model.
pub fn simulate_image(image: &RgbaImage, kind: CvdKind, severity: f64) -> RgbaImage {
    let linear_lut: Vec<f64> = (0..=255).map(linearize_channel).collect();
    let mut out = image.clone();
    for (source, target) in image.pixels().zip(out.pixels_mut()) {
        let linear = [
            linear_lut[source[0] as usize],
            linear_lut[source[1] as usize],
            linear_lut[source[2] as usize],
        ];
        let simulated = simulate_linear(linear, kind, severity);
        target[0] = encode_channel(simulated[0]);
        target[1] = encode_channel(simulated[1]);
        target[2] = encode_channel(simulated[2]);
    }
    out
}

/// Difference hash (dHash, 8×8 → 64 bits) of one image.
pub fn dhash(image: &GrayImage) -> u64 {
    let hasher = HasherConfig::new()
        .hash_alg(HashAlg::Gradient)
        .hash_size(8, 8)
        .to_hasher();
    let hash = hasher.hash_image(image);
    let bytes = hash.as_bytes();
    let mut out = [0u8; 8];
    if let Some(first_eight) = bytes.get(..8) {
        out.copy_from_slice(first_eight);
    }
    u64::from_le_bytes(out)
}

/// Hamming distance between two dHashes. `<= 4` means "confounded" in P-02 §4.
pub fn dhash_distance(a: &GrayImage, b: &GrayImage) -> u32 {
    (dhash(a) ^ dhash(b)).count_ones()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulation_preserves_alpha_and_dimensions() {
        let image = RgbaImage::from_pixel(3, 2, image::Rgba([255, 0, 0, 128]));
        let simulated = simulate_image(&image, CvdKind::Deuteranopia, 1.0);
        assert_eq!(simulated.dimensions(), (3, 2));
        assert_eq!(simulated.get_pixel(1, 1)[3], 128);
        let expected = crate::matrices::simulate_rgb([255, 0, 0], CvdKind::Deuteranopia, 1.0);
        let actual = simulated.get_pixel(0, 0).0;
        assert_eq!(&actual[..3], &expected);
    }

    #[test]
    fn identical_images_have_zero_hamming_distance() {
        let image = GrayImage::from_pixel(16, 16, image::Luma([42]));
        assert_eq!(dhash_distance(&image, &image), 0);
    }
}
