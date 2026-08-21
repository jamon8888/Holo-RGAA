pub struct EvalJsTool {
    pub snippet: String,
}

impl EvalJsTool {
    /// Execute JavaScript via CDP Runtime.evaluate.
    /// Returns the string result of the expression.
    pub async fn execute(&self, _session: &crate::BrowserSession) -> Result<String, String> {
        Err("eval_js not yet connected to CDP".to_string())
    }
}
