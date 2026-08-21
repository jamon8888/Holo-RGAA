# Design: Rig Agentic Loop for Audit + Remediation

- **Date:** 2026-08-21
- **Status:** Approved design, pending implementation plan
- **Owner:** rgaa-agent + rgaa-browser-tools + rgaa-remediation + rgaa-orchestrator
- **Approach:** Rig agent + custom tools (Approach B)
- **Supersedes:** Placeholder agent integration in `rgaa-agent/src/agent.rs`
- **Complements:** `2026-08-20-holo3-agentic-redesign-design.md` (architecture overview)
- **Complements:** `2026-08-18-claude-code-rgaa-remediation-plugin-design.md` (remediation lifecycle)

## Context & Goal

The current `rgaa-agent` crate has a hand-rolled `RgaaAgent` that returns placeholder results
(`"Agent integration pending"`). rig-core v0.42.0 is a workspace dependency but zero code uses it.
The browser tools exist as plain structs with `execute()` methods that aren't registered as rig
tools. The remediation system is complete but disconnected from the agent.

**Goal:** Wire rig-core as the agent runtime so the model can drive the browser via tools,
evaluate IA_ASSISTE criteria in a multi-turn loop, and propose remediation fixes for failures.

## Approach: Rig Agent + Custom Tools

Use rig's `AgentBuilder` for agent construction, implement browser tools as rig `Tool` traits,
but keep the existing `ModelRouter` and `RateLimiter` as external orchestration. This preserves
the dual-model routing (35b tactical / 122b reasoning) while gaining rig's tool-calling loop.

## Section 1: Rig Provider + Agent Construction

### Problem

`HoloClient` calls Holo3 via raw `reqwest`. Holo3's API (`api.hcompany.ai/v1/chat/completions`)
is OpenAI-compatible. rig-core has an OpenAI provider that handles retries, streaming, and tool
calling natively.

### Solution

Wrap Holo3 as a rig OpenAI provider client. Build the agent via `AgentBuilder`. Keep `ModelRouter`
for tier selection outside rig's loop.

```rust
// Build rig OpenAI client pointing at Holo3
let holo_client = openai::Client::builder()
    .base_url("https://api.hcompany.ai/v1")
    .api_key(api_key)
    .build()?;

// Build 35b tactical agent
let tactical_agent = holo_client.agent("holo3-1-35b-a3b")
    .preamble(EXPERT_SYSTEM_PROMPT)
    .tool(NavigateTool { session.clone() })
    .tool(ScreenshotTool { session.clone() })
    .tool(A11yTreeTool { session.clone() })
    .tool(ClickTool { session.clone() })
    .tool(PressKeyTool { session.clone() })
    .tool(TabOrderTool { session.clone() })
    .build();

// Build 122b reasoning agent (same tools, different model)
let reasoning_agent = holo_client.agent("holo3-122b-a10b")
    .preamble(EXPERT_SYSTEM_PROMPT)
    .tool(NavigateTool { session.clone() })
    .tool(ScreenshotTool { session.clone() })
    .tool(A11yTreeTool { session.clone() })
    .tool(ClickTool { session.clone() })
    .tool(PressKeyTool { session.clone() })
    .tool(TabOrderTool { session.clone() })
    .build();
```

### Key Changes

| Component | Before | After |
|-----------|--------|-------|
| HoloClient | Raw reqwest HTTP calls | rig OpenAI provider |
| RgaaAgent | Hand-rolled struct | rig `Agent` via `AgentBuilder` |
| evaluate_criterion() | Returns placeholder | Calls `agent.prompt()` |
| Response parsing | N/A (placeholder) | `HoloClient::extract_json()` on rig response |
| ModelRouter | Stays | Selects which rig agent to invoke per criterion |
| RateLimiter | Stays | rig has no built-in rate limiting |

### Response Flow

```
1. ModelRouter routes criterion to 35b or 122b agent
2. RateLimiter acquires permit
3. PromptBuilder builds enriched prompt with criterion definition
4. agent.prompt(&prompt) → rig runs multi-turn loop
5. Rig returns final text response
6. HoloClient::extract_json() parses verdict/confidence/justification
7. verify::map_verdict() maps to CriterionStatus
8. CriterionResult returned with evidence traces
```

## Section 2: Browser Tools as rig Tools

### Problem

The 9 browser tools in `rgaa-browser-tools/src/tools/` are plain structs. rig needs tools that
implement the `Tool` trait so the model can call them during evaluation.

### Solution

Each tool implements `rig::tool::Tool` with typed `Args`/`Output`, `schemars::JsonSchema` for
parameter schemas, and shared `BrowserSession` via `Arc<Mutex<BrowserSession>>`.

### Tool Inventory

| Tool | rig NAME | Args | Output | CDP Connection |
|------|----------|------|--------|----------------|
| NavigateTool | `navigate` | `url: String` | success + message | `Page.navigate` |
| ScreenshotTool | `screenshot` | none | base64 PNG | `Page.captureScreenshot` |
| A11yTreeTool | `a11y_tree` | none | AXTree JSON | `Accessibility.getFullAXTree` |
| ClickTool | `click` | `ref_id: String` | success + focused element | `DOM.focus` + `Input.dispatchMouseEvent` |
| TypeTool | `type_input` | `ref_id: String, text: String` | success | `Input.dispatchKeyEvent` |
| PressKeyTool | `press_key` | `key: String` | success + focused element | `Input.dispatchKeyEvent` |
| TabOrderTool | `tab_order` | none | ordered focusable elements | Derived from a11y tree |
| EvalJsTool | `eval_js` | `expression: String` | JS result | `Runtime.evaluate` |
| AssertStateTool | `assert_state` | `predicate: String` | bool | Varies per predicate |

### Shared State Pattern

```rust
pub struct ToolContext {
    pub session: Arc<Mutex<BrowserSession>>,
}

// Each tool holds a clone of the Arc
pub struct NavigateTool {
    ctx: Arc<Mutex<BrowserSession>>,
}

impl Tool for NavigateTool {
    // ...

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let mut session = self.ctx.lock().await;
        session.set_current_url(args.url.clone());
        // CDP call via session.bridge()
        Ok(NavigateOutput { success: true, message: format!("Navigated to {}", args.url) })
    }
}
```

### Tool Definition Pattern

```rust
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct NavigateArgs {
    /// The URL to navigate the browser to
    pub url: String,
}

impl Tool for NavigateTool {
    const NAME: &str = "navigate";
    type Error = ToolError;
    type Args = NavigateArgs;
    type Output = NavigateOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "navigate".to_string(),
            description: "Navigate the browser to a URL and return success status".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(NavigateArgs)).unwrap(),
        }
    }
}
```

### Caveats

- `Tool::call()` takes `&self` but we need `&mut session`. `Mutex` handles this.
- Lock held only during tool execution, not across await points in agent loop.
- rig converts tool errors to strings and feeds them back to the model for retry.

## Section 3: Agentic Act→Verify Loop

### Problem

The current `evaluate_criterion()` builds a prompt and returns a placeholder. For interaction
criteria (12.8 tab order, 11.2 label relevance), the agent needs to drive the browser.

### How rig Handles This

rig's `Agent` implements the `Prompt` trait. When you call `agent.prompt()`, rig runs a
multi-turn loop: send prompt → model responds → if model calls a tool, execute it, feed result
back → repeat until model emits a final text response. This IS the act→verify loop.

### Two Evaluation Modes

**Mode 1: Text-only (tactical 35b criteria)**

```rust
// Simple prompt, no tools needed
let response = tactical_agent.prompt(
    "Évalue critère 8.6: Titre de page pertinent. Contexte: ..."
).await;
// Single turn, model reads context and returns verdict
```

**Mode 2: Agentic (reasoning 122b criteria)**

```rust
// Model uses tools to interact with the page
let response = reasoning_agent.prompt(
    "Évalue critère 12.8: Ordre tabulation cohérent. \
     Utilise les outils navigate, press_key, a11y_tree pour tester l'ordre de tabulation."
).await;
// Multi-turn: navigate → a11y_tree → press_key(Tab) × N → compare → verdict
```

### Implementation

```rust
async fn evaluate_criterion(&self, criterion, page_context, screenshot) -> CriterionResult {
    let tier = self.model_router.route_for(criterion.id);
    let agent = self.select_agent(tier);

    self.rate_limiter.acquire(tier_to_model_tier(tier)).await;

    let prompt = if let Some(img) = screenshot {
        PromptBuilder::build_with_image(criterion.id, page_context, img)
    } else {
        PromptBuilder::build(criterion.id, page_context)
    };

    // rig runs the multi-turn loop with tool calls
    let response = agent.prompt(&prompt).await;

    match HoloClient::extract_json(&response) {
        Some(holo_resp) => {
            let status = verify::map_verdict(holo_resp);
            CriterionResult {
                criterion_id: criterion.id.to_string(),
                title: criterion.title.to_string(),
                classification: Classification::IaAssiste,
                status,
                confidence: Some(holo_resp.confidence),
                justification: Some(holo_resp.justification),
                source: "agent".to_string(),
                violations: vec![],
            }
        }
        None => CriterionResult {
            criterion_id: criterion.id.to_string(),
            title: criterion.title.to_string(),
            classification: Classification::IaAssiste,
            status: CriterionStatus::NeedsReview,
            confidence: None,
            justification: Some("Failed to parse agent response".into()),
            source: "agent".into(),
            violations: vec![],
        },
    }
}
```

### Evidence Traces

rig's agent loop provides tool call history via completion calls. We capture it via the
agent's completion history or by inspecting `AgentRun` steps. The existing `ActionTrace`
type in `verify.rs` matches what we need.

### Retry on Stale State

If the model calls `press_key` but the a11y tree hasn't updated, the tool returns an error.
rig converts tool errors to strings and feeds them back to the model, which can retry.
Max turns controlled by rig's `max_turns` parameter on `AgentBuilder`.

## Section 4: Remediation Integration

### Problem

The agent evaluates criteria and produces `CriterionResult` with `Fail` status, but there's no
path from failure → fix proposal → approval → verification. `rgaa-remediation` has the complete
lifecycle but isn't wired to the agent.

### Solution

Add a `RemediateTool` to the rig agent that the model can call when it detects a failure.
The tool takes a finding, generates a `PatchProposal` via the existing `FrameworkAdapter`,
and returns it for human approval.

### New Rig Tool — RemediateTool

```rust
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct RemediateArgs {
    /// The finding ID to remediate
    pub finding_id: String,
    /// The axe-core rule ID (e.g., "image-alt")
    pub rule: String,
    /// The HTML source of the offending element
    pub element_html: String,
    /// The page URL where the finding was detected
    pub page_url: String,
    /// Source file locations for the fix
    pub source_locations: Vec<SourceLocation>,
}

impl Tool for RemediateTool {
    const NAME: &str = "remediate";
    type Error = ToolError;
    type Args = RemediateArgs;
    type Output = RemediationOutcome;

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

### Lifecycle Integration

```
1. Agent evaluates criterion 1.3 → Fail (missing alt text)
2. Agent calls remediate_finding(finding_id, rule, element_html, source_locations)
3. Tool: detect framework → ReactAdapter.propose() → PatchProposal
4. Proposal returned to orchestrator for approval gate
5. After approval: orchestrator applies patch
6. Agent re-evaluates criterion → Pass → finding resolved
```

### Finding Lifecycle States

```
Open → Triaged → FixProposed → AwaitingApproval → Applied → Verifying → Resolved
```

Alternative terminal states: `NeedsReview`, `NotApplicable`, `FalsePositive`, `Deferred`.

### Scope

- Wire `RemediateTool` as a rig tool on the agent
- Connect `FrameworkAdapter` detection to the tool
- Approval gate stays in orchestrator (not in agent loop)
- `VerifyTool` (re-audit after fix) added as follow-up

## Architecture Diagram

```
rgaa-orchestrator
  ├─ ObscuraBridge (CDP)
  │    ├─ run_axe / run_gap_fix        → Déterministe criteria
  │    └─ screenshot(url)              → base64 PNG
  ├─ RgaaAgent (rig AgentBuilder)
  │    ├─ ModelRouter                  → 35b (tactical) / 122b (reasoning)
  │    ├─ RateLimiter                  → token bucket per tier
  │    ├─ rig Agent (35b)             → text-only evaluations
  │    ├─ rig Agent (122b)            → agentic with browser tools
  │    ├─ Browser Tools (rig Tool)    → navigate, screenshot, a11y_tree, etc.
  │    ├─ RemediateTool (rig Tool)    → fix proposals via FrameworkAdapter
  │    └─ PromptBuilder               → criterion definitions + WCAG refs
  ├─ Merge: axe + gap-fix + agent + manuel
  ├─ Approval Gate                    → PatchProposal::ensure_approved()
  └─ AuditResult with evidence traces
```

## Data Flow

```
1. Orchestrator starts ObscuraBridge, builds rig agents (35b + 122b)
2. For each URL:
   a. axe + gap-fix (Déterministe) — unchanged
   b. screenshot(url) once → base64 PNG
   c. For each of 27 IA_ASSISTE criteria:
      - ModelRouter selects tier (35b or 122b rig agent)
      - RateLimiter acquires permit
      - PromptBuilder builds prompt with criterion definition
      - agent.prompt() → rig multi-turn loop with tool calls
      - HoloResponse parsed → CriterionResult with evidence
      - If Fail: agent calls remediate tool → PatchProposal
   d. Merge: axe + gap-fix + agent + manuel
   e. Approval gate for proposals
   f. Calculate compliance over all 106 criteria
3. Return AuditResult with evidence traces + remediation proposals
```

## Error Handling

| Failure | Behavior |
|---------|----------|
| Missing API key | Startup error, no fake default |
| Rate limit (429) | RateLimiter paces; residual → rig retry with backoff |
| Tool call failure | rig feeds error back to model, model can retry |
| Structured output parse failure | NeedsReview with "Failed to parse agent response" |
| Screenshot failure | Evaluation proceeds text-only (warn) |
| Browser tool failure | rig retries (max_turns) → NeedsReview with evidence |
| Model timeout | rig retry → NeedsReview after exhaustion |
| Remediation framework mismatch | NeedsReview with "unsupported framework" |

## Testing Strategy

### Unit tests

- Each rig tool: mock `BrowserSession` → assert output shape
- `RemediateTool`: mock adapter → assert proposal generation
- `ModelRouter`: assert 35b/122b selection per criterion ID
- `PromptBuilder`: assert criterion definition appears in prompt
- `RateLimiter`: fire >RPM requests → assert actual send rate ≤ RPM

### Integration tests

- Full pipeline with mock rig agent: axe + gap-fix + agent → AuditResult
- Agent with mock tools: navigate → a11y_tree → press_key → verdict
- Remediation: agent calls remediate → proposal returned → approval → resolution

### E2E test (optional, live API)

- Navigate to test page → screenshot → 122b → verdict with evidence
- Tab through form → focus order verdict with action traces
- Remediate missing alt text → proposal → approval → re-audit → Pass

## Acceptance Criteria

1. `cargo test -p rgaa-browser-tools` passes (rig tool definitions + CDP mocks)
2. `cargo test -p rgaa-agent` passes (rig agents, router, prompts, rate limiter)
3. `cargo test -p rgaa-orchestrator` passes (full pipeline with rig agents)
4. Live run: visual criteria get screenshots, interaction criteria get a11y tree + actions
5. Agent calls browser tools during multi-turn evaluation (not placeholder)
6. Agent calls remediate tool for failures → PatchProposal generated
7. Low-confidence verdicts (< 0.6) → NeedsReview, not blind Pass/Fail
8. Rate limiter enforces ≤10 RPM for 35b model
9. Every criterion result carries evidence trace (screenshot or action traces)
10. `cargo clippy -p rgaa-browser-tools -p rgaa-agent` clean, no warnings
