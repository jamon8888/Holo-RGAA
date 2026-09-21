use crate::vector::schema::EMBEDDING_DIM;
use lancedb::arrow::arrow_schema::{DataType, Field, Schema, SchemaRef};
use std::sync::Arc;

/// Table name for the versioned regulatory-corpus index (seeded by #128,
/// queried read-only by [`super::tools::ReferentielSearchTool`]).
pub const RAG_REFERENTIEL_TABLE: &str = "rag_referentiel";

/// Table name for the per-audit crawl index (written and purged by #129,
/// queried read-only by [`super::tools::CrawlSearchTool`]).
pub const RAG_CRAWL_TABLE: &str = "rag_crawl";

/// Schema for [`RAG_REFERENTIEL_TABLE`]: one row per chunk of the regulatory
/// corpus, chunked per RGAA test so each version has enough rows for an ANN
/// index (see ticket #128).
pub fn rag_referentiel_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("test_id", DataType::Utf8, false),
        Field::new("referentiel_version", DataType::Utf8, false),
        Field::new("content", DataType::Utf8, false),
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

/// Schema for [`RAG_CRAWL_TABLE`]: one row per piece of structured
/// deep-extraction evidence indexed for the current audit. `expires_at` is a
/// Unix-seconds timestamp; rows past it are eligible for purge on the next
/// run (ticket #129). Never populated from raw crawler HTML — only from
/// structured evidence.
pub fn rag_crawl_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("url", DataType::Utf8, false),
        Field::new("captured_at", DataType::Utf8, false),
        Field::new("evidence_hash", DataType::Utf8, false),
        Field::new("content", DataType::Utf8, false),
        Field::new("expires_at", DataType::Int64, true),
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
