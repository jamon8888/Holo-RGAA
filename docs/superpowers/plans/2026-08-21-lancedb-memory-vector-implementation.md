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
