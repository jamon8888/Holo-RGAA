//! JSON Schema of the P-01 `mesures` object (P-00 acceptance criterion 6).

use crate::mesures::Mesures;
use rgaa_analysis::{analysis_error, Result};
use schemars::schema_for;
use serde_json::Value;

/// Dumps the `mesures` schema, to be written under `types/` by CI.
///
/// # Errors
///
/// Returns [`rgaa_analysis::RgaaError::Analysis`] when the schema cannot be
/// serialised.
pub fn mesures_schema() -> Result<Value> {
    serde_json::to_value(schema_for!(Mesures))
        .map_err(|error| analysis_error("contrast", format!("schéma mesures invalide : {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_has_no_status_or_verdict() {
        let schema = mesures_schema().expect("schema");
        let text = schema.to_string();
        assert!(!text.contains("\"status\""));
        assert!(!text.contains("\"verdict\""));
        assert!(text.contains("fond_p95"));
    }
}
