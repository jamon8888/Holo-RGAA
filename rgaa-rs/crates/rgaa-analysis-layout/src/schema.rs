//! JSON Schema of the P-07 `mesures` objects (P-00 acceptance criterion 6).

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
        .map_err(|error| analysis_error("layout", format!("schéma mesures invalide : {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has_key(value: &Value, key: &str) -> bool {
        match value {
            Value::Object(map) => {
                map.contains_key(key) || map.values().any(|child| has_key(child, key))
            }
            Value::Array(items) => items.iter().any(|child| has_key(child, key)),
            _ => false,
        }
    }

    #[test]
    fn schema_has_no_status_or_verdict() {
        let schema = mesures_schema().expect("schema");
        assert!(!has_key(&schema, "status"), "{schema}");
        assert!(!has_key(&schema, "verdict"), "{schema}");
        let text = schema.to_string();
        assert!(text.contains("titres_visuels"));
        assert!(text.contains("sauts_positionnes"));
    }
}
