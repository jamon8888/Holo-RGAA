# Production Readiness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Holo3 agentic architecture production-ready by connecting stubs to real implementations, fixing known issues, and adding hardening.

**Architecture:** Wire existing stubs (browser tools, agent evaluation) to real CDP/LLM backends, fix rate limiter semantics, extract shared code, add drift-prevention tests, and harden error handling.

**Tech Stack:** Rust 1.80+, tokio, reqwest, serde, rmcp 3.1.3, rig-core 0.42, rgaa-obscura (CDP), rgaa-core (domain types)

## Global Constraints

- Rust edition 2021, MSRV 1.80
- All crates in workspace: rgaa-rs/Cargo.toml
- rmcp already a workspace dependency (version 3.1.3)
- rig-core already a workspace dependency (version 0.42)
- cargo clippy clean on all modified crates
- All prompts in French for RGAA domain terms
- Follow existing codebase patterns (unit structs, thiserror, structured tracing)

---

## File Structure

| File | Responsibility |
|------|----------------|
| `rgaa-rs/crates/rgaa-agent/src/ratelimit.rs` | Token bucket rate limiter |
| `rgaa-rs/crates/rgaa-agent/src/agent.rs` | RgaaAgent orchestration |
| `rgaa-rs/crates/rgaa-agent/src/models.rs` | ModelRouter dual-model routing |
| `rgaa-rs/crates/rgaa-agent/src/prompts.rs` | Enriched prompt builder |
| `rgaa-rs/crates/rgaa-agent/tests/integration_test.rs` | Agent integration tests |
| `rgaa-rs/crates/rgaa-holo/src/client.rs` | Holo3 HTTP client |
| `rgaa-rs/crates/rgaa-holo/src/prompts.rs` | Holo3 prompt builder |
| `rgaa-rs/crates/rgaa-browser-tools/src/ax_tree.rs` | Accessibility tree types |
| `rgaa-rs/crates/rgaa-browser-tools/src/session.rs` | Browser session |
| `rgaa-rs/crates/rgaa-browser-tools/src/tools/assert_state.rs` | State assertion tool |

---

## Task 1: Fix Rate Limiter Token Refill

**Files:**
- Modify: `rgaa-rs/crates/rgaa-agent/src/ratelimit.rs`
- Test: `rgaa-rs/crates/rgaa-agent/src/ratelimit.rs` (existing test module)

**Interfaces:**
- Consumes: existing `RateLimiter`, `RateLimiterInner`, `ModelTier`
- Produces: same types with smooth refill semantics

- [ ] **Step 1: Read current implementation**

Read `rgaa-rs/crates/rgaa-agent/src/ratelimit.rs` to understand the current bulk-refill logic.

- [ ] **Step 2: Write test for smooth refill**

Add to existing `#[cfg(test)] mod tests`:

```rust
#[tokio::test]
async fn test_smooth_token_refill() {
    let limiter = RateLimiter::new(10, 20);
    for _ in 0..10 {
        limiter.acquire(ModelTier::Tactical).await;
    }
    assert_eq!(limiter.tokens(ModelTier::Tactical), 0);
    tokio::time::sleep(Duration::from_millis(200)).await;
    let tokens = limiter.tokens(ModelTier::Tactical);
    assert!(tokens >= 1 && tokens <= 3, "expected ~2 tokens, got {tokens}");
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p rgaa-agent test_smooth_token_refill`
Expected: FAIL (no `tokens()` method yet)

- [ ] **Step 4: Add `tokens()` accessor**

```rust
pub fn tokens(&self, tier: ModelTier) -> u32 {
    match tier {
        ModelTier::Tactical => self.inner.tactical_tokens.load(Ordering::Acquire),
        ModelTier::Reasoning => self.inner.reasoning_tokens.load(Ordering::Acquire),
    }
}
```

- [ ] **Step 5: Refactor refill to be smooth**

Replace bulk refill in `acquire()` with per-request refill based on elapsed time.

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo test -p rgaa-agent test_smooth_token_refill`
Expected: PASS

- [ ] **Step 7: Run all agent tests**

Run: `cargo test -p rgaa-agent`
Expected: all tests pass

- [ ] **Step 8: Commit**

```bash
git add rgaa-rs/crates/rgaa-agent/src/ratelimit.rs
git commit -m "fix(agent): implement smooth token-per-interval rate limiter refill"
```

---

## Task 2: Make RateLimiterInner Private

**Files:**
- Modify: `rgaa-rs/crates/rgaa-agent/src/ratelimit.rs`

**Interfaces:**
- Consumes: `RateLimiterInner` struct
- Produces: private `RateLimiterInner`

- [ ] **Step 1: Change pub to pub(crate)**

Change `pub struct RateLimiterInner` to `pub(crate) struct RateLimiterInner`.

- [ ] **Step 2: Run tests**

Run: `cargo test -p rgaa-agent`
Expected: all tests pass

- [ ] **Step 3: Commit**

```bash
git add rgaa-rs/crates/rgaa-agent/src/ratelimit.rs
git commit -m "fix(agent): make RateLimiterInner pub(crate)"
```

---

## Task 3: Add Drift-Prevention Test for Criterion Definitions

**Files:**
- Modify: `rgaa-rs/crates/rgaa-agent/tests/integration_test.rs`

**Interfaces:**
- Consumes: `rgaa_agent::criteria_defs::get_criterion_definition`, `rgaa_core::RgaaCriteria::ia_assiste()`
- Produces: test that fails if definitions drift

- [ ] **Step 1: Write drift-prevention test**

```rust
#[test]
fn criteria_defs_match_rgaa_core_catalog() {
    use rgaa_agent::criteria_defs::get_criterion_definition;
    use rgaa_core::RgaaCriteria;

    let ia_criteria = RgaaCriteria::ia_assiste();
    assert_eq!(ia_criteria.len(), 27, "rgaa-core IA_ASSISTE catalog changed");

    for criterion in &ia_criteria {
        let def = get_criterion_definition(&criterion.id)
            .unwrap_or_else(|| panic!("missing definition for criterion {}", criterion.id));

        assert_eq!(def.title, criterion.title, "title mismatch for {}", criterion.id);
        assert_eq!(def.wcag_refs, criterion.wcag_refs, "wcag_refs mismatch for {}", criterion.id);
    }
}
```

- [ ] **Step 2: Run test**

Run: `cargo test -p rgaa-agent --test integration_test criteria_defs_match_rgaa_core_catalog`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add rgaa-rs/crates/rgaa-agent/tests/integration_test.rs
git commit -m "test(agent): add drift-prevention test for criterion definitions"
```

---

## Task 4: Add `#[must_use]` Annotations

**Files:**
- Modify: `rgaa-rs/crates/rgaa-holo/src/client.rs`
- Modify: `rgaa-rs/crates/rgaa-agent/src/agent.rs`
- Modify: `rgaa-rs/crates/rgaa-agent/src/ratelimit.rs`
- Modify: `rgaa-rs/crates/rgaa-agent/src/models.rs`
- Modify: `rgaa-rs/crates/rgaa-browser-tools/src/ax_tree.rs`
- Modify: `rgaa-rs/crates/rgaa-browser-tools/src/session.rs`

**Interfaces:**
- Consumes: all public constructors and important methods
- Produces: `#[must_use]` annotated items

- [ ] **Step 1: Add #[must_use] to HoloClient**

In `client.rs`, add `#[must_use]` to `new()`, `evaluate()`, `evaluate_multimodal()`.

- [ ] **Step 2: Add #[must_use] to agent types**

In `agent.rs`, add `#[must_use]` to `new()`, `build()`, `create_simple_agent()`.

- [ ] **Step 3: Add #[must_use] to rate limiter**

In `ratelimit.rs`, add `#[must_use]` to `new()`, `acquire()`.

- [ ] **Step 4: Add #[must_use] to model router**

In `models.rs`, add `#[must_use]` to `new()`, `new_placeholder()`, `route_for()`.

- [ ] **Step 5: Add #[must_use] to browser types**

In `ax_tree.rs`, add `#[must_use]` to `find_by_ref()`, `focused_element()`.
In `session.rs`, add `#[must_use]` to `new()`, `last_a11y()`.

- [ ] **Step 6: Run clippy**

Run: `cargo clippy -p rgaa-holo -p rgaa-agent -p rgaa-browser-tools -- -D warnings`
Expected: no new warnings

- [ ] **Step 7: Run tests**

Run: `cargo test -p rgaa-holo -p rgaa-agent -p rgaa-browser-tools`
Expected: all tests pass

- [ ] **Step 8: Commit**

```bash
git add rgaa-rs/crates/rgaa-holo/src/client.rs rgaa-rs/crates/rgaa-agent/src/agent.rs rgaa-rs/crates/rgaa-agent/src/ratelimit.rs rgaa-rs/crates/rgaa-agent/src/models.rs rgaa-rs/crates/rgaa-browser-tools/src/ax_tree.rs rgaa-rs/crates/rgaa-browser-tools/src/session.rs
git commit -m "fix: add #[must_use] to public constructors and important methods"
```

---

## Task 5: Remove Dead Code in assert_state.rs

**Files:**
- Modify: `rgaa-rs/crates/rgaa-browser-tools/src/tools/assert_state.rs`

**Interfaces:**
- Consumes: `AssertStateTool` struct
- Produces: clean code without dead variables

- [ ] **Step 1: Read current implementation**

Read `rgaa-rs/crates/rgaa-browser-tools/src/tools/assert_state.rs`.

- [ ] **Step 2: Remove unused _wrapped variable**

Find the `_wrapped` variable and either remove the computation or use the value.

- [ ] **Step 3: Run clippy**

Run: `cargo clippy -p rgaa-browser-tools -- -D warnings`
Expected: no warnings

- [ ] **Step 4: Commit**

```bash
git add rgaa-rs/crates/rgaa-browser-tools/src/tools/assert_state.rs
git commit -m "fix(browser-tools): remove dead code in assert_state.rs"
```

---

## Task 6: Extract Shared Page Context Renderer

**Files:**
- Modify: `rgaa-rs/crates/rgaa-holo/src/prompts.rs` (add public function)
- Modify: `rgaa-rs/crates/rgaa-agent/src/prompts.rs` (use shared function)

**Interfaces:**
- Consumes: `PageContext` type
- Produces: `pub fn format_page_context(ctx: &PageContext) -> String`

- [ ] **Step 1: Read both prompt files**

Read both `rgaa-holo/src/prompts.rs` and `rgaa-agent/src/prompts.rs` to identify duplicated code.

- [ ] **Step 2: Write test for format_page_context**

```rust
#[test]
fn format_page_context_includes_title() {
    let ctx = PageContext {
        title: Some("Test Page".into()),
        lang: Some("fr".into()),
        headings: vec![],
        images: vec![],
        iframes: vec![],
        links: vec![],
        forms: vec![],
        media: vec![],
        navigation: vec![],
    };
    let formatted = format_page_context(&ctx);
    assert!(formatted.contains("Test Page"));
    assert!(formatted.contains("fr"));
}
```

- [ ] **Step 3: Extract format_page_context into rgaa-holo**

Move page context rendering code into a public function in `rgaa-holo/src/prompts.rs`.

- [ ] **Step 4: Update rgaa-agent to use shared function**

Replace duplicated rendering code with `use rgaa_holo::prompts::format_page_context;`.

- [ ] **Step 5: Run tests**

Run: `cargo test -p rgaa-holo -p rgaa-agent`
Expected: all tests pass

- [ ] **Step 6: Commit**

```bash
git add rgaa-rs/crates/rgaa-holo/src/prompts.rs rgaa-rs/crates/rgaa-agent/src/prompts.rs
git commit -m "refactor: extract shared format_page_context into rgaa-holo"
```

---

## Task 7: Add Input Validation for Base64 Images

**Files:**
- Modify: `rgaa-rs/crates/rgaa-holo/src/client.rs`

**Interfaces:**
- Consumes: `evaluate_multimodal()` method
- Produces: validation error on invalid base64

- [ ] **Step 1: Write test for invalid base64**

```rust
#[tokio::test]
async fn test_evaluate_multimodal_invalid_base64() {
    let client = HoloClient::new("test-key".into());
    let result = client.evaluate_multimodal("test prompt", "not-valid-base64!!!", None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("base64"));
}
```

- [ ] **Step 2: Add base64 validation**

Add `base64 = "0.22"` to Cargo.toml. In `evaluate_multimodal()`, validate before sending:

```rust
use base64::Engine;
base64::engine::general_purpose::STANDARD
    .decode(image_base64)
    .map_err(|e| format!("invalid base64 image data: {e}"))?;
```

- [ ] **Step 3: Run test**

Run: `cargo test -p rgaa-holo test_evaluate_multimodal_invalid_base64`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add rgaa-rs/crates/rgaa-holo/src/client.rs rgaa-rs/Cargo.lock
git commit -m "fix(holo): add base64 validation for multimodal image input"
```

---

## Task 8: Add Doc Comments with # Errors

**Files:**
- Modify: `rgaa-rs/crates/rgaa-holo/src/client.rs`
- Modify: `rgaa-rs/crates/rgaa-agent/src/agent.rs`
- Modify: `rgaa-rs/crates/rgaa-agent/src/models.rs`
- Modify: `rgaa-rs/crates/rgaa-agent/src/ratelimit.rs`
- Modify: `rgaa-rs/crates/rgaa-browser-tools/src/session.rs`

**Interfaces:**
- Consumes: all public APIs
- Produces: documented APIs with error conditions

- [ ] **Step 1: Document HoloClient methods**

Add `# Errors`, `# Panics`, `# Examples` sections to `new()`, `evaluate()`, `evaluate_multimodal()`.

- [ ] **Step 2: Document agent methods**

Add `# Arguments`, `# Returns`, `# Errors`, `# Examples` sections to `new()`, `run_ia_assiste()`.

- [ ] **Step 3: Document rate limiter**

Add `# Arguments`, `# Token Budget`, `# Behavior` sections to `new()`, `acquire()`.

- [ ] **Step 4: Document model router**

Add `# Arguments`, `# Routing Rules` sections to `route_for()`.

- [ ] **Step 5: Run clippy**

Run: `cargo clippy -p rgaa-holo -p rgaa-agent -p rgaa-browser-tools -- -D warnings`
Expected: no warnings

- [ ] **Step 6: Commit**

```bash
git add rgaa-rs/crates/rgaa-holo/src/client.rs rgaa-rs/crates/rgaa-agent/src/agent.rs rgaa-rs/crates/rgaa-agent/src/models.rs rgaa-rs/crates/rgaa-agent/src/ratelimit.rs rgaa-rs/crates/rgaa-browser-tools/src/session.rs
git commit -m "docs: add # Errors and # Panics sections to public APIs"
```

---

## Task 9: Add Tests for Helper Methods

**Files:**
- Modify: `rgaa-rs/crates/rgaa-browser-tools/tests/tools_test.rs`

**Interfaces:**
- Consumes: `AXTree::focused_element()`, `AXTree::focusable_elements()`
- Produces: test coverage for untested helpers

- [ ] **Step 1: Write tests for focused_element**

```rust
#[test]
fn test_focused_element_returns_none_when_no_focused() {
    let tree = AXTree { nodes: vec![] };
    assert!(tree.focused_element().is_none());
}

#[test]
fn test_focused_element_returns_node_with_focused_property() {
    let tree = AXTree {
        nodes: vec![
            AXNode {
                backend_node_id: "1".into(),
                role: Some("root".into()),
                name: Some("Root".into()),
                children: vec![],
                properties: vec![],
            },
            AXNode {
                backend_node_id: "2".into(),
                role: Some("button".into()),
                name: Some("Submit".into()),
                children: vec![],
                properties: vec![AXProperty {
                    name: "focused".into(),
                    value: "true".into(),
                }],
            },
        ],
    };
    let focused = tree.focused_element();
    assert!(focused.is_some());
    assert_eq!(focused.unwrap().backend_node_id, "2");
}
```

- [ ] **Step 2: Write tests for focusable_elements**

```rust
#[test]
fn test_focusable_elements_filters_by_role() {
    let tree = AXTree {
        nodes: vec![
            AXNode {
                backend_node_id: "1".into(),
                role: Some("button".into()),
                name: Some("Submit".into()),
                children: vec![],
                properties: vec![],
            },
            AXNode {
                backend_node_id: "2".into(),
                role: Some("img".into()),
                name: Some("Logo".into()),
                children: vec![],
                properties: vec![],
            },
        ],
    };
    let focusable = tree.focusable_elements();
    assert_eq!(focusable.len(), 1);
    assert_eq!(focusable[0].backend_node_id, "1");
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p rgaa-browser-tools`
Expected: all tests pass

- [ ] **Step 4: Commit**

```bash
git add rgaa-rs/crates/rgaa-browser-tools/tests/tools_test.rs
git commit -m "test(browser-tools): add tests for focused_element and focusable_elements"
```

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-08-21-production-readiness.md`. Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?
