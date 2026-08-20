pub struct NavigateTool {
    pub url: String,
}

impl NavigateTool {
    pub async fn execute(&self, session: &mut crate::BrowserSession) -> Result<String, String> {
        session.set_current_url(self.url.clone());
        Ok(format!("Navigated to {}", self.url))
    }
}
