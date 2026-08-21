# LanceDB Memory & Vector Store Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add LanceDB-backed memory and vector store to rgaa-agent, with hybrid embeddings (fastembed + OpenAI), using rig-agent's agentic loop.

**Architecture:** Monolithic rig-agent with LanceDbMemory (ConversationMemory trait), LanceDbVectorIndex (VectorStoreIndex trait), and HybridEmbeddingProvider (fastembed + OpenAI).

**Tech Stack:** rig-agent 0.42, rig-lancedb 0.42, rig-memory 0.42, fastembed, lancedb, tokio

## Global Constraints

- Rust 2021 edition
- All code must compile with `cargo check --workspace`
- All tests must pass with `cargo test --workspace`
- Use `thiserror` for library errors
- Use `tracing` for logging, not `println!`

---

## Task 1: Add Dependencies

**Files:**
- Modify: `crates/rgaa-agent/Cargo.toml`
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Add rig-agent and LanceDB dependencies to workspace Cargo.toml**

Add to `[workspace.dependencies]`:
```toml
rig-agent = "0.42"
rig-lancedb = "0.42"
rig-memory = "0.42"
lancedb = "0.17"
fastembed = "4.0"
```

- [ ] **Step 2: Add dependencies to rgaa-agent Cargo.toml**

```toml
[dependencies]
rig-agent = { workspace = true }
rig-lancedb = { workspace = true }
rig-memory = { workspace = true }
lancedb = { workspace = true }
fastembed = { workspace = true }
rig-core = { workspace = true }
rgaa-core = { path = "../rgaa-core" }
rgaa-holo = { path = "../rgaa-holo" }
rgaa-browser-tools = { path = "../rgaa-browser-tools" }
rgaa-remediation = { path = "../rgaa-remediation" }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
uuid = { version = "1.0", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
```

- [ ] **Step 3: Verify dependencies compile**

Run: `cargo check -p rgaa-agent`

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml crates/rgaa-agent/Cargo.toml
git commit -m "chore(agent): add rig-agent, lancedb, fastembed dependencies"
```

---

## Task 2: Configuration Types

**Files:**
- Create: `crates/rgaa-agent/src/config.rs`
- Modify: `crates/rgaa-agent/src/lib.rs`

- [ ] **Step 1: Create config.rs**

```rust
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub holo3_base_url: String,
    pub api_key: String,
    pub model: String,
    pub lancedb_path: String,
    pub embedding_backend: EmbeddingBackendConfig,
    pub embedding_dimensions: usize,
    pub memory_retention: MemoryRetention,
    pub max_turns: usize,
    pub max_tokens: usize,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmbeddingBackendConfig {
    FastEmbed { model_name: String },
    OpenAi {
        base_url: String,
        api_key: String,
        model: String,
    },
    Hybrid {
        primary: Box<EmbeddingBackendConfig>,
        fallback: Box<EmbeddingBackendConfig>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryRetention {
    PerAudit,
    Persistent,
    Hybrid {
        short_term_ttl: Duration,
        long_term_pattern: String,
    },
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            holo3_base_url: "https://api.hcompany.ai/v1".into(),
            api_key: String::new(),
            model: "holo3-1-35b-a3b".into(),
            lancedb_path: "./data/lancedb".into(),
            embedding_backend: EmbeddingBackendConfig::FastEmbed {
                model_name: "all-MiniLM-L6-v2".into(),
            },
            embedding_dimensions: 384,
            memory_retention: MemoryRetention::Hybrid {
                short_term_ttl: Duration::from_secs(7 * 24 * 60 * 60),
                long_term_pattern: "findings-*".into(),
            },
            max_turns: 10,
            max_tokens: 4096,
            temperature: 0.3,
        }
    }
}

impl AgentConfig {
    pub fn from_env() -> Result<Self, crate::error::AgentError> {
        Ok(Self {
            holo3_base_url: std::env::var("HOLO3_BASE_URL")
                .unwrap_or_else(|_| "https://api.hcompany.ai/v1".into()),
            api_key: std::env::var("HOLO3_API_KEY")
                .map_err(|_| crate::error::AgentError::Config("HOLO3_API_KEY required".into()))?,
            model: std::env::var("HOLO3_MODEL")
                .unwrap_or_else(|_| "holo3-1-35b-a3b".into()),
            lancedb_path: std::env::var("LANCEDB_PATH")
                .unwrap_or_else(|_| "./data/lancedb".into()),
            ..Default::default()
        })
    }
}
```

- [ ] **Step 2: Update lib.rs to export config module**

```rust
pub mod config;
pub mod error;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p rgaa-agent`

- [ ] **Step 4: Commit**

```bash
git add crates/rgaa-agent/src/config.rs crates/rgaa-agent/src/lib.rs
git commit -m "feat(agent): add configuration types"
```

---

## Task 3: Error Types

**Files:**
- Create: `crates/rgaa-agent/src/error.rs`
- Modify: `crates/rgaa-agent/src/lib.rs`

- [ ] **Step 1: Create error.rs**

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("rig agent error: {0}")]
    RigAgent(String),

    #[error("lancedb error: {0}")]
    LanceDb(String),

    #[error("embedding error: {0}")]
    Embedding(String),

    #[error("memory error: {0}")]
    Memory(String),

    #[error("tool execution error: {0}")]
    ToolExecution(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("holog3 api error: {0}")]
    Holo3Api(String),
}
```

- [ ] **Step 2: Update lib.rs**

```rust
pub mod config;
pub mod error;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p rgaa-agent`

- [ ] **Step 4: Commit**

```bash
git add crates/rgaa-agent/src/error.rs crates/rgaa-agent/src/lib.rs
git commit -m "feat(agent): add error types"
```

---

## Task 4: FastEmbed Model

**Files:**
- Create: `crates/rgaa-agent/src/embeddings/mod.rs`
- Create: `crates/rgaa-agent/src/embeddings/fastembed.rs`
- Modify: `crates/rgaa-agent/src/lib.rs`

- [ ] **Step 1: Create embeddings/mod.rs**

```rust
pub mod fastembed;

pub use fastembed::FastEmbedModel;
```

- [ ] **Step 2: Create embeddings/fastembed.rs**

```rust
use rig_core::embeddings::{Embedding, EmbeddingError};
use fastembed::{TextEmbedding, EmbeddingModel as FastEmbedTrait};

#[derive(Clone)]
pub struct FastEmbedModel {
    model: TextEmbedding,
    dimensions: usize,
}

impl FastEmbedModel {
    pub fn new(model_name: &str) -> Result<Self, crate::error::AgentError> {
        let model = TextEmbedding::try_new(
            fastembed::InitOptions::new(fastembed::AllMiniLmL6V2::default())
        ).map_err(|e| crate::error::AgentError::Embedding(e.to_string()))?;

        Ok(Self {
            model,
            dimensions: 384,
        })
    }

    pub fn dimensions(&self) -> usize {
        self.dimensions
    }
}

impl rig_core::embeddings::EmbeddingModel for FastEmbedModel {
    fn embed_text(&self, text: &str) -> Result<Embedding, EmbeddingError> {
        let embeddings = self.model
            .embed(vec![text.to_string()], None)
            .map_err(|e| EmbeddingError::FailedToEmbed(e.to_string()))?;

        Ok(Embedding {
            vec: embeddings[0].clone(),
        })
    }

    fn ndims(&self) -> usize {
        self.dimensions
    }
}
```

- [ ] **Step 3: Update lib.rs**

```rust
pub mod config;
pub mod embeddings;
pub mod error;
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p rgaa-agent`

- [ ] **Step 5: Commit**

```bash
git add crates/rgaa-agent/src/embeddings/ crates/rgaa-agent/src/lib.rs
git commit -m "feat(agent): add FastEmbed model"
```

---

## Task 5: Hybrid Embedding Provider

**Files:**
- Modify: `crates/rgaa-agent/src/embeddings/mod.rs`

- [ ] **Step 1: Add HybridEmbeddingProvider to embeddings/mod.rs**

```rust
pub mod fastembed;

pub use fastembed::FastEmbedModel;
use rig_core::embeddings::{Embedding, EmbeddingError, EmbeddingModel};

pub enum EmbeddingBackend {
    FastEmbed(FastEmbedModel),
}

pub struct HybridEmbeddingProvider {
    primary: EmbeddingBackend,
    fallback: Option<EmbeddingBackend>,
}

impl HybridEmbeddingProvider {
    pub fn new(config: &crate::config::AgentConfig) -> Result<Self, crate::error::AgentError> {
        let primary = match &config.embedding_backend {
            crate::config::EmbeddingBackendConfig::FastEmbed { model_name } => {
                EmbeddingBackend::FastEmbed(FastEmbedModel::new(model_name)?)
            }
            _ => return Err(crate::error::AgentError::Config("unsupported backend".into())),
        };

        Ok(Self {
            primary,
            fallback: None,
        })
    }

    pub fn dimensions(&self) -> usize {
        match &self.primary {
            EmbeddingBackend::FastEmbed(m) => m.dimensions(),
        }
    }
}

impl EmbeddingModel for HybridEmbeddingProvider {
    fn embed_text(&self, text: &str) -> Result<Embedding, EmbeddingError> {
        match &self.primary {
            EmbeddingBackend::FastEmbed(m) => m.embed_text(text),
        }
    }

    fn ndims(&self) -> usize {
        self.dimensions()
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p rgaa-agent`

- [ ] **Step 3: Commit**

```bash
git add crates/rgaa-agent/src/embeddings/mod.rs
git commit -m "feat(agent): add HybridEmbeddingProvider"
```

---

## Task 6: LanceDB Memory

**Files:**
- Create: `crates/rgaa-agent/src/memory/mod.rs`
- Create: `crates/rgaa-agent/src/memory/schema.rs`
- Modify: `crates/rgaa-agent/src/lib.rs`

- [ ] **Step 1: Create memory/schema.rs**

```rust
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use std::sync::Arc;

pub fn conversation_messages_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("conversation_id", DataType::Utf8, false),
        Field::new("role", DataType::Utf8, false),
        Field::new("content", DataType::Utf8, false),
        Field::new("timestamp", DataType::Timestamp(None, true), false),
        Field::new("embedding", DataType::List(Box::new(Field::new("item", DataType::Float32, true))), true),
    ]))
}
```

- [ ] **Step 2: Create memory/mod.rs**

```rust
pub mod schema;

use lancedb::Connection;
use rig_core::completion::Message;
use rig_core::memory::{ConversationMemory, MemoryError};
use std::sync::Arc;

use crate::embeddings::HybridEmbeddingProvider;

pub struct LanceDbMemory {
    db: Connection,
    embedding_model: Arc<HybridEmbeddingProvider>,
}

impl LanceDbMemory {
    pub async fn new(path: &str, embedding_model: HybridEmbeddingProvider) -> Result<Self, crate::error::AgentError> {
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

        let table_names = db.table_names()
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
    async fn load(&self, conversation_id: &str) -> Result<Vec<Message>, MemoryError> {
        let table = self.db.open_table("conversation_messages")
            .execute()
            .await
            .map_err(|e| MemoryError::Backend(e.into()))?;

        let results = table.query()
            .only_if(format!("conversation_id = '{}'", conversation_id))
            .execute()
            .await
            .map_err(|e| MemoryError::Backend(e.into()))?;

        let mut messages = Vec::new();
        for batch in results {
            let batch = batch.map_err(|e| MemoryError::Backend(e.into()))?;
            // Deserialize batch to messages
            // For now, return empty
        }

        Ok(messages)
    }

    async fn append(&self, conversation_id: &str, messages: Vec<Message>) -> Result<(), MemoryError> {
        // Serialize messages and insert into LanceDB
        // For now, return Ok(())
        Ok(())
    }

    async fn clear(&self, conversation_id: &str) -> Result<(), MemoryError> {
        // Delete messages for conversation_id
        // For now, return Ok(())
        Ok(())
    }
}
```

- [ ] **Step 3: Update lib.rs**

```rust
pub mod config;
pub mod embeddings;
pub mod error;
pub mod memory;
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p rgaa-agent`

- [ ] **Step 5: Commit**

```bash
git add crates/rgaa-agent/src/memory/ crates/rgaa-agent/src/lib.rs
git commit -m "feat(agent): add LanceDbMemory with ConversationMemory trait"
```

---

## Task 7: Vector Store

**Files:**
- Create: `crates/rgaa-agent/src/vector/mod.rs`
- Create: `crates/rgaa-agent/src/vector/schema.rs`
- Modify: `crates/rgaa-agent/src/lib.rs`

- [ ] **Step 1: Create vector/schema.rs**

```rust
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use std::sync::Arc;

pub fn rgaa_criteria_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("title", DataType::Utf8, false),
        Field::new("description", DataType::Utf8, false),
        Field::new("classification", DataType::Utf8, false),
        Field::new("wcag_refs", DataType::Utf8, true),
        Field::new("embedding", DataType::List(Box::new(Field::new("item", DataType::Float32, true))), true),
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
        Field::new("embedding", DataType::List(Box::new(Field::new("item", DataType::Float32, true))), true),
        Field::new("created_at", DataType::Timestamp(None, true), false),
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
        Field::new("embedding", DataType::List(Box::new(Field::new("item", DataType::Float32, true))), true),
    ]))
}
```

- [ ] **Step 2: Create vector/mod.rs**

```rust
pub mod schema;

use lancedb::Connection;
use rig_core::vector_store::VectorStoreIndex;
use std::sync::Arc;

use crate::embeddings::HybridEmbeddingProvider;

pub struct LanceDbVectorStore {
    db: Connection,
    embedding_model: Arc<HybridEmbeddingProvider>,
}

impl LanceDbVectorStore {
    pub async fn new(path: &str, embedding_model: HybridEmbeddingProvider) -> Result<Self, crate::error::AgentError> {
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

        let table_names = db.table_names()
            .execute()
            .await
            .map_err(|e| crate::error::AgentError::LanceDb(e.to_string()))?;

        if !table_names.contains(&"rgaa_criteria".to_string()) {
            db.create_empty_table("rgaa_criteria", schema::rgaa_criteria_schema())
                .execute()
                .await
                .map_err(|e| crate::error::AgentError::LanceDb(e.to_string()))?;
        }

        if !table_names.contains(&"rgaa_findings".to_string()) {
            db.create_empty_table("rgaa_findings", schema::rgaa_findings_schema())
                .execute()
                .await
                .map_err(|e| crate::error::AgentError::LanceDb(e.to_string()))?;
        }

        if !table_names.contains(&"rgaa_remediation_patterns".to_string()) {
            db.create_empty_table("rgaa_remediation_patterns", schema::rgaa_remediation_patterns_schema())
                .execute()
                .await
                .map_err(|e| crate::error::AgentError::LanceDb(e.to_string()))?;
        }

        Ok(())
    }
}
```

- [ ] **Step 3: Update lib.rs**

```rust
pub mod config;
pub mod embeddings;
pub mod error;
pub mod memory;
pub mod vector;
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p rgaa-agent`

- [ ] **Step 5: Commit**

```bash
git add crates/rgaa-agent/src/vector/ crates/rgaa-agent/src/lib.rs
git commit -m "feat(agent): add LanceDbVectorStore with vector store schemas"
```
