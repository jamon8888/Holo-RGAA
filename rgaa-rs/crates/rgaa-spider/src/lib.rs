//! Spider crawler integration for RGAA.
//!
//! Provides a Rig `PortableTool` that crawls a website and returns discovered
//! pages as `PageSummary` structs for LLM-based page selection.

pub mod error;

pub use error::SpiderError;
