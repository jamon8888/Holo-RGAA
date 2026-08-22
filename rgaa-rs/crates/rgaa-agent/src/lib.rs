//! RGAA Agentic Evaluator
//!
//! This crate provides a dual-model agentic evaluator for RGAA IA-assistée
//! criteria. It combines:
//! - A fast tactical model (Holo3 35b) for routine checks
//! - A larger reasoning model (Holo3 122b) for complex criteria
//! - Token-bucket rate limiting per model tier
//! - LanceDB-backed conversation memory and vector retrieval
//! - Structured prompts enriched with criterion definitions and WCAG references
//! - Confidence-based `NeedsReview` escalation for uncertain verdicts

pub mod agent;
pub mod config;
pub mod criteria_defs;
pub mod embeddings;
pub mod error;
pub mod memory;
pub mod vector;
pub mod models;
pub mod prompts;
pub mod ratelimit;
pub mod remediate;
pub mod verify;

pub use agent::RgaaAgent;
pub use config::AgentConfig;
pub use embeddings::HybridEmbeddingProvider;
pub use error::AgentError;
pub use memory::LanceDbMemory;
pub use vector::LanceDbVectorStore;