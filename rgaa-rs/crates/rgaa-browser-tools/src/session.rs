use crate::ax_tree::AXTree;
use rgaa_obscura::ObscuraBridge;
use std::sync::Arc;
use tokio::sync::Mutex;

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
///
/// Wraps `BrowserSession` in `Arc<Mutex<>>` so multiple rig tools can access
/// the session concurrently during an agent's tool-use loop.
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
