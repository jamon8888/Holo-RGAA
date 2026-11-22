//! Local model declarations for P-04 (spec §6, R-P3).
//!
//! Models are *declared* here, downloaded only by `rgaa models pull media`,
//! and verified by SHA-256 before use. An audit never fetches them.

use rgaa_analysis::models as contract;
use rgaa_analysis::{ModelSpec, Result};
use std::path::PathBuf;

/// Silero VAD v5, ONNX, MIT. Pinned to a commit so the digest never drifts.
pub const VAD_MODEL: ModelSpec = ModelSpec {
    nom: "silero_vad.onnx",
    url: "https://raw.githubusercontent.com/snakers4/silero-vad/066409f3d9db45a30d21dfcbf9147826ffdd7052/src/silero_vad/data/silero_vad.onnx",
    sha256: "1a153a22f4509e292a94e67d6f9b85e8deb25b4988682b7e174c65279d8788e3",
    taille_octets: 2_327_524,
    licence: "MIT",
};

/// Whisper `small` in ggml format, MIT. Multilingual; default ASR model.
pub const ASR_MODEL: ModelSpec = ModelSpec {
    nom: "ggml-small.bin",
    url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
    sha256: "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
    taille_octets: 487_601_967,
    licence: "MIT",
};

/// Models served by this plugin, in stable order.
pub const MODELES: &[ModelSpec] = &[VAD_MODEL, ASR_MODEL];

/// Cache path of one model. No I/O, no download.
#[must_use]
pub fn path(spec: &ModelSpec) -> PathBuf {
    contract::path_in(&contract::models_dir(), spec)
}

/// Refuses to run when the model is absent or corrupted. The only remedy is
/// `rgaa models pull media`; this function never writes to the cache (R-P3).
///
/// # Errors
/// Returns [`rgaa_analysis::RgaaError::Analysis`] naming the pull command.
pub fn require(spec: &ModelSpec) -> Result<PathBuf> {
    contract::ensure_present("media", spec)?;
    Ok(path(spec))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_specs_are_complete_and_verifiable() {
        for spec in MODELES {
            assert!(spec.url.starts_with("https://"), "{}", spec.nom);
            assert_eq!(spec.sha256.len(), 64, "{}", spec.nom);
            assert!(spec.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()));
            assert!(spec.taille_octets > 0, "{}", spec.nom);
            assert_eq!(spec.licence, "MIT", "{}", spec.nom);
        }
        assert_eq!(MODELES.len(), 2);
    }

    #[test]
    fn an_absent_model_is_an_error_not_an_automatic_download() {
        let dir = tempfile::tempdir().expect("temp dir");
        // A spec that cannot exist in the cache; require() must refuse, not fetch.
        let spec = ModelSpec {
            nom: "absent-p04.onnx",
            url: VAD_MODEL.url,
            sha256: VAD_MODEL.sha256,
            taille_octets: 1,
            licence: "MIT",
        };
        let error = contract::ensure_present_in("media", &spec, dir.path())
            .expect_err("absent model must fail");
        assert!(
            error.to_string().contains("rgaa models pull media"),
            "{error}"
        );
    }
}
