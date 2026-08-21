use crate::ToolContext;
use rig_core::tool::PortableTool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Errors that can occur when using the a11y tree tool.
#[derive(Debug, thiserror::Error)]
pub enum A11yTreeError {
    #[error("a11y tree not yet connected to CDP")]
    NotConnected,
}

/// Arguments for the a11y tree tool (no parameters needed).
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct A11yTreeArgs {}

/// Output from the a11y tree tool.
#[derive(Debug, Serialize, Deserialize)]
pub struct A11yTreeOutput {
    pub tree_json: serde_json::Value,
}

/// Tool that retrieves the full accessibility tree of the current page.
pub struct A11yTreeTool {
    ctx: ToolContext,
}

impl A11yTreeTool {
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

impl PortableTool for A11yTreeTool {
    const NAME: &str = "a11y_tree";
    type Error = A11yTreeError;
    type Args = A11yTreeArgs;
    type Output = A11yTreeOutput;

    fn description(&self) -> String {
        "Get the full accessibility tree of the current page".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(A11yTreeArgs)).expect("valid schema")
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let _session = self.ctx.session().lock().await;
        // TODO: CDP Accessibility.getFullAXTree in Task 6
        Err(A11yTreeError::NotConnected)
    }
}

/// Legacy unit-struct tool for backward compatibility with existing tests.
pub struct AccessibilityTreeLegacy;

impl AccessibilityTreeLegacy {
    /// Fetch the accessibility tree via CDP Accessibility.getFullAXTree.
    /// Returns a structured AXTree with stable backendNodeIds.
    pub async fn execute(
        &self,
        _session: &mut crate::BrowserSession,
    ) -> Result<crate::AXTree, String> {
        Err("a11y tree not yet connected to CDP".to_string())
    }
}
