pub struct TypeTool {
    pub ref_id: String,
    pub text: String,
}

impl TypeTool {
    /// Type text into an element by its a11y tree ref.
    /// Uses CDP DOM.focus + Input.dispatchKeyEvent.
    pub async fn execute(&self, _session: &crate::BrowserSession) -> Result<String, String> {
        Err("type not yet connected to CDP".to_string())
    }
}
