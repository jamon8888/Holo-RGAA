use crate::ax_tree::AXTree;
use rgaa_obscura::ObscuraBridge;

pub struct BrowserSession {
    bridge: ObscuraBridge,
    last_a11y: Option<AXTree>,
    current_url: Option<String>,
}

impl BrowserSession {
    pub fn new(bridge: ObscuraBridge) -> Self {
        Self {
            bridge,
            last_a11y: None,
            current_url: None,
        }
    }

    pub fn new_placeholder() -> Self {
        Self {
            bridge: ObscuraBridge::new(),
            last_a11y: None,
            current_url: None,
        }
    }

    pub fn bridge(&self) -> &ObscuraBridge {
        &self.bridge
    }

    pub fn current_url(&self) -> Option<&str> {
        self.current_url.as_deref()
    }

    pub fn set_current_url(&mut self, url: String) {
        self.current_url = Some(url);
    }

    pub fn last_a11y(&self) -> Option<&AXTree> {
        self.last_a11y.as_ref()
    }

    pub fn set_last_a11y(&mut self, tree: AXTree) {
        self.last_a11y = Some(tree);
    }
}
