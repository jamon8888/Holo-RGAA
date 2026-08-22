use lancedb::arrow::arrow_schema::{DataType, Field, Schema, SchemaRef, TimeUnit};
use std::sync::Arc;

pub fn conversation_messages_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::UInt64, false),
        Field::new("conversation_id", DataType::Utf8, false),
        Field::new("role", DataType::Utf8, true),
        Field::new("content", DataType::Utf8, false),
        Field::new("timestamp", DataType::Timestamp(TimeUnit::Nanosecond, None), false),
    ]))
}