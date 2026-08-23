//! RGAA Agentic Evaluator
//!
//! This crate provides an agentic evaluator for RGAA IA-assistée criteria.
//! It combines:
//! - A single Holo3 model for criterion evaluation (configurable via `AgentConfig::model`)
//! - Token-bucket rate limiting to protect the API
//! - LanceDB-backed conversation memory and vector retrieval
//! - Structured prompts enriched with criterion definitions and WCAG references
//! - Confidence-based `NeedsReview` escalation for uncertain verdicts
//!
//! The evaluator uses a single model for all criteria. Dual-model routing
//! (tactical/reasoning) is planned for a future release.

pub mod agent;
pub mod config;
pub mod criteria_defs;
pub mod embeddings;
pub mod error;
pub mod memory;
pub mod models;
pub mod prompts;
pub mod ratelimit;
pub mod remediate;
pub mod vector;
pub mod verify;

pub use agent::RgaaAgent;
pub use config::AgentConfig;
pub use embeddings::HybridEmbeddingProvider;
pub use error::AgentError;
pub use memory::LanceDbMemory;
pub use vector::LanceDbVectorStore;
