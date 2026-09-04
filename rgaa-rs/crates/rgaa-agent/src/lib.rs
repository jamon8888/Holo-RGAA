//! RGAA Agentic Evaluator
//!
//! This crate provides an agentic evaluator for RGAA IA-assistée criteria.
//! It combines:
//! - A single Holo3 model for criterion evaluation (configurable via `AgentConfig::model`)
//! - Token-bucket rate limiting to protect the API
//! - LanceDB-backed conversation memory and vector retrieval (optional, feature "vector-store")
//! - Structured prompts enriched with criterion definitions and WCAG references
//! - Confidence-based `NeedsReview` escalation for uncertain verdicts

pub mod agent;
pub mod config;
pub mod criteria_defs;
pub mod error;
pub mod prompts;
pub mod ratelimit;
pub mod remediate;
pub mod verify;

// Optional: LanceDB vector storage
#[cfg(feature = "vector-store")]
pub mod embeddings;
#[cfg(feature = "vector-store")]
pub mod memory;
#[cfg(feature = "vector-store")]
pub mod vector;

pub use agent::RgaaAgent;
pub use config::AgentConfig;
pub use error::AgentError;

#[cfg(feature = "vector-store")]
pub use embeddings::HybridEmbeddingProvider;
#[cfg(feature = "vector-store")]
pub use memory::LanceDbMemory;
#[cfg(feature = "vector-store")]
pub use vector::LanceDbVectorStore;
