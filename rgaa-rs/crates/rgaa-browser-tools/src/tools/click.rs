pub struct ClickTool {
    pub ref_id: String,
}

impl ClickTool {
    /// Click an element by its a11y tree backendNodeId ref.
    /// Uses CDP DOM.focus + Input.dispatchMouseEvent.
    pub async fn execute(&self, _session: &crate::BrowserSession) -> Result<String, String> {
        Err("click not yet connected to CDP".to_string())
    }
}
