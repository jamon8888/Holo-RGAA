//! JSON Schema du champ `mesures` du plugin images (P-00 critère 6).

use crate::mesures::Mesures;
use rgaa_analysis::{analysis_error, Result};
use schemars::schema_for;
use serde_json::Value;

/// Schéma à écrire dans `types/` par CI.
///
/// # Errors
///
/// Renvoie [`rgaa_analysis::RgaaError::Analysis`] si la sérialisation échoue.
pub fn mesures_schema() -> Result<Value> {
    serde_json::to_value(schema_for!(Mesures))
        .map_err(|error| analysis_error("images", format!("schéma mesures invalide : {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_has_no_status_or_verdict() {
        let texte = mesures_schema().expect("schema").to_string();
        assert!(!texte.contains("\"status\""));
        assert!(!texte.contains("\"verdict\""));
    }
}
