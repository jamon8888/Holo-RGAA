use crate::ToolContext;
use rig_core::tool::PortableTool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Errors that can occur when using the click tool.
#[derive(Debug, thiserror::Error)]
pub enum ClickError {
    #[error("click not yet connected to CDP")]
    NotConnected,
    #[error("click failed: {0}")]
    ClickFailed(String),
}

/// Arguments for the click tool.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ClickArgs {
    /// The CSS selector of the element to click
    pub selector: String,
}

/// Output from the click tool.
#[derive(Debug, Serialize, Deserialize)]
pub struct ClickOutput {
    pub success: bool,
    pub message: String,
}

/// Tool that clicks an element by its CSS selector.
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
        "Click an element by its CSS selector".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(ClickArgs)).expect("valid schema")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let session = self.ctx.session().lock().await;
        session
            .click_element(&args.selector)
            .await
            .map_err(ClickError::ClickFailed)?;
        Ok(ClickOutput {
            success: true,
            message: format!("Clicked element: {}", args.selector),
        })
    }
}

/// Legacy struct for backward compatibility with existing tests.
pub struct ClickToolLegacy {
    /// CSS selector for the element to click.
    pub selector: String,
}

impl ClickToolLegacy {
    /// Click an element by its CSS selector.
    /// Uses CDP Runtime.evaluate to click by selector.
    pub async fn execute(&self, session: &crate::BrowserSession) -> Result<String, String> {
        session.click_element(&self.selector).await?;
        Ok(format!("Clicked element: {}", self.selector))
    }
}
