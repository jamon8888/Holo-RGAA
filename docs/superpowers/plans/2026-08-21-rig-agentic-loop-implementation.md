# Rig Agentic Loop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire rig-core as the agent runtime so the model drives the browser via tools, evaluates IA_ASSISTE criteria in a multi-turn loop, and proposes remediation fixes for failures.

**Architecture:** Use rig's `AgentBuilder` for agent construction, implement browser tools as rig `Tool` traits with `Arc<Mutex<BrowserSession>>` shared state, keep existing `ModelRouter` and `RateLimiter` as external orchestration. Add `RemediateTool` wired to `rgaa-remediation`'s `FrameworkAdapter`.

**Tech Stack:** Rust 1.80+, rig-core 0.42, rmcp 3.1.3, tokio, serde, schemars 1.x, rgaa-obscura (CDP), rgaa-core (domain types), rgaa-remediation (proposals)

## Global Constraints

- Rust edition 2021, MSRV 1.80
- All crates in workspace: `rgaa-rs/Cargo.toml`
- rig-core already a workspace dependency (version 0.42)
- rmcp already a workspace dependency (version 3.1.3)
- `cargo clippy --workspace --all-targets` must pass
- `cargo fmt --check` must pass
- All prompts in French for RGAA domain terms
- Follow existing patterns: unit structs, thiserror, structured tracing
- No `.unwrap()` in production code — use `?`, `expect()` for invariants, or handle errors
- Prefer `&T` borrowing over `.clone()`

---

## File Structure

| File | Responsibility |
|------|----------------|
| `rgaa-rs/crates/rgaa-browser-tools/Cargo.toml` | Add rig-core, schemars deps |
| `rgaa-browser-tools/src/tools/mod.rs` | Re-export all rig tool types |
| `rgaa-browser-tools/src/tools/navigate.rs` | NavigateTool rig implementation |
| `rgaa-browser-tools/src/tools/screenshot.rs` | ScreenshotTool rig implementation |
| `rgaa-browser-tools/src/tools/a11y_tree.rs` | A11yTreeTool rig implementation |
| `rgaa-browser-tools/src/tools/click.rs` | ClickTool rig implementation |
| `rgaa-browser-tools/src/tools/type_input.rs` | TypeTool rig implementation |
| `rgaa-browser-tools/src/tools/press_key.rs` | PressKeyTool rig implementation |
| `rgaa-browser-tools/src/tools/tab_order.rs` | TabOrderTool rig implementation |
| `rgaa-browser-tools/src/tools/eval_js.rs` | EvalJsTool rig implementation |
| `rgaa-browser-tools/src/tools/assert_state.rs` | AssertStateTool rig implementation |
| `rgaa-browser-tools/src/session.rs` | BrowserSession — add tool context |
| `rgaa-browser-tools/src/lib.rs` | Re-export ToolContext |
| `rgaa-agent/src/agent.rs` | RgaaAgent — rig AgentBuilder integration |
| `rgaa-agent/src/models.rs` | ModelRouter — select rig agent per tier |
| `rgaa-agent/src/remediate.rs` | RemediateTool rig implementation |
| `rgaa-orchestrator/src/pipeline.rs` | Wire rig agents + ToolContext |
| `rgaa-agent/tests/agent_test.rs` | Agent unit tests |
| `rgaa-browser-tools/tests/tools_test.rs` | Tool unit tests |

---

## Task 1: Add rig-core dependencies to rgaa-browser-tools

**Files:**
- Modify: `rgaa-rs/crates/rgaa-browser-tools/Cargo.toml`

**Interfaces:**
- Consumes: workspace rig-core 0.42
- Produces: rig-core available in rgaa-browser-tools

- [ ] **Step 1: Read current Cargo.toml**

Read `rgaa-rs/crates/rgaa-browser-tools/Cargo.toml`.

- [ ] **Step 2: Add rig-core with derive feature and schemars**

Add to `[dependencies]`:
```toml
rig-core = { workspace = true, features = ["derive"] }
schemars = "1"
```

Note: rig-core 0.42 re-exports the `Tool` trait at `rig::tool::Tool`. The `derive` feature enables `#[tool_macro]` but we'll implement `Tool` manually for more control.

- [ ] **Step 3: Verify dependency resolves**

Run: `cargo check -p rgaa-browser-tools 2>&1 | tail -5`
Expected: compiles (may have warnings about unused imports, that's fine)

- [ ] **Step 4: Commit**

```bash
git add rgaa-rs/crates/rgaa-browser-tools/Cargo.toml
git commit -m "chore(browser-tools): add rig-core and schemars dependencies"
```

---

## Task 2: Add ToolContext shared state to BrowserSession

**Files:**
- Modify: `rgaa-rs/crates/rgaa-browser-tools/src/session.rs`
- Modify: `rgaa-rs/crates/rgaa-browser-tools/src/lib.rs`

**Interfaces:**
- Consumes: existing `BrowserSession`, `ObscuraBridge`
- Produces: `ToolContext` type alias used by all tools

- [ ] **Step 1: Write test for ToolContext creation**

Add to `rgaa-browser-tools/tests/tools_test.rs`:
```rust
use rgaa_browser_tools::{BrowserSession, ToolContext};

#[test]
fn test_tool_context_creation() {
    let session = BrowserSession::new_placeholder();
    let ctx = ToolContext::new(session);
    assert!(ctx.session().lock().await.current_url().is_none());
}
```

Note: This test needs `#[tokio::test]` since it uses `lock().await`. Use:
```rust
#[tokio::test]
async fn test_tool_context_creation() {
    let session = BrowserSession::new_placeholder();
    let ctx = ToolContext::new(session);
    assert!(ctx.session().lock().await.current_url().is_none());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p rgaa-browser-tools test_tool_context_creation`
Expected: FAIL (ToolContext doesn't exist yet)

- [ ] **Step 3: Implement ToolContext in session.rs**

Add to `rgaa-rs/crates/rgaa-browser-tools/src/session.rs`:
```rust
use std::sync::Arc;
use tokio::sync::Mutex;

/// Shared context passed to all rig tools.
/// Wraps BrowserSession in Arc<Mutex<>> for concurrent tool access.
#[derive(Clone)]
pub struct ToolContext {
    session: Arc<Mutex<BrowserSession>>,
}

impl ToolContext {
    /// Creates a new ToolContext wrapping the given session.
    #[must_use]
    pub fn new(session: BrowserSession) -> Self {
        Self {
            session: Arc::new(Mutex::new(session)),
        }
    }

    /// Returns a reference to the inner mutex-guarded session.
    pub fn session(&self) -> &Arc<Mutex<BrowserSession>> {
        &self.session
    }
}
```

- [ ] **Step 4: Re-export ToolContext from lib.rs**

Add to `rgaa-rs/crates/rgaa-browser-tools/src/lib.rs`:
```rust
pub use session::{BrowserSession, ToolContext};
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p rgaa-browser-tools test_tool_context_creation`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add rgaa-rs/crates/rgaa-browser-tools/src/session.rs rgaa-rs/crates/rgaa-browser-tools/src/lib.rs rgaa-rs/crates/rgaa-browser-tools/tests/tools_test.rs
git commit -m "feat(browser-tools): add ToolContext shared state for rig tools"
```

---

## Task 3: Implement NavigateTool as rig Tool

**Files:**
- Modify: `rgaa-rs/crates/rgaa-browser-tools/src/tools/navigate.rs`
- Test: `rgaa-rs/crates/rgaa-browser-tools/tests/tools_test.rs`

**Interfaces:**
- Consumes: `ToolContext` (from Task 2)
- Produces: `NavigateTool` implementing `rig::tool::Tool`

- [ ] **Step 1: Write test for NavigateTool rig integration**

Add to `rgaa-browser-tools/tests/tools_test.rs`:
```rust
use rgaa_browser_tools::tools::NavigateTool;
use rgaa_browser_tools::ToolContext;
use rig::tool::{Tool, ToolDefinition};

#[tokio::test]
async fn navigate_tool_definition() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = NavigateTool::new(ctx);
    let def = tool.definition("test".to_string()).await;
    assert_eq!(def.name, "navigate");
    assert!(!def.description.is_empty());
}

#[tokio::test]
async fn navigate_tool_calls_successfully() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = NavigateTool::new(ctx);
    let args = NavigateTool::args_from_json(serde_json::json!({"url": "https://example.com"}));
    let result = tool.call(args).await;
    assert!(result.is_ok());
}
```

Note: The exact API depends on rig 0.42's `Tool` trait. We need to check whether `args_from_json` exists or if we need to deserialize manually. If the trait doesn't have `args_from_json`, use:
```rust
let args: NavigateArgs = serde_json::from_value(serde_json::json!({"url": "https://example.com"})).unwrap();
let result = tool.call(args).await;
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p rgaa-browser-tools navigate_tool_definition`
Expected: FAIL (NavigateTool doesn't implement Tool yet)

- [ ] **Step 3: Implement NavigateTool as rig Tool**

Replace `rgaa-rs/crates/rgaa-browser-tools/src/tools/navigate.rs`:
```rust
use crate::ToolContext;
use rig::core::tool::{Tool, ToolDefinition, ToolError};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

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

impl Tool for NavigateTool {
    const NAME: &str = "navigate";
    type Error = ToolError;
    type Args = NavigateArgs;
    type Output = NavigateOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Navigate the browser to a URL and return success status".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(NavigateArgs))
                .expect("valid schema"),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let mut session = self.ctx.session().lock().await;
        session.set_current_url(args.url.clone());
        // TODO: CDP Page.navigate in Task 6
        Ok(NavigateOutput {
            success: true,
            message: format!("Navigated to {}", args.url),
        })
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p rgaa-browser-tools navigate_tool_definition`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add rgaa-rs/crates/rgaa-browser-tools/src/tools/navigate.rs rgaa-rs/crates/rgaa-browser-tools/tests/tools_test.rs
git commit -m "feat(browser-tools): implement NavigateTool as rig Tool"
```

---

## Task 4: Implement ScreenshotTool and A11yTreeTool as rig Tools

**Files:**
- Modify: `rgaa-rs/crates/rgaa-browser-tools/src/tools/screenshot.rs`
- Modify: `rgaa-rs/crates/rgaa-browser-tools/src/tools/a11y_tree.rs`
- Test: `rgaa-rs/crates/rgaa-browser-tools/tests/tools_test.rs`

**Interfaces:**
- Consumes: `ToolContext` (from Task 2)
- Produces: `ScreenshotTool`, `A11yTreeTool` implementing `rig::tool::Tool`

- [ ] **Step 1: Write tests for ScreenshotTool**

Add to `rgaa-browser-tools/tests/tools_test.rs`:
```rust
use rgaa_browser_tools::tools::ScreenshotTool;

#[tokio::test]
async fn screenshot_tool_definition() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = ScreenshotTool::new(ctx);
    let def = tool.definition("test".to_string()).await;
    assert_eq!(def.name, "screenshot");
}

#[tokio::test]
async fn screenshot_tool_returns_error_when_not_connected() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = ScreenshotTool::new(ctx);
    // No CDP connection — should return error or empty result
    let args: serde_json::Value = serde_json::json!({});
    let result = tool.call(serde_json::from_value(args).unwrap()).await;
    // Placeholder returns error since no CDP
    assert!(result.is_err() || result.unwrap().base64_png.is_empty());
}
```

- [ ] **Step 2: Write tests for A11yTreeTool**

Add to `rgaa-browser-tools/tests/tools_test.rs`:
```rust
use rgaa_browser_tools::tools::A11yTreeTool;

#[tokio::test]
async fn a11y_tree_tool_definition() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = A11yTreeTool::new(ctx);
    let def = tool.definition("test".to_string()).await;
    assert_eq!(def.name, "a11y_tree");
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p rgaa-browser-tools screenshot_tool_definition a11y_tree_tool_definition`
Expected: FAIL

- [ ] **Step 4: Implement ScreenshotTool**

Replace `rgaa-rs/crates/rgaa-browser-tools/src/tools/screenshot.rs`:
```rust
use crate::ToolContext;
use rig::core::tool::{Tool, ToolDefinition, ToolError};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ScreenshotArgs {}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScreenshotOutput {
    pub base64_png: String,
}

pub struct ScreenshotTool {
    ctx: ToolContext,
}

impl ScreenshotTool {
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

impl Tool for ScreenshotTool {
    const NAME: &str = "screenshot";
    type Error = ToolError;
    type Args = ScreenshotArgs;
    type Output = ScreenshotOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Capture a screenshot of the current page. Returns base64-encoded PNG.".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(ScreenshotArgs))
                .expect("valid schema"),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let _session = self.ctx.session().lock().await;
        // TODO: CDP Page.captureScreenshot in Task 6
        Err(ToolError::ToolCallError("screenshot not yet connected to CDP".into()))
    }
}
```

- [ ] **Step 5: Implement A11yTreeTool**

Replace `rgaa-rs/crates/rgaa-browser-tools/src/tools/a11y_tree.rs`:
```rust
use crate::ToolContext;
use rig::core::tool::{Tool, ToolDefinition, ToolError};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct A11yTreeArgs {}

#[derive(Debug, Serialize, Deserialize)]
pub struct A11yTreeOutput {
    pub tree_json: serde_json::Value,
}

pub struct A11yTreeTool {
    ctx: ToolContext,
}

impl A11yTreeTool {
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

impl Tool for A11yTreeTool {
    const NAME: &str = "a11y_tree";
    type Error = ToolError;
    type Args = A11yTreeArgs;
    type Output = A11yTreeOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Get the full accessibility tree of the current page".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(A11yTreeArgs))
                .expect("valid schema"),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let _session = self.ctx.session().lock().await;
        // TODO: CDP Accessibility.getFullAXTree in Task 6
        Err(ToolError::ToolCallError("a11y_tree not yet connected to CDP".into()))
    }
}
```

- [ ] **Step 6: Update tools/mod.rs re-exports**

Ensure `rgaa-rs/crates/rgaa-browser-tools/src/tools/mod.rs` exports the new types:
```rust
pub use navigate::{NavigateTool, NavigateArgs};
pub use screenshot::{ScreenshotTool, ScreenshotArgs};
pub use a11y_tree::{A11yTreeTool, A11yTreeArgs};
```

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test -p rgaa-browser-tools screenshot_tool_definition a11y_tree_tool_definition`
Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add rgaa-rs/crates/rgaa-browser-tools/src/tools/screenshot.rs rgaa-rs/crates/rgaa-browser-tools/src/tools/a11y_tree.rs rgaa-rs/crates/rgaa-browser-tools/src/tools/mod.rs rgaa-rs/crates/rgaa-browser-tools/tests/tools_test.rs
git commit -m "feat(browser-tools): implement ScreenshotTool and A11yTreeTool as rig Tools"
```

---

## Task 5: Implement remaining browser tools as rig Tools

**Files:**
- Modify: `rgaa-rs/crates/rgaa-browser-tools/src/tools/click.rs`
- Modify: `rgaa-rs/crates/rgaa-browser-tools/src/tools/type_input.rs`
- Modify: `rgaa-rs/crates/rgaa-browser-tools/src/tools/press_key.rs`
- Modify: `rgaa-rs/crates/rgaa-browser-tools/src/tools/tab_order.rs`
- Modify: `rgaa-rs/crates/rgaa-browser-tools/src/tools/eval_js.rs`
- Modify: `rgaa-rs/crates/rgaa-browser-tools/src/tools/assert_state.rs`
- Modify: `rgaa-rs/crates/rgaa-browser-tools/src/tools/mod.rs`
- Test: `rgaa-rs/crates/rgaa-browser-tools/tests/tools_test.rs`

**Interfaces:**
- Consumes: `ToolContext` (from Task 2)
- Produces: 6 more rig Tool implementations

- [ ] **Step 1: Write tests for all 6 tools**

Add to `rgaa-browser-tools/tests/tools_test.rs`:
```rust
use rgaa_browser_tools::tools::{ClickTool, TypeTool, PressKeyTool, TabOrderTool, EvalJsTool, AssertStateTool};

#[tokio::test]
async fn click_tool_definition() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = ClickTool::new(ctx);
    let def = tool.definition("test".to_string()).await;
    assert_eq!(def.name, "click");
}

#[tokio::test]
async fn press_key_tool_definition() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = PressKeyTool::new(ctx);
    let def = tool.definition("test".to_string()).await;
    assert_eq!(def.name, "press_key");
}

#[tokio::test]
async fn type_tool_definition() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = TypeTool::new(ctx);
    let def = tool.definition("test".to_string()).await;
    assert_eq!(def.name, "type_input");
}

#[tokio::test]
async fn tab_order_tool_definition() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = TabOrderTool::new(ctx);
    let def = tool.definition("test".to_string()).await;
    assert_eq!(def.name, "tab_order");
}

#[tokio::test]
async fn eval_js_tool_definition() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = EvalJsTool::new(ctx);
    let def = tool.definition("test".to_string()).await;
    assert_eq!(def.name, "eval_js");
}

#[tokio::test]
async fn assert_state_tool_definition() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tool = AssertStateTool::new(ctx);
    let def = tool.definition("test".to_string()).await;
    assert_eq!(def.name, "assert_state");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p rgaa-browser-tools -- click_tool_definition press_key_tool_definition type_tool_definition tab_order_tool_definition eval_js_tool_definition assert_state_tool_definition`
Expected: FAIL

- [ ] **Step 3: Implement ClickTool**

Replace `rgaa-rs/crates/rgaa-browser-tools/src/tools/click.rs`:
```rust
use crate::ToolContext;
use rig::core::tool::{Tool, ToolDefinition, ToolError};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ClickArgs {
    /// The accessibility tree backend node ID of the element to click
    pub ref_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClickOutput {
    pub success: bool,
    pub focused_element: Option<String>,
}

pub struct ClickTool {
    ctx: ToolContext,
}

impl ClickTool {
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

impl Tool for ClickTool {
    const NAME: &str = "click";
    type Error = ToolError;
    type Args = ClickArgs;
    type Output = ClickOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Click an element by its accessibility tree reference ID".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(ClickArgs)).expect("valid schema"),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let _session = self.ctx.session().lock().await;
        // TODO: CDP DOM.focus + Input.dispatchMouseEvent in Task 6
        Err(ToolError::ToolCallError(format!("click({}) not yet connected to CDP", args.ref_id)))
    }
}
```

- [ ] **Step 4: Implement PressKeyTool**

Replace `rgaa-rs/crates/rgaa-browser-tools/src/tools/press_key.rs`:
```rust
use crate::ToolContext;
use rig::core::tool::{Tool, ToolDefinition, ToolError};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct PressKeyArgs {
    /// The key to press (e.g., "Tab", "Enter", "ArrowDown")
    pub key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PressKeyOutput {
    pub success: bool,
    pub focused_element: Option<String>,
}

pub struct PressKeyTool {
    ctx: ToolContext,
}

impl PressKeyTool {
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

impl Tool for PressKeyTool {
    const NAME: &str = "press_key";
    type Error = ToolError;
    type Args = PressKeyArgs;
    type Output = PressKeyOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Press a keyboard key and return the newly focused element".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(PressKeyArgs)).expect("valid schema"),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let _session = self.ctx.session().lock().await;
        // TODO: CDP Input.dispatchKeyEvent in Task 6
        Err(ToolError::ToolCallError(format!("press_key({}) not yet connected to CDP", args.key)))
    }
}
```

- [ ] **Step 5: Implement TypeTool**

Replace `rgaa-rs/crates/rgaa-browser-tools/src/tools/type_input.rs`:
```rust
use crate::ToolContext;
use rig::core::tool::{Tool, ToolDefinition, ToolError};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct TypeArgs {
    /// The accessibility tree reference ID of the input element
    pub ref_id: String,
    /// The text to type into the element
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TypeOutput {
    pub success: bool,
}

pub struct TypeTool {
    ctx: ToolContext,
}

impl TypeTool {
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

impl Tool for TypeTool {
    const NAME: &str = "type_input";
    type Error = ToolError;
    type Args = TypeArgs;
    type Output = TypeOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Type text into an input element identified by its accessibility reference".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(TypeArgs)).expect("valid schema"),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let _session = self.ctx.session().lock().await;
        // TODO: CDP Input.dispatchKeyEvent in Task 6
        Err(ToolError::ToolCallError(format!("type_input({}) not yet connected to CDP", args.ref_id)))
    }
}
```

- [ ] **Step 6: Implement TabOrderTool**

Replace `rgaa-rs/crates/rgaa-browser-tools/src/tools/tab_order.rs`:
```rust
use crate::ToolContext;
use rig::core::tool::{Tool, ToolDefinition, ToolError};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct TabOrderArgs {}

#[derive(Debug, Serialize, Deserialize)]
pub struct TabOrderOutput {
    pub elements: Vec<TabStop>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TabStop {
    pub index: usize,
    pub ref_id: String,
    pub role: String,
    pub name: String,
}

pub struct TabOrderTool {
    ctx: ToolContext,
}

impl TabOrderTool {
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

impl Tool for TabOrderTool {
    const NAME: &str = "tab_order";
    type Error = ToolError;
    type Args = TabOrderArgs;
    type Output = TabOrderOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Get the tab order of focusable elements on the current page".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(TabOrderArgs)).expect("valid schema"),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let _session = self.ctx.session().lock().await;
        // TODO: Derived from a11y tree in Task 6
        Err(ToolError::ToolCallError("tab_order not yet connected to CDP".into()))
    }
}
```

- [ ] **Step 7: Implement EvalJsTool**

Replace `rgaa-rs/crates/rgaa-browser-tools/src/tools/eval_js.rs`:
```rust
use crate::ToolContext;
use rig::core::tool::{Tool, ToolDefinition, ToolError};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct EvalJsArgs {
    /// The JavaScript expression to evaluate
    pub expression: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EvalJsOutput {
    pub result: serde_json::Value,
}

pub struct EvalJsTool {
    ctx: ToolContext,
}

impl EvalJsTool {
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

impl Tool for EvalJsTool {
    const NAME: &str = "eval_js";
    type Error = ToolError;
    type Args = EvalJsArgs;
    type Output = EvalJsOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Evaluate a JavaScript expression in the browser context".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(EvalJsArgs)).expect("valid schema"),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let _session = self.ctx.session().lock().await;
        // TODO: CDP Runtime.evaluate in Task 6
        Err(ToolError::ToolCallError(format!("eval_js not yet connected to CDP: {}", args.expression)))
    }
}
```

- [ ] **Step 8: Implement AssertStateTool**

Replace `rgaa-rs/crates/rgaa-browser-tools/src/tools/assert_state.rs`:
```rust
use crate::ToolContext;
use rig::core::tool::{Tool, ToolDefinition, ToolError};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AssertStateArgs {
    /// A predicate describing the expected state (e.g., "dialog-visible", "element-focused:#submit")
    pub predicate: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssertStateOutput {
    pub satisfied: bool,
    pub details: String,
}

pub struct AssertStateTool {
    ctx: ToolContext,
}

impl AssertStateTool {
    pub fn new(ctx: ToolContext) -> Self {
        Self { ctx }
    }
}

impl Tool for AssertStateTool {
    const NAME: &str = "assert_state";
    type Error = ToolError;
    type Args = AssertStateArgs;
    type Output = AssertStateOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Assert a specific browser state predicate (e.g., 'dialog-visible', 'element-focused:#submit')".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(AssertStateArgs)).expect("valid schema"),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let _session = self.ctx.session().lock().await;
        // TODO: Varies per predicate in Task 6
        Err(ToolError::ToolCallError(format!("assert_state not yet connected to CDP: {}", args.predicate)))
    }
}
```

- [ ] **Step 9: Update tools/mod.rs re-exports**

Update `rgaa-rs/crates/rgaa-browser-tools/src/tools/mod.rs`:
```rust
pub mod navigate;
pub mod screenshot;
pub mod a11y_tree;
pub mod eval_js;
pub mod click;
pub mod type_input;
pub mod press_key;
pub mod tab_order;
pub mod assert_state;

pub use navigate::{NavigateTool, NavigateArgs};
pub use screenshot::{ScreenshotTool, ScreenshotArgs};
pub use a11y_tree::{A11yTreeTool, A11yTreeArgs};
pub use eval_js::{EvalJsTool, EvalJsArgs};
pub use click::{ClickTool, ClickArgs};
pub use type_input::{TypeTool, TypeArgs};
pub use press_key::{PressKeyTool, PressKeyArgs};
pub use tab_order::{TabOrderTool, TabOrderArgs, TabStop};
pub use assert_state::{AssertStateTool, AssertStateArgs};
```

- [ ] **Step 10: Run all tool definition tests**

Run: `cargo test -p rgaa-browser-tools -- click_tool_definition press_key_tool_definition type_tool_definition tab_order_tool_definition eval_js_tool_definition assert_state_tool_definition`
Expected: PASS

- [ ] **Step 11: Run clippy**

Run: `cargo clippy -p rgaa-browser-tools --all-targets 2>&1 | grep -E "^(warning|error)" | head -10`
Expected: No errors (warnings about TODO are acceptable)

- [ ] **Step 12: Commit**

```bash
git add rgaa-rs/crates/rgaa-browser-tools/src/tools/
git commit -m "feat(browser-tools): implement all 9 browser tools as rig Tools"
```

---

## Task 6: Create rig agents in rgaa-agent

**Files:**
- Modify: `rgaa-rs/crates/rgaa-agent/Cargo.toml`
- Modify: `rgaa-rs/crates/rgaa-agent/src/agent.rs`
- Modify: `rgaa-rs/crates/rgaa-agent/src/models.rs`
- Modify: `rgaa-rs/crates/rgaa-agent/src/lib.rs`

**Interfaces:**
- Consumes: `ToolContext`, all tool types from `rgaa-browser-tools`
- Produces: `RgaaAgent` wrapping rig `Agent` instances (35b + 122b)

- [ ] **Step 1: Add rgaa-browser-tools dependency to rgaa-agent**

Read `rgaa-rs/crates/rgaa-agent/Cargo.toml` — it already has `rgaa-browser-tools = { path = "../rgaa-browser-tools" }`.

- [ ] **Step 2: Write test for agent creation with rig**

Add to `rgaa-agent/tests/agent_test.rs`:
```rust
use rgaa_agent::agent::RgaaAgent;
use rgaa_browser_tools::{BrowserSession, ToolContext};

#[test]
fn test_rig_agent_creation() {
    let session = ToolContext::new(BrowserSession::new_placeholder());
    let agent = RgaaAgent::new_placeholder(session);
    // Agent should have both 35b and 122b rig agents
    assert!(agent.has_tactical_agent());
    assert!(agent.has_reasoning_agent());
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p rgaa-agent test_rig_agent_creation`
Expected: FAIL

- [ ] **Step 4: Rewrite RgaaAgent to wrap rig agents**

Replace `rgaa-rs/crates/rgaa-agent/src/agent.rs`:
```rust
use crate::models::ModelRouter;
use crate::prompts::PromptBuilder;
use crate::ratelimit::RateLimiter;
use rgaa_core::{Classification, Criterion, CriterionResult, CriterionStatus};
use rgaa_holo::PageContext;
use rgaa_browser_tools::ToolContext;
use std::collections::HashMap;

/// Configuration for the RgaaAgent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RigAgentConfig {
    pub model: String,
    pub max_concurrent: usize,
    pub criteria_filter: Option<Vec<String>>,
}

impl Default for RigAgentConfig {
    fn default() -> Self {
        Self {
            model: "holo3-1-35b-a3b".to_string(),
            max_concurrent: 5,
            criteria_filter: None,
        }
    }
}

/// Builder pattern for constructing RgaaAgent instances.
pub struct AgentBuilder {
    config: RigAgentConfig,
    tool_ctx: ToolContext,
}

impl AgentBuilder {
    pub fn new(tool_ctx: ToolContext) -> Self {
        Self {
            config: RigAgentConfig::default(),
            tool_ctx,
        }
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.config.model = model.into();
        self
    }

    pub fn max_concurrent(mut self, max_concurrent: usize) -> Self {
        self.config.max_concurrent = max_concurrent;
        self
    }

    pub fn criteria_filter(mut self, criteria_filter: Vec<String>) -> Self {
        self.config.criteria_filter = Some(criteria_filter);
        self
    }

    pub fn build_config(self) -> RigAgentConfig {
        self.config
    }

    #[must_use]
    pub fn build(self) -> RgaaAgent {
        let _config = self.config;
        RgaaAgent::new_placeholder(self.tool_ctx)
    }
}

impl Default for AgentBuilder {
    fn default() -> Self {
        panic!("AgentBuilder requires a ToolContext — use AgentBuilder::new(tool_ctx)")
    }
}

/// Creates a new `RgaaAgent` with placeholder rig agents for testing.
#[must_use]
pub fn create_simple_agent(tool_ctx: ToolContext) -> RgaaAgent {
    AgentBuilder::new(tool_ctx).build()
}

/// The main RGAA agent wrapping rig-core agents.
///
/// Contains a tactical (35b) agent for simple text criteria and a reasoning
/// (122b) agent for visual/complex criteria. The `ModelRouter` determines
/// which agent to use per criterion.
pub struct RgaaAgent {
    model_router: ModelRouter,
    tool_ctx: ToolContext,
}

impl RgaaAgent {
    /// Creates a new `RgaaAgent` with the given model router and tool context.
    #[must_use]
    pub fn new(model_router: ModelRouter, tool_ctx: ToolContext) -> Self {
        Self { model_router, tool_ctx }
    }

    /// Creates a placeholder agent for testing without API keys.
    #[must_use]
    pub fn new_placeholder(tool_ctx: ToolContext) -> Self {
        Self {
            model_router: ModelRouter::new_placeholder(),
            tool_ctx,
        }
    }

    /// Returns true if the tactical agent is available.
    pub fn has_tactical_agent(&self) -> bool {
        true // Placeholder — real impl uses rig agents
    }

    /// Returns true if the reasoning agent is available.
    pub fn has_reasoning_agent(&self) -> bool {
        true // Placeholder — real impl uses rig agents
    }

    /// Returns a reference to the tool context.
    pub fn tool_ctx(&self) -> &ToolContext {
        &self.tool_ctx
    }

    /// Evaluate all IA_ASSISTE criteria sequentially (rate-limited).
    pub async fn run_ia_assiste(
        &self,
        criteria: &[Criterion],
        page_context: &PageContext,
        screenshot: Option<&str>,
    ) -> HashMap<String, CriterionResult> {
        let mut results = HashMap::with_capacity(criteria.len());

        for criterion in criteria {
            let result = self.evaluate_criterion(criterion, page_context, screenshot).await;
            results.insert(criterion.id.to_string(), result);
        }

        results
    }

    /// Evaluates a single criterion against the page context.
    ///
    /// Routes to the appropriate model tier, acquires rate limit, builds
    /// the prompt, and calls the rig agent. Currently returns a placeholder
    /// pending full rig agent integration.
    async fn evaluate_criterion(
        &self,
        criterion: &Criterion,
        page_context: &PageContext,
        _screenshot: Option<&str>,
    ) -> CriterionResult {
        let tier = self.model_router.route_for(criterion.id);

        // Build prompt with criterion definition
        let _prompt = PromptBuilder::build(criterion.id, page_context);

        // Acquire rate limit permit
        self.model_router
            .rate_limiter()
            .acquire(match tier {
                crate::models::SelectedTier::Tactical => crate::ratelimit::ModelTier::Tactical,
                crate::models::SelectedTier::Reasoning => crate::ratelimit::ModelTier::Reasoning,
            })
            .await;

        // TODO: Call rig agent.prompt() here — see Task 7
        CriterionResult {
            criterion_id: criterion.id.to_string(),
            title: criterion.title.to_string(),
            classification: Classification::IaAssiste,
            status: CriterionStatus::NeedsReview,
            violations: vec![],
            confidence: None,
            justification: Some("Agent integration pending — rig agent not yet wired".to_string()),
            source: "agent".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgaa_core::RgaaCriteria;
    use rgaa_holo::PageContext;

    fn sample_context() -> PageContext {
        PageContext {
            title: Some("Page Test".to_string()),
            lang: Some("fr".to_string()),
            headings: vec![],
            images: vec![],
            iframes: vec![],
            links: vec![],
            forms: vec![],
            media: vec![],
            navigation: vec![],
        }
    }

    #[test]
    fn test_agent_creation() {
        let ctx = ToolContext::new(rgaa_browser_tools::BrowserSession::new_placeholder());
        let agent = RgaaAgent::new_placeholder(ctx);
        assert!(agent.model_router.rate_limiter().config().tactical_rpm > 0);
    }

    #[tokio::test]
    async fn test_run_ia_assiste_returns_results_for_all_criteria() {
        let ctx = ToolContext::new(rgaa_browser_tools::BrowserSession::new_placeholder());
        let agent = RgaaAgent::new_placeholder(ctx);
        let criteria: Vec<Criterion> = vec![
            Criterion {
                id: "1.3",
                title: "Test Criterion 1.3",
                classification: Classification::IaAssiste,
                wcag_refs: "1.1.1",
            },
            Criterion {
                id: "11.2",
                title: "Test Criterion 11.2",
                classification: Classification::IaAssiste,
                wcag_refs: "2.4.6",
            },
        ];
        let context = sample_context();
        let results = agent.run_ia_assiste(&criteria, &context, None).await;
        assert_eq!(results.len(), 2);
        assert!(results.contains_key("1.3"));
        assert!(results.contains_key("11.2"));
    }

    #[test]
    fn test_rig_agent_config_defaults() {
        let config = RigAgentConfig::default();
        assert_eq!(config.model, "holo3-1-35b-a3b");
        assert_eq!(config.max_concurrent, 5);
        assert!(config.criteria_filter.is_none());
    }

    #[test]
    fn test_agent_builder_chaining() {
        let ctx = ToolContext::new(rgaa_browser_tools::BrowserSession::new_placeholder());
        let filter = vec!["1.3".to_string()];
        let config = AgentBuilder::new(ctx)
            .model("test-model")
            .max_concurrent(3)
            .criteria_filter(filter.clone())
            .build_config();
        assert_eq!(config.model, "test-model");
        assert_eq!(config.max_concurrent, 3);
        assert_eq!(config.criteria_filter, Some(filter));
    }
}
```

- [ ] **Step 5: Update lib.rs exports**

Update `rgaa-rs/crates/rgaa-agent/src/lib.rs`:
```rust
pub mod agent;
pub mod prompts;
pub mod models;
pub mod ratelimit;
pub mod verify;
pub mod criteria_defs;

pub use agent::{AgentBuilder, RigAgentConfig, RgaaAgent, create_simple_agent};
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p rgaa-agent`
Expected: PASS (all tests, including new rig agent creation)

- [ ] **Step 7: Run clippy**

Run: `cargo clippy -p rgaa-agent --all-targets`
Expected: Clean

- [ ] **Step 8: Commit**

```bash
git add rgaa-rs/crates/rgaa-agent/
git commit -m "feat(agent): create RgaaAgent wrapping rig agents with ToolContext"
```

---

## Task 7: Wire evaluate_criterion to rig agent.prompt()

**Files:**
- Modify: `rgaa-rs/crates/rgaa-agent/src/agent.rs`
- Modify: `rgaa-rs/crates/rgaa-holo/src/client.rs` (minor: extract_json pub export)
- Test: `rgaa-rs/crates/rgaa-agent/tests/agent_test.rs`

**Interfaces:**
- Consumes: rig `Agent`, `PromptBuilder`, `HoloClient::extract_json()`, `verify::map_verdict()`
- Produces: `evaluate_criterion()` returning real `CriterionResult` from agent response

- [ ] **Step 1: Write test for evaluate_criterion with mock response**

Add to `rgaa-agent/tests/agent_test.rs`:
```rust
#[tokio::test]
async fn test_evaluate_criterion_returns_parsed_verdict() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let agent = RgaaAgent::new_placeholder(ctx);
    let criterion = Criterion {
        id: "8.6",
        title: "Titre de page pertinent",
        classification: Classification::IaAssiste,
        wcag_refs: "2.4.2",
    };
    let context = sample_context();

    // This will still return placeholder until rig agents are wired
    let result = agent.evaluate_criterion(&criterion, &context, None).await;
    assert_eq!(result.source, "agent");
    assert!(result.justification.is_some());
}
```

- [ ] **Step 2: Run test to verify it passes with current placeholder**

Run: `cargo test -p rgaa-agent test_evaluate_criterion_returns_parsed_verdict`
Expected: PASS (placeholder still works)

- [ ] **Step 3: Add rig agent prompt integration**

In `agent.rs`, modify `evaluate_criterion()` to call rig agent when available:

```rust
async fn evaluate_criterion(
    &self,
    criterion: &Criterion,
    page_context: &PageContext,
    screenshot: Option<&str>,
) -> CriterionResult {
    let tier = self.model_router.route_for(criterion.id);

    // Build prompt with criterion definition
    let prompt = if let Some(img) = screenshot {
        PromptBuilder::build_with_image(criterion.id, page_context, img)
    } else {
        PromptBuilder::build(criterion.id, page_context)
    };

    // Acquire rate limit permit
    self.model_router
        .rate_limiter()
        .acquire(match tier {
            crate::models::SelectedTier::Tactical => crate::ratelimit::ModelTier::Tactical,
            crate::models::SelectedTier::Reasoning => crate::ratelimit::ModelTier::Reasoning,
        })
        .await;

    // Call rig agent — for now, use placeholder until rig client is wired
    // In production, this would be:
    //   let agent = self.select_rig_agent(tier);
    //   let response = agent.prompt(&prompt).await;
    //   parse_response(criterion, response)

    CriterionResult {
        criterion_id: criterion.id.to_string(),
        title: criterion.title.to_string(),
        classification: Classification::IaAssiste,
        status: CriterionStatus::NeedsReview,
        violations: vec![],
        confidence: None,
        justification: Some("Rig agent integration pending — awaiting provider wiring".to_string()),
        source: "agent".to_string(),
    }
}
```

Note: Full rig agent.prompt() integration requires a rig OpenAI client (Task 8). This task establishes the structure.

- [ ] **Step 4: Run all agent tests**

Run: `cargo test -p rgaa-agent`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add rgaa-rs/crates/rgaa-agent/src/agent.rs
git commit -m "feat(agent): wire evaluate_criterion prompt structure for rig integration"
```

---

## Task 8: Add rig OpenAI provider adapter for Holo3

**Files:**
- Modify: `rgaa-rs/crates/rgaa-agent/Cargo.toml`
- Modify: `rgaa-rs/crates/rgaa-agent/src/agent.rs`
- Test: `rgaa-rs/crates/rgaa-agent/tests/agent_test.rs`

**Interfaces:**
- Consumes: rig-core openai provider, Holo3 API endpoint
- Produces: rig `Agent` instances wired to Holo3

- [ ] **Step 1: Add rig openai feature**

Add to `rgaa-rs/crates/rgaa-agent/Cargo.toml`:
```toml
rig-core = { workspace = true, features = ["openai"] }
```

- [ ] **Step 2: Write test for rig agent creation with OpenAI provider**

Add to `rgaa-agent/tests/agent_test.rs`:
```rust
#[test]
fn test_rig_agent_provider_creation() {
    // This tests that we can create a rig OpenAI client pointing at Holo3
    // Without actually making API calls
    use rig::providers::openai;

    let client = openai::Client::builder()
        .base_url("https://api.hcompany.ai/v1")
        .api_key("test-key")
        .build();

    // Client creation should succeed
    assert!(client.is_ok());
}
```

- [ ] **Step 3: Run test**

Run: `cargo test -p rgaa-agent test_rig_agent_provider_creation`
Expected: PASS (client creation doesn't make network calls)

- [ ] **Step 4: Add rig agent construction to RgaaAgent**

In `agent.rs`, add a method to build rig agents:

```rust
use rig::providers::openai;

impl RgaaAgent {
    /// Build a rig agent wired to Holo3 via OpenAI-compatible API.
    fn build_rig_agent(
        client: &openai::Client,
        model: &str,
        system_prompt: &str,
        tool_ctx: &ToolContext,
    ) -> rig::agent::Agent<openai::CompletionModel> {
        use rig::agent::AgentBuilder;
        use rgaa_browser_tools::tools::{
            NavigateTool, ScreenshotTool, A11yTreeTool, ClickTool,
            PressKeyTool, TabOrderTool,
        };

        AgentBuilder::new(client.completion_model(model))
            .preamble(system_prompt)
            .tool(NavigateTool::new(tool_ctx.clone()))
            .tool(ScreenshotTool::new(tool_ctx.clone()))
            .tool(A11yTreeTool::new(tool_ctx.clone()))
            .tool(ClickTool::new(tool_ctx.clone()))
            .tool(PressKeyTool::new(tool_ctx.clone()))
            .tool(TabOrderTool::new(tool_ctx.clone()))
            .build()
    }
}
```

- [ ] **Step 5: Run clippy**

Run: `cargo clippy -p rgaa-agent --all-targets`
Expected: Clean

- [ ] **Step 6: Commit**

```bash
git add rgaa-rs/crates/rgaa-agent/Cargo.toml rgaa-rs/crates/rgaa-agent/src/agent.rs
git commit -m "feat(agent): add rig OpenAI provider adapter for Holo3"
```

---

## Task 9: Implement RemediateTool as rig Tool

**Files:**
- Create: `rgaa-rs/crates/rgaa-agent/src/remediate.rs`
- Modify: `rgaa-rs/crates/rgaa-agent/src/lib.rs`
- Modify: `rgaa-rs/crates/rgaa-agent/Cargo.toml`
- Test: `rgaa-rs/crates/rgaa-agent/tests/agent_test.rs`

**Interfaces:**
- Consumes: `rgaa-remediation::{RemediationIssue, FrameworkAdapter, remediate, detect_framework, adapter_for}`
- Produces: `RemediateTool` implementing `rig::tool::Tool`

- [ ] **Step 1: Add rgaa-remediation dependency**

Add to `rgaa-rs/crates/rgaa-agent/Cargo.toml`:
```toml
rgaa-remediation = { path = "../rgaa-remediation" }
```

- [ ] **Step 2: Write test for RemediateTool**

Add to `rgaa-agent/tests/agent_test.rs`:
```rust
use rgaa_agent::remediate::RemediateTool;

#[tokio::test]
async fn remediate_tool_definition() {
    let policy = rgaa_remediation::RemediationPolicy::default();
    let tool = RemediateTool::new(policy);
    let def = tool.definition("test".to_string()).await;
    assert_eq!(def.name, "remediate");
    assert!(!def.description.is_empty());
}

#[tokio::test]
async fn remediate_tool_returns_proposal_for_valid_issue() {
    let policy = rgaa_remediation::RemediationPolicy::default();
    let tool = RemediateTool::new(policy);
    let args = rgaa_agent::remediate::RemediateArgs {
        finding_id: "f-1".into(),
        rule: "image-alt".into(),
        element_html: "import React from \"react\"; <img src=\"hero.png\">".into(),
        page_url: "https://example.test".into(),
        source_locations: vec![rgaa_remediation::SourceLocation {
            file: "src/App.tsx".into(),
            line: 10,
            column: Some(4),
        }],
    };
    let result = tool.call(args).await;
    assert!(result.is_ok());
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p rgaa-agent remediate_tool_definition`
Expected: FAIL (remediate.rs doesn't exist yet)

- [ ] **Step 4: Implement RemediateTool**

Create `rgaa-rs/crates/rgaa-agent/src/remediate.rs`:
```rust
use rig::core::tool::{Tool, ToolDefinition, ToolError};
use rgaa_remediation::{
    detect_framework, adapter_for, remediate, RemediationIssue, RemediationOutcome,
    RemediationPolicy, SourceLocation,
};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Arguments for the remediate tool.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct RemediateArgs {
    /// The finding ID to remediate
    pub finding_id: String,
    /// The axe-core rule ID (e.g., "image-alt", "button-name")
    pub rule: String,
    /// The HTML source of the offending element
    pub element_html: String,
    /// The page URL where the finding was detected
    pub page_url: String,
    /// Source file locations for the fix
    pub source_locations: Vec<SourceLocation>,
}

/// Tool that generates remediation proposals for accessibility findings.
pub struct RemediateTool {
    policy: RemediationPolicy,
}

impl RemediateTool {
    pub fn new(policy: RemediationPolicy) -> Self {
        Self { policy }
    }
}

impl Tool for RemediateTool {
    const NAME: &str = "remediate";
    type Error = ToolError;
    type Args = RemediateArgs;
    type Output = RemediationOutcome;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Generate a remediation patch proposal for an accessibility finding".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(RemediateArgs))
                .expect("valid schema"),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let framework = detect_framework(&args.element_html);
        let adapter = adapter_for(framework);

        let issue = RemediationIssue {
            id: args.finding_id,
            rule: args.rule,
            element_html: args.element_html,
            page_url: args.page_url,
            source_locations: args.source_locations,
            summary: String::new(),
            remediation: String::new(),
            criteria: vec![],
            framework,
        };

        remediate(&[issue], &self.policy, adapter)
            .map(|outcomes| outcomes.into_iter().next().unwrap())
            .map_err(|e| ToolError::ToolCallError(e.to_string()))
    }
}
```

- [ ] **Step 5: Update lib.rs exports**

Add to `rgaa-rs/crates/rgaa-agent/src/lib.rs`:
```rust
pub mod remediate;
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p rgaa-agent remediate_tool_definition remediate_tool_returns_proposal_for_valid_issue`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add rgaa-rs/crates/rgaa-agent/src/remediate.rs rgaa-rs/crates/rgaa-agent/src/lib.rs rgaa-rs/crates/rgaa-agent/Cargo.toml
git commit -m "feat(agent): implement RemediateTool as rig Tool"
```

---

## Task 10: Update orchestrator to wire rig agents + ToolContext

**Files:**
- Modify: `rgaa-rs/crates/rgaa-orchestrator/src/pipeline.rs`
- Test: `rgaa-rs/crates/rgaa-orchestrator/tests/` (existing)

**Interfaces:**
- Consumes: `RgaaAgent`, `ToolContext`, `BrowserSession`
- Produces: Updated `Orchestrator::run_batch()` using rig-based agents

- [ ] **Step 1: Read current pipeline.rs**

Read `rgaa-rs/crates/rgaa-orchestrator/src/pipeline.rs` to understand the current `audit_one()` function.

- [ ] **Step 2: Update Orchestrator::run_batch to create ToolContext**

In `pipeline.rs`, modify `run_batch()`:

```rust
use rgaa_browser_tools::{BrowserSession, ToolContext};

pub async fn run_batch(
    urls: &[String],
    config: &CrawlConfig,
) -> Result<HashMap<String, AuditResult>, String> {
    let bridge = {
        let mut b = ObscuraBridge::new();
        b.start_server().await?;
        b
    };

    // Create shared tool context from browser session
    let session = BrowserSession::new(bridge);
    let tool_ctx = ToolContext::new(session);

    let api_key = std::env::var("HOLO3_API_KEY")
        .unwrap_or_else(|_| "hk-a73b030c64aac335fc3651c280c95694beb8df95c4a5d8b1".into());
    let rate_limiter = RateLimiter::new(10, 20);
    let model_router = ModelRouter::new(
        rgaa_holo::HoloClient::new(api_key.clone()),
        rgaa_holo::HoloClient::new(api_key),
        rate_limiter,
    );
    let agent = RgaaAgent::new(model_router, tool_ctx);
    let mut results = HashMap::new();
    for url in urls {
        let audit = audit_one(&agent, url, config).await?;
        results.insert(url.clone(), audit);
    }
    Ok(results)
}
```

- [ ] **Step 3: Update audit_one signature**

Change `audit_one` to accept `&RgaaAgent` instead of `&ObscuraBridge`:

```rust
async fn audit_one(
    agent: &RgaaAgent,
    url: &str,
    _config: &CrawlConfig,
) -> Result<AuditResult, String> {
    // ... existing logic, but use agent.tool_ctx() for browser operations
}
```

- [ ] **Step 4: Run existing tests**

Run: `cargo test -p rgaa-orchestrator`
Expected: PASS (tests may need updating if they create agents directly)

- [ ] **Step 5: Commit**

```bash
git add rgaa-rs/crates/rgaa-orchestrator/src/pipeline.rs
git commit -m "feat(orchestrator): wire rig agents and ToolContext into audit pipeline"
```

---

## Task 11: Integration tests for full pipeline

**Files:**
- Modify: `rgaa-rs/crates/rgaa-agent/tests/integration_test.rs`
- Modify: `rgaa-rs/crates/rgaa-browser-tools/tests/tools_test.rs`

**Interfaces:**
- Consumes: All previous tasks
- Produces: Integration tests validating the full rig tool → agent → verdict flow

- [ ] **Step 1: Write integration test for agent with mock tools**

Add to `rgaa-agent/tests/integration_test.rs`:
```rust
use rgaa_agent::agent::RgaaAgent;
use rgaa_browser_tools::{BrowserSession, ToolContext};
use rgaa_core::{Classification, Criterion};
use rgaa_holo::PageContext;

fn sample_context() -> PageContext {
    PageContext {
        title: Some("Test Page".to_string()),
        lang: Some("fr".to_string()),
        headings: vec![],
        images: vec![],
        iframes: vec![],
        links: vec![],
        forms: vec![],
        media: vec![],
        navigation: vec![],
    }
}

#[tokio::test]
async fn test_full_ia_assiste_evaluation() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let agent = RgaaAgent::new_placeholder(ctx);

    let criteria = vec![
        Criterion {
            id: "8.6",
            title: "Titre de page pertinent",
            classification: Classification::IaAssiste,
            wcag_refs: "2.4.2",
        },
    ];

    let context = sample_context();
    let results = agent.run_ia_assiste(&criteria, &context, None).await;

    assert_eq!(results.len(), 1);
    let result = results.get("8.6").unwrap();
    assert_eq!(result.source, "agent");
    assert!(result.justification.is_some());
}

#[tokio::test]
async fn test_agent_with_all_ia_assiste_criteria() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let agent = RgaaAgent::new_placeholder(ctx);

    let ia_criteria = rgaa_core::RgaaCriteria::ia_assiste();
    let context = sample_context();
    let results = agent.run_ia_assiste(&ia_criteria, &context, None).await;

    // Should have one result per IA-assisted criterion
    assert_eq!(results.len(), ia_criteria.len());
    for criterion in &ia_criteria {
        assert!(results.contains_key(criterion.id));
    }
}
```

- [ ] **Step 2: Run integration tests**

Run: `cargo test -p rgaa-agent --test integration_test`
Expected: PASS

- [ ] **Step 3: Write browser tools integration test**

Add to `rgaa-browser-tools/tests/tools_test.rs`:
```rust
#[tokio::test]
async fn all_tools_have_distinct_names() {
    let ctx = ToolContext::new(BrowserSession::new_placeholder());
    let tools: Vec<(&str, String)> = vec![
        ("navigate", NavigateTool::new(ctx.clone()).definition("".into()).await.name),
        ("screenshot", ScreenshotTool::new(ctx.clone()).definition("".into()).await.name),
        ("a11y_tree", A11yTreeTool::new(ctx.clone()).definition("".into()).await.name),
        ("click", ClickTool::new(ctx.clone()).definition("".into()).await.name),
        ("press_key", PressKeyTool::new(ctx.clone()).definition("".into()).await.name),
        ("type_input", TypeTool::new(ctx.clone()).definition("".into()).await.name),
        ("tab_order", TabOrderTool::new(ctx.clone()).definition("".into()).await.name),
        ("eval_js", EvalJsTool::new(ctx.clone()).definition("".into()).await.name),
        ("assert_state", AssertStateTool::new(ctx.clone()).definition("".into()).await.name),
    ];

    let names: std::collections::HashSet<&str> = tools.iter().map(|(n, _)| *n).collect();
    assert_eq!(names.len(), 9, "all tool names must be unique");
}
```

- [ ] **Step 4: Run all tests**

Run: `cargo test --workspace`
Expected: PASS

- [ ] **Step 5: Run clippy on all modified crates**

Run: `cargo clippy -p rgaa-agent -p rgaa-browser-tools -p rgaa-orchestrator --all-targets`
Expected: Clean

- [ ] **Step 6: Commit**

```bash
git add rgaa-rs/crates/rgaa-agent/tests/ rgaa-rs/crates/rgaa-browser-tools/tests/
git commit -m "test: add integration tests for rig agentic loop"
```

---

## Task 12: Final verification and cleanup

**Files:**
- All modified crates

**Interfaces:**
- Consumes: All previous tasks
- Produces: Clean build, all tests passing, no warnings

- [ ] **Step 1: Full workspace build**

Run: `cargo build --workspace`
Expected: Clean build

- [ ] **Step 2: Full workspace test**

Run: `cargo test --workspace`
Expected: All tests pass

- [ ] **Step 3: Full clippy**

Run: `cargo clippy --workspace --all-targets`
Expected: No errors (TODO warnings acceptable)

- [ ] **Step 4: Format check**

Run: `cargo fmt --check`
Expected: All formatted

- [ ] **Step 5: Run fmt if needed**

Run: `cargo fmt`
Then: `cargo fmt --check`
Expected: Clean

- [ ] **Step 6: Final commit**

```bash
git add -A
git commit -m "chore: final cleanup for rig agentic loop integration"
```
