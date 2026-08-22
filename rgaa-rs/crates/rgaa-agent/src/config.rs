use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Runtime configuration for the RGAA agentic evaluator.
///
/// Construct via [`AgentConfig::default`] for local runs or
/// [`AgentConfig::from_env`] to read credentials from the environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Base URL of the Holo3 evaluation API.
    pub holo3_base_url: String,
    /// API key for the Holo3 evaluation API.
    pub api_key: String,
    /// Reasoning model identifier (e.g. `holo3-1-35b-a3b`).
    pub model: String,
    /// Filesystem path used by LanceDB for memory and vector storage.
    pub lancedb_path: String,
    /// Embedding backend used for memory and vector retrieval.
    pub embedding_backend: EmbeddingBackendConfig,
    /// Expected embedding dimensionality. Must match the chosen model.
    pub embedding_dimensions: usize,
    /// Conversation-memory retention policy.
    pub memory_retention: MemoryRetention,
    /// Maximum agentic turns per criterion evaluation.
    pub max_turns: usize,
    /// Maximum tokens per model completion.
    pub max_tokens: usize,
    /// Sampling temperature for the reasoning model.
    pub temperature: f32,
}

/// Embedding backend selection.
///
/// Only [`EmbeddingBackendConfig::FastEmbed`] is implemented; bring your own
/// backend by extending [`crate::embeddings`] and adding a variant here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmbeddingBackendConfig {
    /// On-device embeddings via `fastembed` (`all-MiniLM-L6-v2` by default).
    FastEmbed {
        /// Model name understood by `fastembed`.
        model_name: String,
    },
}

/// Conversation-memory retention policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryRetention {
    /// Messages live only for the duration of a single audit.
    PerAudit,
    /// Messages persist across audits.
    Persistent,
    /// Short-term messages expire after a TTL; matching long-term patterns persist.
    Hybrid {
        /// Time-to-live for short-term messages.
        short_term_ttl: Duration,
        /// Glob pattern selecting long-term message keys.
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
    /// Builds configuration from environment variables.
    ///
    /// # Environment Variables
    /// - `HOLO3_API_KEY` (required): API key for the Holo3 evaluation API.
    /// - `HOLO3_BASE_URL` (optional): Base URL for the Holo3 API. Defaults to
    ///   `https://api.hcompany.ai/v1`.
    /// - `HOLO3_MODEL` (optional): Model identifier. Defaults to
    ///   `holo3-1-35b-a3b`.
    /// - `LANCEDB_PATH` (optional): LanceDB storage path. Defaults to
    ///   `./data/lancedb`.
    ///
    /// # Errors
    /// Returns [`crate::error::AgentError::Config`] if `HOLO3_API_KEY` is not set.
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