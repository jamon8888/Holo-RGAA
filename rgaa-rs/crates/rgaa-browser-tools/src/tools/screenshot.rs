use crate::ToolContext;
use rig_core::tool::PortableTool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Errors that can occur when using the screenshot tool.
#[derive(Debug, thiserror::Error)]
pub enum ScreenshotError {
    #[error("screenshot not yet connected to CDP")]
    NotConnected,
}

/// Arguments for the screenshot tool (no parameters needed).
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ScreenshotArgs {}

/// Output from the screenshot tool.
#[derive(Debug, Serialize, Deserialize)]
pub struct ScreenshotOutput {
    pub base64_png: String,
}

/// Tool that captures a screenshot of the current page.
pub struct ScreenshotTool {
    ctx: ToolContext,
}

impl ScreenshotTool {
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

impl PortableTool for ScreenshotTool {
    const NAME: &str = "screenshot";
    type Error = ScreenshotError;
    type Args = ScreenshotArgs;
    type Output = ScreenshotOutput;

    fn description(&self) -> String {
        "Capture a screenshot of the current page. Returns base64-encoded PNG.".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(ScreenshotArgs)).expect("valid schema")
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let _session = self.ctx.session().lock().await;
        // TODO: CDP Page.captureScreenshot in Task 6
        Err(ScreenshotError::NotConnected)
    }
}

/// Legacy unit-struct tool for backward compatibility with existing tests.
pub struct ScreenshotLegacy;

impl ScreenshotLegacy {
    /// Capture a screenshot of the current page via CDP Page.captureScreenshot.
    /// Returns base64-encoded PNG.
    pub async fn execute(&self, _session: &crate::BrowserSession) -> Result<String, String> {
        Err("screenshot not yet connected to CDP".to_string())
    }
}
