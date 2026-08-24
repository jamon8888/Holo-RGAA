use crate::ToolContext;
use rig_core::tool::PortableTool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Errors that can occur when using the type tool.
#[derive(Debug, thiserror::Error)]
pub enum TypeError {
    #[error("type failed: {0}")]
    Failed(String),
}

/// Arguments for the type tool.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct TypeArgs {
    /// The CSS selector of the input element
    pub selector: String,
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
        "Type text into an input element identified by its CSS selector".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(TypeArgs)).expect("valid schema")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let session = self.ctx.session().lock().await;
        session
            .type_input(&args.selector, &args.text)
            .await
            .map_err(TypeError::Failed)?;
        Ok(TypeOutput { success: true })
    }
}

/// Legacy struct for backward compatibility with existing tests.
pub struct TypeToolLegacy {
    pub ref_id: String,
    pub text: String,
}

impl TypeToolLegacy {
    /// Type text into an element by its CSS selector.
    /// Uses CDP Runtime.evaluate to set value and dispatch events.
    pub async fn execute(&self, session: &crate::BrowserSession) -> Result<String, String> {
        session.type_input(&self.ref_id, &self.text).await?;
        Ok(format!("Typed '{}' into {}", self.text, self.ref_id))
    }
}
