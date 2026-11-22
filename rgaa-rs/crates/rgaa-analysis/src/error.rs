//! Error helpers for the analysis contract.

pub use rgaa_core::error::{Result, RgaaError};

/// Builds the domain error every plugin failure must use.
pub fn analysis_error(plugin: impl Into<String>, message: impl Into<String>) -> RgaaError {
    RgaaError::Analysis {
        plugin: plugin.into(),
        message: message.into(),
    }
}

/// The orchestrator's review queue label for a plugin that produced no proof.
pub fn missing_proof(plugin: &str, reason: &str) -> RgaaError {
    analysis_error(
        plugin,
        format!("preuve manquante (sidecar {plugin} : {reason})"),
    )
}
