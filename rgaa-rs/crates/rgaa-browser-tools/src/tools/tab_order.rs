use crate::ToolContext;
use rig_core::tool::PortableTool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Errors that can occur when using the tab order tool.
#[derive(Debug, thiserror::Error)]
pub enum TabOrderError {
    #[error("tab_order not yet connected to CDP")]
    NotConnected,
}

/// Arguments for the tab order tool (no parameters needed).
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct TabOrderArgs {}

/// Output from the tab order tool.
#[derive(Debug, Serialize, Deserialize)]
pub struct TabOrderOutput {
    pub elements: Vec<TabStop>,
}

/// A single tab stop in the tab order.
#[derive(Debug, Serialize, Deserialize)]
pub struct TabStop {
    pub index: usize,
    pub ref_id: String,
    pub role: String,
    pub name: String,
}

/// Tool that returns the tab order of focusable elements.
pub struct TabOrderTool {
    ctx: ToolContext,
}

impl TabOrderTool {
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

impl PortableTool for TabOrderTool {
    const NAME: &str = "tab_order";
    type Error = TabOrderError;
    type Args = TabOrderArgs;
    type Output = TabOrderOutput;

    fn description(&self) -> String {
        "Get the tab order of focusable elements on the current page".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(TabOrderArgs)).expect("valid schema")
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let _session = self.ctx.session().lock().await;
        // TODO: Derived from a11y tree in Task 6
        Err(TabOrderError::NotConnected)
    }
}

/// Legacy struct for backward compatibility with existing tests.
pub struct TabOrderToolLegacy;

impl TabOrderToolLegacy {
    /// Return the ordered list of focusable elements from the a11y tree.
    /// Each element has a stable backendNodeId for click/type operations.
    pub async fn execute(
        &self,
        session: &crate::BrowserSession,
    ) -> Result<Vec<crate::AXNode>, String> {
        let tree = session
            .last_a11y()
            .ok_or("no a11y tree available — call AccessibilityTreeTool first")?;
        Ok(tree.focusable_elements().into_iter().cloned().collect())
    }
}
