pub struct ScreenshotTool;

impl ScreenshotTool {
    /// Capture a screenshot of the current page via CDP Page.captureScreenshot.
    /// Returns base64-encoded PNG.
    pub async fn execute(&self, _session: &crate::BrowserSession) -> Result<String, String> {
        Err("screenshot not yet connected to CDP".to_string())
    }
}
