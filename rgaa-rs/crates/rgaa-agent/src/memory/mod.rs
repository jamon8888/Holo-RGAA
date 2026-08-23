pub mod schema;

use arrow::array::{Array, StringArray, TimestampNanosecondArray, UInt64Array};
use arrow::datatypes::SchemaRef;
use arrow::record_batch::RecordBatch;
use futures::TryStreamExt;
use lancedb::database::CreateTableMode;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::Connection;
use rig_core::completion::Message;
use rig_core::memory::{ConversationMemory, MemoryError};
use rig_core::wasm_compat::WasmBoxedFuture;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

static MSG_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

fn escape_single_quotes(s: &str) -> String {
    s.replace('\'', "''")
}

fn role_of(message: &Message) -> &'static str {
    match message {
        Message::System { .. } => "system",
        Message::User { .. } => "user",
        Message::Assistant { .. } => "assistant",
    }
}

/// LanceDB-backed conversation memory.
///
/// Stores and retrieves message histories per conversation ID.
pub struct LanceDbMemory {
    db: Connection,
}

impl LanceDbMemory {
    /// Opens a LanceDB connection and ensures the conversation table exists.
    ///
    /// # Errors
    /// Returns [`crate::error::AgentError::LanceDb`] if the database cannot be
    /// opened or the table initialization fails.
    pub async fn new(path: &str) -> Result<Self, crate::error::AgentError> {
        let db = lancedb::connect(path)
            .execute()
            .await
            .map_err(|e| crate::error::AgentError::LanceDb(e.to_string()))?;
        let store = Self { db };
        store.initialize_tables().await?;
        Ok(store)
    }

    /// Creates the conversation table if absent. Idempotent.
    ///
    /// # Errors
    /// Returns [`crate::error::AgentError::LanceDb`] if the table creation fails.
    pub async fn initialize_tables(&self) -> Result<(), crate::error::AgentError> {
        self.db
            .create_empty_table(
                "conversation_messages",
                schema::conversation_messages_schema(),
            )
            .mode(CreateTableMode::exist_ok(|req| req))
            .execute()
            .await
            .map_err(|e| crate::error::AgentError::LanceDb(e.to_string()))?;
        Ok(())
    }
}

impl ConversationMemory for LanceDbMemory {
    fn load<'a>(
        &'a self,
        conversation_id: &'a str,
    ) -> WasmBoxedFuture<'a, Result<Vec<Message>, MemoryError>> {
        Box::pin(async move {
            let table = self
                .db
                .open_table("conversation_messages")
                .execute()
                .await
                .map_err(MemoryError::backend)?;

            let mut stream = table
                .query()
                .only_if(format!(
                    "conversation_id = '{}'",
                    escape_single_quotes(conversation_id)
                ))
                .execute()
                .await
                .map_err(MemoryError::backend)?;

            let mut entries = Vec::new();
            while let Some(batch) = stream.try_next().await.map_err(MemoryError::backend)? {
                let content = batch
                    .column_by_name("content")
                    .ok_or_else(|| MemoryError::backend("missing content column"))?;
                let content = content
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .ok_or_else(|| MemoryError::backend("content is not utf8"))?;

                let id_col = batch
                    .column_by_name("id")
                    .ok_or_else(|| MemoryError::backend("missing id column"))?
                    .as_any()
                    .downcast_ref::<UInt64Array>()
                    .ok_or_else(|| MemoryError::backend("id is not uint64"))?;

                let ts_col = batch
                    .column_by_name("timestamp")
                    .ok_or_else(|| MemoryError::backend("missing timestamp column"))?
                    .as_any()
                    .downcast_ref::<TimestampNanosecondArray>()
                    .ok_or_else(|| MemoryError::backend("timestamp is not i64"))?;

                for i in 0..content.len() {
                    if content.is_null(i) {
                        continue;
                    }
                    let msg: Message = serde_json::from_str(content.value(i))
                        .map_err(|e| MemoryError::backend(format!("deserialize message: {e}")))?;
                    let id = id_col.value(i);
                    let ts = ts_col.value(i);
                    entries.push((ts, id, msg));
                }
            }

            // Sort by timestamp, then id as deterministic tiebreaker
            entries.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
            let messages = entries.into_iter().map(|e| e.2).collect();

            Ok(messages)
        })
    }

    fn append<'a>(
        &'a self,
        conversation_id: &'a str,
        messages: Vec<Message>,
    ) -> WasmBoxedFuture<'a, Result<(), MemoryError>> {
        Box::pin(async move {
            if messages.is_empty() {
                return Ok(());
            }
            let table = self
                .db
                .open_table("conversation_messages")
                .execute()
                .await
                .map_err(MemoryError::backend)?;

            let schema: SchemaRef = schema::conversation_messages_schema();
            let mut ids = Vec::with_capacity(messages.len());
            let mut conv_ids = Vec::with_capacity(messages.len());
            let mut roles = Vec::with_capacity(messages.len());
            let mut contents = Vec::with_capacity(messages.len());
            let mut timestamps = Vec::with_capacity(messages.len());

            // Use a single timestamp base for all messages to preserve order
            let base_ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos() as i64)
                .unwrap_or(0);

            // Use atomic counter for guaranteed-unique IDs across all calls
            let next_id = MSG_ID_COUNTER.fetch_add(messages.len() as u64, Ordering::Relaxed);

            for (idx, message) in messages.iter().enumerate() {
                let serialized = serde_json::to_string(message)
                    .map_err(|e| MemoryError::backend(format!("serialize message: {e}")))?;
                let id = next_id.saturating_add(idx as u64);
                let ts = base_ts.saturating_add(idx as i64);
                ids.push(id);
                conv_ids.push(conversation_id.to_string());
                roles.push(role_of(message).to_string());
                contents.push(serialized);
                timestamps.push(ts);
            }

            let batch = RecordBatch::try_new(
                schema,
                vec![
                    Arc::new(UInt64Array::from(ids)),
                    Arc::new(StringArray::from(conv_ids)),
                    Arc::new(StringArray::from(roles)),
                    Arc::new(StringArray::from(contents)),
                    Arc::new(TimestampNanosecondArray::from(timestamps)),
                ],
            )
            .map_err(|e| MemoryError::backend(format!("build batch: {e}")))?;

            table
                .add(batch)
                .execute()
                .await
                .map_err(MemoryError::backend)?;
            Ok(())
        })
    }

    fn clear<'a>(
        &'a self,
        conversation_id: &'a str,
    ) -> WasmBoxedFuture<'a, Result<(), MemoryError>> {
        Box::pin(async move {
            let table = self
                .db
                .open_table("conversation_messages")
                .execute()
                .await
                .map_err(MemoryError::backend)?;
            table
                .delete(&format!(
                    "conversation_id = '{}'",
                    escape_single_quotes(conversation_id)
                ))
                .await
                .map_err(MemoryError::backend)?;
            Ok(())
        })
    }
}
