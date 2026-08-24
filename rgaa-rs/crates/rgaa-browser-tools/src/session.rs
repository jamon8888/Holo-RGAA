use std::sync::Arc;

use tokio::sync::Mutex;

use crate::ax_tree::AXTree;
use rgaa_obscura::ObscuraBridge;

/// A browser session that maintains state for accessibility evaluation.
///
/// Wraps an `ObscuraBridge` connection and caches the last accessibility tree
/// and current URL for multi-step evaluation workflows.
pub struct BrowserSession {
    bridge: ObscuraBridge,
    last_a11y: Option<AXTree>,
    current_url: Option<String>,
}

impl BrowserSession {
    /// Creates a new `BrowserSession` with the given bridge.
    ///
    /// # Arguments
    ///
    /// * `bridge` - The `ObscuraBridge` connection to the browser.
    #[must_use]
    pub fn new(bridge: ObscuraBridge) -> Self {
        Self {
            bridge,
            last_a11y: None,
            current_url: None,
        }
    }

    /// Creates a placeholder session for testing without a real browser connection.
    #[must_use]
    pub fn new_placeholder() -> Self {
        Self {
            bridge: ObscuraBridge::new(),
            last_a11y: None,
            current_url: None,
        }
    }

    /// Returns a reference to the underlying `ObscuraBridge`.
    pub fn bridge(&self) -> &ObscuraBridge {
        &self.bridge
    }

    /// Returns the current URL if set.
    pub fn current_url(&self) -> Option<&str> {
        self.current_url.as_deref()
    }

    /// Sets the current URL for this session.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to set as current.
    pub fn set_current_url(&mut self, url: String) {
        self.current_url = Some(url);
    }

    /// Navigate the browser to a URL
    pub async fn navigate(&mut self, url: &str) -> Result<(), String> {
        self.bridge.navigate(url).await?;
        self.current_url = Some(url.to_string());
        Ok(())
    }

    /// Evaluate JavaScript in the current page
    pub async fn eval_js(&self, expression: &str) -> Result<serde_json::Value, String> {
        self.bridge.eval_js(expression).await
    }

    /// Click an element by CSS selector
    pub async fn click_element(&self, selector: &str) -> Result<(), String> {
        let url = self.current_url.as_deref().unwrap_or("about:blank");
        self.bridge.click_element(url, selector).await
    }

    /// Take a screenshot of the current page
    pub async fn screenshot(&self) -> Result<String, String> {
        let url = self.current_url.as_deref().unwrap_or("about:blank");
        self.bridge.screenshot(url).await
    }

    /// Get the accessibility tree for the current page
    pub async fn get_a11y_tree(&self) -> Result<serde_json::Value, String> {
        let url = self.current_url.as_deref().unwrap_or("about:blank");
        self.bridge.get_accessibility_tree(url).await
    }

    /// Returns a reference to the last accessibility tree, if available.
    #[must_use]
    pub fn last_a11y(&self) -> Option<&AXTree> {
        self.last_a11y.as_ref()
    }

    /// Stores the given accessibility tree as the last captured tree.
    ///
    /// # Arguments
    ///
    /// * `tree` - The accessibility tree to store.
    pub fn set_last_a11y(&mut self, tree: AXTree) {
        self.last_a11y = Some(tree);
    }
}

/// Shared context passed to all rig tools.
/// Wraps `BrowserSession` in `Arc<Mutex<>>` for concurrent tool access.
#[derive(Clone)]
pub struct ToolContext {
    session: Arc<Mutex<BrowserSession>>,
}

impl ToolContext {
    /// Creates a new `ToolContext` wrapping the given session.
    #[must_use]
    pub fn new(session: BrowserSession) -> Self {
        Self {
            session: Arc::new(Mutex::new(session)),
        }
    }

    /// Returns a reference to the inner mutex-guarded session.
    pub fn session(&self) -> &Arc<Mutex<BrowserSession>> {
        &self.session
    }
}
