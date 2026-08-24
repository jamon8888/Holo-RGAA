use crate::ToolContext;
use rig_core::tool::PortableTool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Errors that can occur when using the assert state tool.
#[derive(Debug, thiserror::Error)]
pub enum AssertStateError {
    #[error("assert_state failed: {0}")]
    Failed(String),
}

/// Arguments for the assert state tool.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AssertStateArgs {
    /// A JavaScript expression that returns a boolean or value
    /// (e.g., "document.querySelector('.dialog') !== null")
    pub predicate: String,
}

/// Output from the assert state tool.
#[derive(Debug, Serialize, Deserialize)]
pub struct AssertStateOutput {
    pub satisfied: bool,
    pub details: String,
}

/// Tool that asserts a specific browser state predicate.
pub struct AssertStateTool {
    ctx: ToolContext,
}

impl AssertStateTool {
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

impl PortableTool for AssertStateTool {
    const NAME: &str = "assert_state";
    type Error = AssertStateError;
    type Args = AssertStateArgs;
    type Output = AssertStateOutput;

    fn description(&self) -> String {
        "Assert a specific browser state by evaluating a JavaScript predicate".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(AssertStateArgs)).expect("valid schema")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let session = self.ctx.session().lock().await;
        let result = session
            .assert_state(&args.predicate)
            .await
            .map_err(AssertStateError::Failed)?;
        let satisfied = result.as_bool().unwrap_or(false);
        Ok(AssertStateOutput {
            satisfied,
            details: result.to_string(),
        })
    }
}

/// Legacy struct for backward compatibility with existing tests.
pub struct AssertStateToolLegacy {
    pub predicate: String,
}

impl AssertStateToolLegacy {
    /// Evaluate a JavaScript predicate in-page and return its boolean result.
    /// Used by the act→verify loop to confirm state changes after actions.
    pub async fn execute(&self, session: &crate::BrowserSession) -> Result<bool, String> {
        let result = session.assert_state(&self.predicate).await?;
        Ok(result.as_bool().unwrap_or(false))
    }
}
