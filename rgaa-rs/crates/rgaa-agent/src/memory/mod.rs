pub mod schema;

use lancedb::Connection;
use lancedb::query::{ExecutableQuery, QueryBase};
use rig_core::completion::Message;
use rig_core::memory::{ConversationMemory, MemoryError};
use rig_core::wasm_compat::WasmBoxedFuture;
use std::sync::Arc;

use crate::embeddings::HybridEmbeddingProvider;

pub struct LanceDbMemory {
    db: Connection,
    embedding_model: Arc<HybridEmbeddingProvider>,
}

impl LanceDbMemory {
    pub async fn new(
        path: &str,
        embedding_model: HybridEmbeddingProvider,
    ) -> Result<Self, crate::error::AgentError> {
        let db = lancedb::connect(path)
            .execute()
            .await
            .map_err(|e| crate::error::AgentError::LanceDb(e.to_string()))?;

        Ok(Self {
            db,
            embedding_model: Arc::new(embedding_model),
        })
    }

    pub async fn initialize_tables(path: &str) -> Result<(), crate::error::AgentError> {
        let db = lancedb::connect(path)
            .execute()
            .await
            .map_err(|e| crate::error::AgentError::LanceDb(e.to_string()))?;

        let table_names = db
            .table_names()
            .execute()
            .await
            .map_err(|e| crate::error::AgentError::LanceDb(e.to_string()))?;

        if !table_names.contains(&"conversation_messages".to_string()) {
            db.create_empty_table("conversation_messages", schema::conversation_messages_schema())
                .execute()
                .await
                .map_err(|e| crate::error::AgentError::LanceDb(e.to_string()))?;
        }

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

            let _stream = table
                .query()
                .only_if(format!("conversation_id = '{}'", conversation_id))
                .execute()
                .await
                .map_err(MemoryError::backend)?;

            // TODO: deserialize record batches into Vec<Message>
            Ok(Vec::new())
        })
    }

    fn append<'a>(
        &'a self,
        _conversation_id: &'a str,
        _messages: Vec<Message>,
    ) -> WasmBoxedFuture<'a, Result<(), MemoryError>> {
        Box::pin(async move {
            // TODO: serialize messages and insert into LanceDB
            Ok(())
        })
    }

    fn clear<'a>(
        &'a self,
        _conversation_id: &'a str,
    ) -> WasmBoxedFuture<'a, Result<(), MemoryError>> {
        Box::pin(async move {
            // TODO: delete messages for conversation_id
            Ok(())
        })
    }
}
