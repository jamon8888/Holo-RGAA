//! Small shared helpers for querying a LanceDB table and extracting typed
//! columns from the resulting `RecordBatch`es. Used by [`super::store`] and
//! [`super::patterns`].

use crate::error::AgentError;
use arrow::array::{Array, Float32Array, Int32Array, RecordBatch, StringArray};

/// Escapes a single-quote-delimited SQL literal for use in `only_if`.
pub(super) fn escape_literal(s: &str) -> String {
    s.replace('\'', "''")
}

pub(super) fn lancedb_err(e: impl std::fmt::Display) -> AgentError {
    AgentError::LanceDb(e.to_string())
}

pub(super) async fn collect_batches(
    query: lancedb::query::VectorQuery,
) -> Result<Vec<RecordBatch>, AgentError> {
    use futures::TryStreamExt;
    use lancedb::query::ExecutableQuery;
    query
        .execute()
        .await
        .map_err(lancedb_err)?
        .try_collect::<Vec<_>>()
        .await
        .map_err(lancedb_err)
}

pub(super) fn utf8_column<'a>(
    batch: &'a RecordBatch,
    name: &str,
) -> Result<&'a StringArray, AgentError> {
    batch
        .column_by_name(name)
        .and_then(|c| c.as_any().downcast_ref::<StringArray>())
        .ok_or_else(|| AgentError::LanceDb(format!("missing/invalid column `{name}`")))
}

/// As [`utf8_column`], but reads one cell as `None` when it is SQL NULL —
/// for nullable text columns (e.g. `before_html`/`after_html`).
pub(super) fn nullable_utf8(array: &StringArray, i: usize) -> Option<String> {
    if array.is_null(i) {
        None
    } else {
        Some(array.value(i).to_string())
    }
}

pub(super) fn f32_column<'a>(
    batch: &'a RecordBatch,
    name: &str,
) -> Result<&'a Float32Array, AgentError> {
    batch
        .column_by_name(name)
        .and_then(|c| c.as_any().downcast_ref::<Float32Array>())
        .ok_or_else(|| AgentError::LanceDb(format!("missing/invalid column `{name}`")))
}

pub(super) fn i32_column<'a>(
    batch: &'a RecordBatch,
    name: &str,
) -> Result<&'a Int32Array, AgentError> {
    batch
        .column_by_name(name)
        .and_then(|c| c.as_any().downcast_ref::<Int32Array>())
        .ok_or_else(|| AgentError::LanceDb(format!("missing/invalid column `{name}`")))
}
