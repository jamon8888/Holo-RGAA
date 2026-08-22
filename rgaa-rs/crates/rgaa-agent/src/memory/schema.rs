use lancedb::arrow::arrow_schema::{DataType, Field, Schema, SchemaRef, TimeUnit};
use std::sync::Arc;

/// Returns the Arrow schema for the `conversation_messages` table.
///
/// Columns:
/// - `id` (UInt64, not null): Unique message identifier (timestamp-based).
/// - `conversation_id` (Utf8, not null): Conversation identifier.
/// - `role` (Utf8, nullable): Message role (system, user, assistant).
/// - `content` (Utf8, not null): JSON-serialized [`rig_core::completion::Message`].
/// - `timestamp` (Timestamp[Nanosecond], not null): Message creation time.
pub fn conversation_messages_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::UInt64, false),
        Field::new("conversation_id", DataType::Utf8, false),
        Field::new("role", DataType::Utf8, true),
        Field::new("content", DataType::Utf8, false),
        Field::new("timestamp", DataType::Timestamp(TimeUnit::Nanosecond, None), false),
    ]))
}