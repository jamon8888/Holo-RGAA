//! Spider crawler integration for RGAA.
//!
//! Provides a Rig `PortableTool` that crawls a website and returns discovered
//! pages as `PageSummary` structs for LLM-based page selection.

pub mod error;
pub mod spider_tool;
pub mod types;

pub use error::SpiderError;
pub use spider_tool::SpiderTool;
pub use types::{CrawlSiteArgs, CrawlSiteOutput, CrawlStats, PageSummary};
