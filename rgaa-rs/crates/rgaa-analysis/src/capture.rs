//! What a plugin receives from the page (P-00 §4).
//!
//! The P-00 crate stays free of CDP and `image` dependencies: these are
//! deliberately small, serialisable stand-ins that the orchestrator adapts
//! from its own captured types. No plugin ever fetches from the network
//! itself — [`ResourceFetcher`] has already done that, under the audit's
//! cookies and origin (R-P3).

use crate::error::Result;
use async_trait::async_trait;

/// Minimal calculated-style DOM snapshot (CDP `DOMSnapshot.captureSnapshot`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DomSnapshot {
    /// Serialised snapshot, including computed styles.
    pub json: String,
}

impl DomSnapshot {
    pub fn new(json: impl Into<String>) -> Self {
        Self { json: json.into() }
    }
}

/// One node of the accessibility tree (CDP `Accessibility.getFullAXTree`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AxNode {
    pub role: String,
    pub name: String,
}

/// Minimal accessibility tree.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AxTree {
    pub nodes: Vec<AxNode>,
}

/// Full-page RGBA image, at the device pixel ratio given in [`PageCapture`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    /// `width * height * 4` bytes in RGBA order.
    pub pixels: Vec<u8>,
}

/// Downloads a page resource (image, media) before a plugin is called.
#[async_trait]
pub trait ResourceFetcher: Send + Sync {
    /// Returns the resource bytes, or an error naming the resource.
    async fn fetch(&self, url: &str) -> Result<Vec<u8>>;
}

/// Used by the P-00 convenience runner, which has no resource context yet.
pub struct NoopResourceFetcher;

#[async_trait]
impl ResourceFetcher for NoopResourceFetcher {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>> {
        Err(crate::error::analysis_error(
            "resources",
            format!("no resource fetcher configured for {url}"),
        ))
    }
}

/// Everything a plugin may look at: what has already been captured, nothing more.
pub struct PageCapture<'a> {
    pub url: &'a str,
    pub dom_snapshot: &'a DomSnapshot,
    pub ax_tree: Option<&'a AxTree>,
    pub screenshot: Option<&'a RgbaImage>,
    pub dpr: f32,
    pub resources: &'a dyn ResourceFetcher,
    pub lang_declared: Option<&'a str>,
}
