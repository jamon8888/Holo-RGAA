pub mod schema;

use futures::TryStreamExt;
use lancedb::arrow::array::{StringArray, TimestampNanosecondArray, UInt64Array};
use lancedb::arrow::record_batch::RecordBatch;
use lancedb::arrow::arrow_schema::SchemaRef;
use lancedb::Connection;
use lancedb::database::CreateTableMode;
use lancedb::query::{ExecutableQuery, QueryBase};
use rig_core::completion::Message;
use rig_core::memory::{ConversationMemory, MemoryError};
use rig_core::wasm_compat::WasmBoxedFuture;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

fn role_of(message: &Message) -> &'static str {
    match message {
        Message::System { .. } => "system",
        Message::User { .. } => "user",
        Message::Assistant { .. } => "assistant",
    }
}

fn now_nanos() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}

pub struct LanceDbMemory {
    db: Connection,
}

impl LanceDbMemory {
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
    pub async fn initialize_tables(&self) -> Result<(), crate::error::AgentError> {
        self.db
            .create_empty_table("conversation_messages", schema::conversation_messages_schema())
            .mode(CreateTableMode::exist_ok(|req| req))
            .execute()
            .await
            .map_err(|e| crate::error::AgentError::LanceDb(e.to_string()))?;
        Ok(())
    }

    async fn append_one(
        &self,
        conversation_id: &str,
        message: &Message,
    ) -> Result<(), crate::error::AgentError> {
        let table = self
            .db
            .open_table("conversation_messages")
            .execute()
            .await
            .map_err(|e| crate::error::AgentError::LanceDb(e.to_string()))?;

        let serialized = serde_json::to_string(message)
            .map_err(|e| crate::error::AgentError::LanceDb(format!("serialize message: {e}")))?;
        let id = now_nanos().max(0) as u64;
        let ts = now_nanos();

        let schema: SchemaRef = schema::conversation_messages_schema();
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(UInt64Array::from(vec![id])),
                Arc::new(StringArray::from(vec![conversation_id.to_string()])),
                Arc::new(StringArray::from(vec![role_of(message).to_string()])),
                Arc::new(StringArray::from(vec![serialized])),
                Arc::new(TimestampNanosecondArray::from(vec![ts])),
            ],
        )
        .map_err(|e| crate::error::AgentError::LanceDb(format!("build batch: {e}")))?;

        table
            .add(batch)
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
                .only_if(format!("conversation_id = '{}'", conversation_id))
                .execute()
                .await
                .map_err(MemoryError::backend)?;

            let mut messages = Vec::new();
            while let Some(batch) = stream.try_next().await.map_err(MemoryError::backend)? {
                let content = batch
                    .column_by_name("content")
                    .ok_or_else(|| MemoryError::backend("missing content column".into()))?;
                let content = content
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .ok_or_else(|| MemoryError::backend("content is not utf8".into()))?;
                for i in 0..content.len() {
                    if content.is_null(i) {
                        continue;
                    }
                    let msg: Message = serde_json::from_str(content.value(i))
                        .map_err(|e| MemoryError::backend(format!("deserialize message: {e}")))?;
                    messages.push(msg);
                }
            }
            Ok(messages)
        })
    }

    fn append<'a>(
        &'a self,
        conversation_id: &'a str,
        messages: Vec<Message>,
    ) -> WasmBoxedFuture<'a, Result<(), MemoryError>> {
        Box::pin(async move {
            for message in &messages {
                self.append_one(conversation_id, message)
                    .await
                    .map_err(MemoryError::backend)?;
            }
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
                .delete(format!("conversation_id = '{}'", conversation_id))
                .await
                .map_err(MemoryError::backend)?;
            Ok(())
        })
    }
}