use crate::ToolContext;
use rig_core::tool::PortableTool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Errors that can occur when using the navigate tool.
#[derive(Debug, thiserror::Error)]
pub enum NavigateError {
    #[error("navigation failed: {0}")]
    NavigationFailed(String),
}

/// Arguments for the navigate tool.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct NavigateArgs {
    /// The URL to navigate the browser to
    pub url: String,
}

/// Output from the navigate tool.
#[derive(Debug, Serialize, Deserialize)]
pub struct NavigateOutput {
    pub success: bool,
    pub message: String,
}

/// Tool that navigates the browser to a URL.
pub struct NavigateTool {
    ctx: ToolContext,
}

impl NavigateTool {
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

impl PortableTool for NavigateTool {
    const NAME: &str = "navigate";
    type Error = NavigateError;
    type Args = NavigateArgs;
    type Output = NavigateOutput;

    fn description(&self) -> String {
        "Navigate the browser to a URL and return success status".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(NavigateArgs)).expect("valid schema")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // The session is cloned because the lock is synchronous and cannot be
        // held across the await below.
        let mut session = self.ctx.session().lock().clone();
        session
            .navigate(&args.url)
            .await
            .map_err(NavigateError::NavigationFailed)?;
        // `navigate` set `current_url` on the clone, which is dropped here.
        // Writing it back is what every later tool reads: without this, click,
        // screenshot, type_input, press_key, tab_order, assert_state and
        // a11y_tree all fell back to "about:blank".
        self.ctx.session().lock().set_current_url(args.url.clone());
        Ok(NavigateOutput {
            success: true,
            message: format!("Navigated to {}", args.url),
        })
    }
}

// Keep legacy struct for backward compatibility with existing tests
pub struct NavigateLegacy {
    pub url: String,
}

impl NavigateLegacy {
    pub async fn execute(&self, session: &mut crate::BrowserSession) -> Result<String, String> {
        session.set_current_url(self.url.clone());
        Ok(format!("Navigated to {}", self.url))
    }
}
