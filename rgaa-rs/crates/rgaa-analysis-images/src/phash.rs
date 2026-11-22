//! pHash (image_hasher) pour le dédoublonnage et le cache de décision (P-03 §3).

use image::DynamicImage;
use image_hasher::{FilterType, HashAlg, HasherConfig};
use rgaa_analysis::{analysis_error, Result};

/// Deux images dont la distance de Hamming est ≤ 6 comptent comme des copies.
pub const DISTANCE_MAX_REPETITION: u32 = 6;

/// Empreinte perceptuelle 8×8 (64 bits), stable pour les copies redimensionnées.
///
/// # Errors
///
/// Renvoie [`rgaa_analysis::RgaaError::Analysis`] si la taille de hash produite
/// n'est pas 64 bits.
pub fn calculer(image: &DynamicImage) -> Result<u64> {
    let hasher = HasherConfig::new()
        .hash_size(8, 8)
        .resize_filter(FilterType::Lanczos3)
        // pHash = DCT + médiane (image_hasher n'a pas de variante `PHash`).
        .hash_alg(HashAlg::Median)
        .preproc_dct()
        .to_hasher();
    let bytes = hasher.hash_image(image).as_bytes().to_vec();
    let tableau: [u8; 8] = bytes.try_into().map_err(|bytes: Vec<u8>| {
        analysis_error(
            "images",
            format!("pHash de taille inattendue : {}", bytes.len()),
        )
    })?;
    Ok(u64::from_be_bytes(tableau))
}

/// Distance de Hamming entre deux pHash.
#[must_use]
pub fn distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

/// Vrai quand deux images sont des copies au sens de la spec.
#[must_use]
pub fn sont_des_copies(a: u64, b: u64) -> bool {
    distance(a, b) <= DISTANCE_MAX_REPETITION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_of_known_bit_patterns() {
        // Valeurs arithmétiques indépendantes de image_hasher.
        assert_eq!(distance(0b0000, 0b0000), 0);
        assert_eq!(distance(0b0000, 0b1111), 4);
        assert_eq!(distance(0b1010, 0b0110), 2);
    }
}
