use std::sync::Arc;

use tokio::sync::Mutex;

use crate::BrowserSession;

pub struct BrowserMcpServer {
    session: Arc<Mutex<BrowserSession>>,
}

impl BrowserMcpServer {
    pub fn new_placeholder() -> Self {
        Self {
            session: Arc::new(Mutex::new(BrowserSession::new_placeholder())),
        }
    }

    pub fn tool_names(&self) -> Vec<String> {
        vec![
            "screenshot".into(),
            "navigate".into(),
            "accessibility_tree".into(),
            "eval_js".into(),
            "click".into(),
            "type".into(),
            "press_key".into(),
            "tab_order".into(),
            "assert_state".into(),
        ]
    }
}
