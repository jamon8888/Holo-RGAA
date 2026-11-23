//! Local model cache (P-00 §6).
//!
//! A plugin never downloads during an audit (R-P3). These helpers only read
//! and verify the cache; `rgaa models pull` is the sole writer.

use crate::error::{analysis_error, Result};
use crate::sha256_hex;
use crate::types::ModelSpec;
use std::fs;
use std::path::{Path, PathBuf};

/// `$RGAA_MODELS_DIR`, or `~/.cache/rgaa/models` by default.
pub fn models_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("RGAA_MODELS_DIR") {
        return PathBuf::from(dir);
    }
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_owned()))
        .join(".cache/rgaa/models")
}

pub fn path_in(dir: &Path, spec: &ModelSpec) -> PathBuf {
    dir.join(spec.nom)
}

pub fn is_present(spec: &ModelSpec) -> bool {
    path_in(&models_dir(), spec).is_file()
}

/// Checks a file against the model's expected digest.
pub fn verify_file(spec: &ModelSpec, path: &Path) -> Result<()> {
    let bytes = fs::read(path).map_err(|error| {
        analysis_error(
            "models",
            format!("impossible de lire le modèle {}: {error}", spec.nom),
        )
    })?;
    let actual = sha256_hex(&bytes);
    if actual != spec.sha256 {
        return Err(analysis_error(
            "models",
            format!(
                "empreinte du modèle {} invalide : attendu {}, trouvé {}",
                spec.nom, spec.sha256, actual
            ),
        ));
    }
    Ok(())
}

/// Fails with an actionable message when the model is absent or corrupted.
pub fn ensure_present_in(plugin: &str, spec: &ModelSpec, dir: &Path) -> Result<()> {
    let path = path_in(dir, spec);
    if !path.is_file() {
        return Err(analysis_error(
            plugin,
            format!(
                "modèle absent : lancez `rgaa models pull {plugin}` ; preuve manquante ({plugin})"
            ),
        ));
    }
    verify_file(spec, &path).map_err(|error| match error {
        rgaa_core::RgaaError::Analysis { message, .. } => {
            analysis_error(plugin, format!("{message} ; preuve manquante ({plugin})"))
        }
        other => analysis_error(plugin, other.to_string()),
    })
}

pub fn ensure_present(plugin: &str, spec: &ModelSpec) -> Result<()> {
    ensure_present_in(plugin, spec, &models_dir())
}

/// Installs downloaded bytes in the cache, refusing a digest mismatch.
pub fn install(spec: &ModelSpec, bytes: &[u8]) -> Result<PathBuf> {
    let actual = sha256_hex(bytes);
    if actual != spec.sha256 {
        return Err(analysis_error(
            "models",
            format!(
                "empreinte du modèle {} invalide : attendu {}, trouvé {}",
                spec.nom, spec.sha256, actual
            ),
        ));
    }
    let dir = models_dir();
    fs::create_dir_all(&dir).map_err(|error| {
        analysis_error(
            "models",
            format!("impossible de créer {}: {error}", dir.display()),
        )
    })?;
    let path = path_in(&dir, spec);
    fs::write(&path, bytes).map_err(|error| {
        analysis_error(
            "models",
            format!("impossible d'écrire {}: {error}", path.display()),
        )
    })?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> ModelSpec {
        ModelSpec {
            nom: "tiny.bin",
            url: "https://example.test/tiny.bin",
            sha256: Box::leak(sha256_hex(b"tiny").into_boxed_str()),
            taille_octets: 4,
            licence: "MIT",
        }
    }

    #[test]
    fn install_then_verify_accepts_the_right_bytes() {
        let dir = tempfile::tempdir().expect("temp dir");
        let spec = spec();
        fs::write(path_in(dir.path(), &spec), b"tiny").expect("write");
        ensure_present_in("images", &spec, dir.path()).expect("valid model");
    }

    #[test]
    fn a_corrupted_model_is_refused() {
        let dir = tempfile::tempdir().expect("temp dir");
        let spec = spec();
        fs::write(path_in(dir.path(), &spec), b"evil").expect("write");
        let error = ensure_present_in("images", &spec, dir.path()).expect_err("bad digest");
        assert!(error.to_string().contains("empreinte"), "{error}");
    }
}
