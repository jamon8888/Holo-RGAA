use crate::ToolContext;
use rig_core::tool::PortableTool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Errors that can occur when using the assert state tool.
#[derive(Debug, thiserror::Error)]
pub enum AssertStateError {
    #[error("assert_state not yet connected to CDP")]
    NotConnected,
}

/// Arguments for the assert state tool.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AssertStateArgs {
    /// A predicate describing the expected state
    /// (e.g., "dialog-visible", "element-focused:#submit")
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
        "Assert a specific browser state predicate (e.g., 'dialog-visible', 'element-focused:#submit')"
            .to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(AssertStateArgs)).expect("valid schema")
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let _session = self.ctx.session().lock().await;
        // TODO: Varies per predicate in Task 6
        Err(AssertStateError::NotConnected)
    }
}

/// Legacy struct for backward compatibility with existing tests.
pub struct AssertStateToolLegacy {
    pub predicate: String,
}

impl AssertStateToolLegacy {
    /// Evaluate a JavaScript predicate in-page and return its boolean result.
    /// Used by the act→verify loop to confirm state changes after actions.
    pub async fn execute(&self, _session: &crate::BrowserSession) -> Result<bool, String> {
        Err("assert_state not yet connected to CDP".to_string())
    }
}
