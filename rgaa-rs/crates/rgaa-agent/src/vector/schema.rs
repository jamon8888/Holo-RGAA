use lancedb::arrow::arrow_schema::{DataType, Field, Schema, SchemaRef, TimeUnit};
use std::sync::Arc;

/// Shared embedding width. The producer (FastEmbed `all-MiniLM-L6-v2`) and the
/// storage schema must agree on this dimension; declare it once here.
pub const EMBEDDING_DIM: usize = 384;

pub fn rgaa_criteria_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("title", DataType::Utf8, false),
        Field::new("description", DataType::Utf8, false),
        Field::new("classification", DataType::Utf8, false),
        Field::new("wcag_refs", DataType::Utf8, true),
        Field::new(
            "embedding",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                EMBEDDING_DIM as i32,
            ),
            true,
        ),
    ]))
}

pub fn rgaa_findings_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("criterion_id", DataType::Utf8, false),
        Field::new("rule", DataType::Utf8, false),
        Field::new("element_html", DataType::Utf8, true),
        Field::new("page_url", DataType::Utf8, false),
        Field::new("remediation", DataType::Utf8, true),
        Field::new(
            "embedding",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                EMBEDDING_DIM as i32,
            ),
            true,
        ),
        Field::new(
            "created_at",
            DataType::Timestamp(TimeUnit::Second, None),
            false,
        ),
    ]))
}

pub fn rgaa_remediation_patterns_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("rule", DataType::Utf8, false),
        Field::new("framework", DataType::Utf8, false),
        Field::new("before_html", DataType::Utf8, true),
        Field::new("after_html", DataType::Utf8, true),
        Field::new("description", DataType::Utf8, true),
        Field::new("success_count", DataType::Int32, false),
        Field::new(
            "embedding",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                EMBEDDING_DIM as i32,
            ),
            true,
        ),
    ]))
}
