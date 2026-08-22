use lancedb::arrow::arrow_schema::{DataType, Field, Schema, SchemaRef, TimeUnit};
use std::sync::Arc;

pub fn conversation_messages_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("conversation_id", DataType::Utf8, false),
        Field::new("role", DataType::Utf8, false),
        Field::new("content", DataType::Utf8, false),
        Field::new("timestamp", DataType::Timestamp(TimeUnit::Second, None), false),
        Field::new(
            "embedding",
            DataType::List(Arc::new(Field::new("item", DataType::Float32, true))),
            true,
        ),
    ]))
}
