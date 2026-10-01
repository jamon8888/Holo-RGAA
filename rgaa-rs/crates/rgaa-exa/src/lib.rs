//! Exa web-search grounding for RGAA audits.
//!
//! RGAA evaluation is grounded on a frozen regulatory corpus (see
//! `rgaa-agent`'s LanceDB référentiel). This crate covers what that corpus
//! cannot: current external guidance — WCAG techniques, ARIA authoring
//! practices, vendor accessibility notes — retrieved at audit time so a
//! remediation proposal cites a checkable source instead of model recall.
//!
//! # Setup
//!
//! ```bash
//! export EXA_API_KEY="your_api_key_here"
//! ```
//!
//! Binaries in this workspace load `.env` through `dotenvy`, so the key can
//! also live in the repo-root `.env` (gitignored).
//!
//! # Example
//!
//! ```no_run
//! use rgaa_exa::{remediation_guidance, ExaClient};
//! use rgaa_core::RgaaCriteria;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let client = ExaClient::from_env()?;
//! let criterion = RgaaCriteria::all()
//!     .iter()
//!     .find(|c| c.id == "1.3")
//!     .ok_or("unknown criterion")?;
//! let refs = remediation_guidance(&client, criterion, Some("img sans attribut alt")).await?;
//! for r in &refs {
//!     println!("{} — {}", r.title, r.url);
//! }
//! # Ok(())
//! # }
//! ```

pub mod client;
pub mod config;
pub mod error;
pub mod guidance;
pub mod search;

pub use client::ExaClient;
pub use config::ExaConfig;
pub use error::{ExaError, Result};
pub use guidance::{
    methodology_guidance, remediation_guidance, GuidanceQuery, GuidanceReference,
    REFERENCES_PER_CRITERION,
};
pub use search::{ContentsOptions, SearchRequest, SearchResponse, SearchResult, SearchType};
