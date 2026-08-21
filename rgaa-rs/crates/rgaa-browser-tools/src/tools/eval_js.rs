use crate::ToolContext;
use rig_core::tool::PortableTool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Errors that can occur when using the eval JS tool.
#[derive(Debug, thiserror::Error)]
pub enum EvalJsError {
    #[error("eval_js not yet connected to CDP")]
    NotConnected,
}

/// Arguments for the eval JS tool.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct EvalJsArgs {
    /// The JavaScript expression to evaluate
    pub expression: String,
}

/// Output from the eval JS tool.
#[derive(Debug, Serialize, Deserialize)]
pub struct EvalJsOutput {
    pub result: serde_json::Value,
}

/// Tool that evaluates JavaScript in the browser context.
pub struct EvalJsTool {
    ctx: ToolContext,
}

impl EvalJsTool {
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

impl PortableTool for EvalJsTool {
    const NAME: &str = "eval_js";
    type Error = EvalJsError;
    type Args = EvalJsArgs;
    type Output = EvalJsOutput;

    fn description(&self) -> String {
        "Evaluate a JavaScript expression in the browser context".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(EvalJsArgs)).expect("valid schema")
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let _session = self.ctx.session().lock().await;
        // TODO: CDP Runtime.evaluate in Task 6
        Err(EvalJsError::NotConnected)
    }
}

/// Legacy struct for backward compatibility with existing tests.
pub struct EvalJsToolLegacy {
    pub snippet: String,
}

impl EvalJsToolLegacy {
    /// Execute JavaScript via CDP Runtime.evaluate.
    /// Returns the string result of the expression.
    pub async fn execute(&self, _session: &crate::BrowserSession) -> Result<String, String> {
        Err("eval_js not yet connected to CDP".to_string())
    }
}
