# Design: Production-Grade Agentic Holo3 Integration

- **Date:** 2026-08-20
- **Status:** Approved design, pending implementation plan
- **Owner:** rgaa-agent + rgaa-browser-tools + rgaa-orchestrator
- **Supersedes:** `2026-08-16-holo3-deep-integration-design.md` (reliability-only approach)
- **Complements:** `2026-08-16-agentic-rgaa-auditor-architecture.md` (rig+MCP architecture)

## Context & Goal

The current Holo3 integration (`rgaa-holo`) is a 600-line text-only LLM evaluator
that sends page context to the model but never shows it the page. Despite Holo3
being a vision-language model, no screenshots are sent. Prompts lack criterion
definitions. Output parsing is fragile (regex fallback). Low-confidence verdicts
are treated as definitive. Rate limiting is broken (12 concurrent > 10 RPM free
tier). The model cannot interact with the page — it can only read static DOM
snapshots.

**Goal:** A production-grade agentic architecture using `rig` as the agent runtime,
with dual model routing (35b/122b), multimodal prompts for visual criteria,
browser tools for interaction tests, structured outputs, and a per-criterion
evidence trail compliant with EU accessibility enforcement requirements.

## Decisions (from brainstorming)

| Topic | Decision |
|-------|----------|
| Agent runtime | `rig` (OpenAI-compatible provider → Holo3) |
| Tool surface | Both native rig tools AND MCP server (shared `BrowserSession` backend) |
| Model tiering | Dual from day one: 35b tactical + 122b reasoning |
| Scope | Both visual reads + interaction tests from day one |
| Architecture | Layered: `rgaa-browser-tools` → `rgaa-agent` → `rgaa-orchestrator` |
| Low-confidence | `NeedsReview` for confidence < 0.6 |

## Architecture Overview

```
rgaa-orchestrator (existing, modified)
  ├─ ObscuraBridge (CDP)
  │    ├─ run_axe / run_gap_fix        → Déterministe criteria
  │    └─ screenshot(url)              → base64 PNG (once per URL)
  ├─ RgaaAgent (NEW)
  │    ├─ ModelRouter                  → 35b (visual reads) / 122b (interaction)
  │    ├─ PromptBuilder (enriched)     → criterion definitions + WCAG refs
  │    ├─ BrowserTools (native rig)    → navigate, screenshot, a11y-tree, etc.
  │    ├─ MCP Server (rmcp)            → same tools for external consumers
  │    └─ Act→verify loop              → interaction criteria with retries
  ├─ Merge: axe + gap-fix + agent + manuel
  └─ AuditResult with evidence traces
```

## Crate Layout

### `rgaa-browser-tools` (NEW)

Exposes CDP browser capabilities as both native rig tools and an MCP server.

**Dependencies:** `rgaa-obscura`, `rig-core`, `rmcp`, `serde`, `serde_json`, `tokio`

**Files:**
```
src/
  lib.rs           — public API
  session.rs       — BrowserSession (CDP connection, a11y tree cache)
  ax_tree.rs       — AXTree / AXNode types for stable element refs
  tools/
    mod.rs
    navigate.rs    — NavigateTool { url }
    screenshot.rs  — ScreenshotTool {} → base64 PNG
    a11y_tree.rs   — AccessibilityTreeTool {} → AXTree
    eval_js.rs     — EvalJsTool { snippet }
    click.rs       — ClickTool { ref_id }
    type.rs        — TypeTool { ref_id, text }
    press_key.rs   — PressKeyTool { key }
    tab_order.rs   — TabOrderTool {} → ordered focusable elements
    assert_state.rs — AssertStateTool { predicate } → bool
  mcp/
    mod.rs         — MCP server wrapper (rmcp stdio/SSE)
```

**Key types:**

```rust
pub struct BrowserSession {
    bridge: ObscuraBridge,
    last_a11y: Option<AXTree>,
    current_url: Option<String>,
}

pub struct AXTree {
    nodes: Vec<AXNode>,
}

pub struct AXNode {
    backend_node_id: String,  // stable ref for click/type
    role: String,
    name: String,
    children: Vec<String>,
    properties: HashMap<String, String>,
}
```

**Native rig tools** use `#[rig_tool]` derive. Each tool takes a `&BrowserSession`
reference and returns a typed result. The `BrowserSession` is shared across tools
within a single evaluation run.

**MCP server** wraps the same `BrowserSession` implementation. Tool schemas are
identical — the MCP server is a thin protocol adapter.

### `rgaa-agent` (NEW)

The rig-based agent that evaluates IA_ASSISTE criteria.

**Dependencies:** `rgaa-core`, `rgaa-holo`, `rgaa-browser-tools`, `rig-core`, `tokio`

**Files:**
```
src/
  lib.rs           — public API: run_ia_assiste()
  agent.rs         — rig Agent definition, tool registration
  prompts.rs       — enriched PromptBuilder with criterion definitions
  models.rs        — ModelRouter (35b tactical / 122b reasoning)
  ratelimit.rs     — token-bucket RateLimiter per model tier
  verify.rs        — act→verify loop, confidence thresholds
  criteria_defs.rs — curated definitions for 27 IA_ASSISTE criteria
```

**ModelRouter:**

```rust
pub struct ModelRouter {
    tactical: Arc<HoloClient>,   // holo3-1-35b-a3b, 10 RPM (free)
    reasoning: Arc<HoloClient>,  // holo3-122b-a10b, configurable RPM (paid)
    rate_limiter: Arc<RateLimiter>,
}

impl ModelRouter {
    pub async fn evaluate(
        &self,
        criterion: &Criterion,
        prompt: &str,
        image: Option<&str>,
    ) -> Result<HoloResponse> {
        let tier = self.select_tier(criterion);
        self.rate_limiter.acquire(tier).await;
        let client = match tier {
            ModelTier::Tactical => &self.tactical,
            ModelTier::Reasoning => &self.reasoning,
        };
        client.evaluate(prompt, image).await
    }

    fn select_tier(&self, criterion: &Criterion) -> ModelTier {
        if VISUAL_CRITERIA.contains(&criterion.id)
            || criterion.id.starts_with("11.")
            || criterion.id == "12.8"
        {
            ModelTier::Reasoning  // 122b for visual/complex criteria
        } else {
            ModelTier::Tactical   // 35b for simple text reads
        }
    }
}
```

**Criteria classification for model routing:**

```rust
/// Criteria that benefit from visual understanding (screenshot) or complex reasoning.
/// These are routed to the 122b model.
const VISUAL_CRITERIA: &[&str] = &[
    "1.3",  // alt text relevance — compare alt vs actual image
    "1.7",  // detailed description relevance — compare description vs image
    "3.1",  // color-only information — must SEE the page
    "10.3", // reading order — must SEE layout
    "10.10",// CSS-positioned content — must SEE rendering
    "11.2", // label relevance — must SEE label next to input
    "11.3", // fieldset/legend — must SEE form grouping
    "11.7", // error suggestion — complex reasoning
    "11.8", // error identification — complex reasoning
    "11.9", // mandatory field indication — complex reasoning
    "11.10",// form field purpose — complex reasoning
    "12.8", // focus order — must INTERACT with page
    "13.6", // table linearization — must SEE table rendering
];
```

| Tier | Criteria | Why |
|------|----------|-----|
| 122b (reasoning) | 1.3, 1.7, 3.1, 10.3, 10.10, 11.2, 11.3, 11.7, 11.8, 11.9, 11.10, 12.8, 13.6 | Visual judgment, complex forms, focus order |
| 35b (tactical) | 2.2, 4.2, 4.4, 4.6, 4.9, 5.2, 5.3, 5.5, 7.2, 8.4, 8.6, 8.8, 9.2, 12.3 | Text-based relevance checks |

**Enriched prompts:**

```rust
pub fn build(criterion_id: &str, context: &PageContext) -> String {
    let def = get_criterion_definition(criterion_id);
    format!(
        "Évalue le critère RGAA {id} sur cette page web.\n\n\
         ## Critère à évaluer\n\
         - **ID:** {id}\n\
         - **Titre:** {title}\n\
         - **Références WCAG:** {wcag_refs}\n\
         - **Définition:** {definition}\n\n\
         ## Contexte de la page\n{context}\n\n\
         ## Éléments de la page\n{elements}\n\n\
         ## Instructions\n\
         1. Analyse le critère en fonction de la définition et des éléments\n\
         2. Si une capture d'écran est fournie, utilise-la pour juger\n\
         3. Retourne un JSON: verdict (pass/fail/na), confidence (0-1), justification",
        id = criterion_id,
        title = def.title,
        wcag_refs = def.wcag_refs,
        definition = def.definition,
        context = format_context(context),
        elements = format_elements(context),
    )
}
```

**Act→verify loop (interaction criteria):**

```
1. Agent calls navigate(url)
2. Agent calls accessibility_tree() → AXTree
3. Model reads tree, decides: "I need to Tab through this form"
4. Agent calls press_key("Tab") → assert focused element changed
5. Agent calls press_key("Tab") → assert focused element changed
6. ... repeat for expected tab stops
7. Model compares actual focus order vs DOM order
8. Model emits verdict with ActionTrace evidence
```

Retry guard: max 3 retries per action. On stale state → re-fetch a11y tree.
On persistent failure → NeedsReview with "Browser interaction failed" justification.

**Confidence → NeedsReview:**

```rust
const CONFIDENCE_THRESHOLD: f64 = 0.6;

fn map_verdict(response: HoloResponse) -> CriterionStatus {
    if response.confidence < CONFIDENCE_THRESHOLD {
        CriterionStatus::NeedsReview
    } else {
        match response.verdict.as_str() {
            "pass" | "conforme" => CriterionStatus::Pass,
            "fail" | "non_conforme" => CriterionStatus::Fail,
            _ => CriterionStatus::NeedsReview,
        }
    }
}
```

**Rate limiter:**

Token-bucket replacing the static `Semaphore`:

```rust
pub struct RateLimiter {
    tactical_refill: u32,    // 10 tokens per minute
    reasoning_refill: u32,   // configurable (default 20)
    tactical_tokens: AtomicU32,
    reasoning_tokens: AtomicU32,
    last_refill: Mutex<Instant>,
}

impl RateLimiter {
    pub async fn acquire(&self, tier: ModelTier) {
        loop {
            self.refill_if_needed();
            let tokens = match tier {
                ModelTier::Tactical => &self.tactical_tokens,
                ModelTier::Reasoning => &self.reasoning_tokens,
            };
            let prev = tokens.load(Ordering::Acquire);
            if prev > 0 {
                if tokens.compare_exchange(prev, prev - 1, Ordering::AcqRel, Ordering::Acquire).is_ok() {
                    return;
                }
            } else {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}
```

### `rgaa-orchestrator` (MODIFIED)

Minimal changes to integrate the new agent:

```rust
// Before: text-only Holo3 loop
let ia_criteria = RgaaCriteria::ia_assiste();
for criterion in &ia_criteria {
    let prompt = PromptBuilder::build(criterion.id, &page_context);
    let response = holo.evaluate(&prompt).await;
    // ...
}

// After: agentic evaluation
let agent = RgaaAgent::new(model_router, browser_session);
let agent_results = agent.run_ia_assiste(&ia_criteria, &page_context, screenshot).await;
```

The orchestrator still handles axe-core, gap-fix, merge, and compliance calculation.
The agent replaces only the Holo3 evaluation loop.

## Evidence Trail

Every criterion result carries structured evidence:

```rust
pub struct CriterionEvidence {
    pub screenshot: Option<String>,       // base64 PNG at evaluation time
    pub a11y_tree_snapshot: Option<AXTree>,
    pub actions_taken: Vec<ActionTrace>,   // for interaction criteria
    pub page_context: PageContext,
}

pub struct ActionTrace {
    pub tool: String,         // "press_key", "click", "type"
    pub ref_id: Option<String>,
    pub key: Option<String>,
    pub text: Option<String>,
    pub resulting_focused_element: Option<String>,
    pub timestamp_ms: u64,
}
```

This satisfies the EU requirement for per-criterion evidence with provenance.

## Error Handling

| Failure | Behavior |
|---------|----------|
| Missing API key | Startup error, no fake default |
| Rate limit (429) | Limiter already paces; residual → bounded backoff + jitter |
| Structured output parse failure | Retry with stricter prompt → fallback `extract_json` → NeedsReview |
| Screenshot failure | Evaluation proceeds text-only (warn) |
| Browser tool failure | Act→verify retries (max 3) → NeedsReview with evidence |
| Agent panic | Caught by rig, criterion → Error with justification |
| Model timeout | Retry with backoff → NeedsReview after exhaustion |

## Testing

### Unit tests (`rgaa-browser-tools`)
- Each tool: mock CDP response → assert output shape
- MCP server: `rmcp` test client → verify tool schemas and execution
- AXTree parsing: deterministic tree from known a11y snapshot

### Unit tests (`rgaa-agent`)
- Model router: assert 35b/122b selection per criterion ID
- Prompt enrichment: assert criterion definition appears in prompt
- Confidence mapping: 0.3 → NeedsReview, 0.8 → Pass, unknown → NeedsReview
- Rate limiter: fire >RPM requests → assert actual send rate ≤ RPM
- Act→verify: mock browser → assert retry on stale state, max 3 retries
- Mock model: rig `MockModel` → deterministic verdict → correct CriterionResult

### Integration tests (`rgaa-orchestrator`)
- Full pipeline with mock agent: axe + gap-fix + agent → AuditResult
- NeedsReview appears for low-confidence mock verdict
- Compliance calculation includes NeedsReview in denominator

### E2E test (optional, live API)
- Navigate to test page → screenshot → 122b → verdict with evidence
- Tab through form → focus order verdict with action traces

## Data Flow

```
1. Orchestrator starts ObscuraBridge, builds ModelRouter (35b+122b+rate_limiter)
2. For each URL:
   a. axe + gap-fix (Déterministe) — unchanged
   b. screenshot(url) once → base64 PNG
   c. For each of 27 IA_ASSISTE criteria (concurrent, rate-limited):
      - ModelRouter selects tier (35b or 122b)
      - PromptBuilder::build(criterion, context) with definition
      - If visual criterion: send screenshot as multimodal content
      - If interaction criterion: agent drives browser (tab, click, assert)
      - HoloResponse → map_verdict → CriterionResult with evidence
3. Merge: axe + gap-fix + agent + manuel
4. Calculate compliance over all 106 criteria
5. Return AuditResult with evidence traces
```

## Phased Build

| Phase | What | Crates | Depends on |
|-------|------|--------|------------|
| 1 | Browser tools (native rig + MCP) | `rgaa-browser-tools` | `rgaa-obscura` |
| 2 | Agent core (router, prompts, rate limiter) | `rgaa-agent` | Phase 1 |
| 3 | Visual criterion evaluation | `rgaa-agent` | Phase 2 |
| 4 | Interaction criterion evaluation (act→verify) | `rgaa-agent` | Phase 2 |
| 5 | Orchestrator integration | `rgaa-orchestrator` | Phase 3+4 |
| 6 | E2E testing + hardening | all | Phase 5 |

Each phase is a separate PR targeting the previous phase's branch.

## Out of Scope (Phase 2)

- Regulatory RAG (RGAA catalog + EN 301 549 + WCAG 2.2 embeddings)
- Accessibility statement auto-generation
- Multi-year remediation plan generation
- Human-in-the-loop approval hooks (`AgentHook` from rig)
- Crawl support (single URL first, batch later)
- Derogation register

## Acceptance Criteria

1. `cargo test -p rgaa-browser-tools` passes (unit + MCP tests)
2. `cargo test -p rgaa-agent` passes (router, prompts, rate limiter, mock model)
3. `cargo test -p rgaa-orchestrator` passes (full pipeline with mock agent)
4. Live run: visual criteria get screenshots, interaction criteria get a11y tree + actions
5. No hardcoded API key; missing key fails fast with clear message
6. Low-confidence verdicts (< 0.6) → NeedsReview, not blind Pass/Fail
7. Rate limiter enforces ≤10 RPM for 35b model
8. Every criterion result carries evidence trace (screenshot or action traces)
9. MCP server exposes same tools as native rig tools
10. `cargo clippy -p rgaa-browser-tools -p rgaa-agent` clean, no warnings
