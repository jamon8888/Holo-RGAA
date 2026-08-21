use crate::ToolContext;
use rig_core::tool::PortableTool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Errors that can occur when using the click tool.
#[derive(Debug, thiserror::Error)]
pub enum ClickError {
    #[error("click not yet connected to CDP")]
    NotConnected,
}

/// Arguments for the click tool.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ClickArgs {
    /// The accessibility tree backend node ID of the element to click
    pub ref_id: String,
}

/// Output from the click tool.
#[derive(Debug, Serialize, Deserialize)]
pub struct ClickOutput {
    pub success: bool,
    pub focused_element: Option<String>,
}

/// Tool that clicks an element by its accessibility tree reference ID.
pub struct ClickTool {
    ctx: ToolContext,
}

impl ClickTool {
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

impl PortableTool for ClickTool {
    const NAME: &str = "click";
    type Error = ClickError;
    type Args = ClickArgs;
    type Output = ClickOutput;

    fn description(&self) -> String {
        "Click an element by its accessibility tree reference ID".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(ClickArgs)).expect("valid schema")
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let _session = self.ctx.session().lock().await;
        // TODO: CDP DOM.focus + Input.dispatchMouseEvent in Task 6
        Err(ClickError::NotConnected)
    }
}

/// Legacy struct for backward compatibility with existing tests.
pub struct ClickToolLegacy {
    pub ref_id: String,
}

impl ClickToolLegacy {
    /// Click an element by its a11y tree backendNodeId ref.
    /// Uses CDP DOM.focus + Input.dispatchMouseEvent.
    pub async fn execute(&self, _session: &crate::BrowserSession) -> Result<String, String> {
        Err("click not yet connected to CDP".to_string())
    }
}
