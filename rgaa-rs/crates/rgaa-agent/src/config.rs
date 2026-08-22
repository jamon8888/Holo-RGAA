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