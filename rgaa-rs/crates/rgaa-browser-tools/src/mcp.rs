use std::sync::Arc;

use tokio::sync::Mutex;

use crate::tools::A11yTreeTool;
use crate::BrowserSession;
use rig_core::tool::PortableTool;

pub struct BrowserMcpServer {
    #[allow(dead_code)]
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
            A11yTreeTool::NAME.into(),
            "eval_js".into(),
            "click".into(),
            "type".into(),
            "press_key".into(),
            "tab_order".into(),
            "assert_state".into(),
        ]
    }
}
