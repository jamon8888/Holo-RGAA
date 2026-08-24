use crate::ToolContext;
use rig_core::tool::PortableTool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Errors that can occur when using the screenshot tool.
#[derive(Debug, thiserror::Error)]
pub enum ScreenshotError {
    #[error("screenshot not yet connected to CDP")]
    NotConnected,
    #[error("screenshot capture failed: {0}")]
    CaptureFailed(String),
}

/// Arguments for the screenshot tool (no parameters needed).
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ScreenshotArgs {}

/// Output from the screenshot tool.
#[derive(Debug, Serialize, Deserialize)]
pub struct ScreenshotOutput {
    pub data_base64: String,
    pub width: u32,
    pub height: u32,
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
        let session = self.ctx.session().lock().await;
        let data_base64 = session
            .screenshot()
            .await
            .map_err(ScreenshotError::CaptureFailed)?;

        // TODO: Get actual dimensions from CDP response
        // For now, return placeholder dimensions
        Ok(ScreenshotOutput {
            data_base64,
            width: 1920,
            height: 1080,
        })
    }
}

/// Legacy unit-struct tool for backward compatibility with existing tests.
pub struct ScreenshotLegacy;

impl ScreenshotLegacy {
    /// Capture a screenshot of the current page via CDP Page.captureScreenshot.
    /// Returns base64-encoded PNG.
    pub async fn execute(&self, session: &crate::BrowserSession) -> Result<String, String> {
        session.screenshot().await
    }
}
