use crate::ToolContext;
use rig_core::tool::PortableTool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Errors that can occur when using the eval JS tool.
#[derive(Debug, thiserror::Error)]
pub enum EvalJsError {
    #[error("eval_js evaluation failed: {0}")]
    EvaluationFailed(String),
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
    pub error: Option<String>,
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

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let session = self.ctx.session().lock().await;
        let result = session
            .eval_js(&args.expression)
            .await
            .map_err(EvalJsError::EvaluationFailed)?;

        // Extract the result value from CDP response
        let value = result
            .get("result")
            .and_then(|r| r.get("value"))
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        let error = result
            .get("exceptionDetails")
            .map(|e| format!("{e}"));

        Ok(EvalJsOutput {
            result: value,
            error,
        })
    }
}

/// Legacy struct for backward compatibility with existing tests.
pub struct EvalJsToolLegacy {
    pub snippet: String,
}

impl EvalJsToolLegacy {
    /// Execute JavaScript via CDP Runtime.evaluate.
    /// Returns the string result of the expression.
    pub async fn execute(&self, session: &crate::BrowserSession) -> Result<String, String> {
        let result = session.eval_js(&self.snippet).await?;
        let value = result
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        Ok(value)
    }
}
