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
