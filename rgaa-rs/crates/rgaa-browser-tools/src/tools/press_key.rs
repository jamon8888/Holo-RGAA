use crate::ToolContext;
use rig_core::tool::PortableTool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Errors that can occur when using the press key tool.
#[derive(Debug, thiserror::Error)]
pub enum PressKeyError {
    #[error("press_key failed: {0}")]
    Failed(String),
}

/// Arguments for the press key tool.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct PressKeyArgs {
    /// The key to press (e.g., "Tab", "Enter", "ArrowDown")
    pub key: String,
}

/// Output from the press key tool.
#[derive(Debug, Serialize, Deserialize)]
pub struct PressKeyOutput {
    pub success: bool,
}

/// Tool that presses a keyboard key.
pub struct PressKeyTool {
    ctx: ToolContext,
}

impl PressKeyTool {
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

impl PortableTool for PressKeyTool {
    const NAME: &str = "press_key";
    type Error = PressKeyError;
    type Args = PressKeyArgs;
    type Output = PressKeyOutput;

    fn description(&self) -> String {
        "Press a keyboard key and return the newly focused element".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(PressKeyArgs)).expect("valid schema")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let session = self.ctx.session().lock().await;
        session
            .press_key(&args.key)
            .await
            .map_err(PressKeyError::Failed)?;
        Ok(PressKeyOutput {
            success: true,
        })
    }
}

/// Legacy struct for backward compatibility with existing tests.
pub struct PressKeyToolLegacy {
    pub key: String,
}

impl PressKeyToolLegacy {
    /// Press a keyboard key via CDP Input.dispatchKeyEvent.
    /// Supports: Tab, Enter, Escape, ArrowUp, ArrowDown, etc.
    pub async fn execute(&self, session: &crate::BrowserSession) -> Result<String, String> {
        session.press_key(&self.key).await?;
        Ok(format!("Pressed key: {}", self.key))
    }
}
