use crate::ToolContext;
use rig_core::tool::PortableTool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Errors that can occur when using the type tool.
#[derive(Debug, thiserror::Error)]
pub enum TypeError {
    #[error("type_input not yet connected to CDP")]
    NotConnected,
}

/// Arguments for the type tool.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct TypeArgs {
    /// The accessibility tree reference ID of the input element
    pub ref_id: String,
    /// The text to type into the element
    pub text: String,
}

/// Output from the type tool.
#[derive(Debug, Serialize, Deserialize)]
pub struct TypeOutput {
    pub success: bool,
}

/// Tool that types text into an input element.
pub struct TypeTool {
    ctx: ToolContext,
}

impl TypeTool {
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

impl PortableTool for TypeTool {
    const NAME: &str = "type_input";
    type Error = TypeError;
    type Args = TypeArgs;
    type Output = TypeOutput;

    fn description(&self) -> String {
        "Type text into an input element identified by its accessibility reference".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(TypeArgs)).expect("valid schema")
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let _session = self.ctx.session().lock().await;
        // TODO: CDP Input.dispatchKeyEvent in Task 6
        Err(TypeError::NotConnected)
    }
}

/// Legacy struct for backward compatibility with existing tests.
pub struct TypeToolLegacy {
    pub ref_id: String,
    pub text: String,
}

impl TypeToolLegacy {
    /// Type text into an element by its a11y tree ref.
    /// Uses CDP DOM.focus + Input.dispatchKeyEvent.
    pub async fn execute(&self, _session: &crate::BrowserSession) -> Result<String, String> {
        Err("type not yet connected to CDP".to_string())
    }
}
