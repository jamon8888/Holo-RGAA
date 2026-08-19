---
name: guided-test
description: "Run bounded, reproducible intelligent guided accessibility tests"
version: 0.1.0
author: RGAA Team
requires:
  - rgaa-mcp
  - rgaa-obscura
mode-default: suggest
---

# guided-test — Intelligent Guided Accessibility Tests

## Overview
This skill executes versioned, bounded guided accessibility tests via the `rgaa-mcp` tool `igt`. It drives a browser through a structured sequence of actions and assertions, capturing PNG evidence and accessibility tree references.

## Inputs
- `test_id` (string, required) — Guided test identifier (must exist in config.guided_tests).
- `version` (integer, default: 1) — Test version for reproducibility.
- `preconditions` (array<string>, optional) — Prerequisites before test execution.
- `steps` (array<object>, required) — Sequence of guided actions.
- `criterion_mapping` (array<string>, optional) — RGAA criterion IDs this test covers.
- `evidence_requirements` (array<string>, optional) — Required evidence kinds (e.g., "screenshot", "tree").

## Step Types
- `navigate` — `{"kind": "navigate", "url": "..."}`
- `accessibilityTree` — `{"kind": "accessibilityTree"}` (captures AX tree)
- `pressKey` — `{"kind": "pressKey", "key": "Tab"}`
- `clickRef` — `{"kind": "clickRef", "reference": "ax:123"}`
- `fillRef` — `{"kind": "fillRef", "reference": "ax:123", "value": "text"}`
- `screenshot` — `{"kind": "screenshot"}` (PNG evidence)
- `assertState` — `{"kind": "assertState", "expected": {...}}` (observed state match)

## Workflow
1. **Resolve test** — Load test definition by ID/version.
2. **Validate steps** — Mutating actions (navigate, click, fill, press) MUST be followed by observation/assertion.
3. **Execute via `igt`** — Call `rgaa-mcp` tool `igt` with `GuidedTestRequest`.
4. **Capture evidence** — PNG screenshots, DOM snapshots, AX tree refs (`ax:<backendNodeId>`).
5. **Evaluate assertions** — Observed state matched against expected (subset match).
6. **Produce result** — `GuidedRunResult` with issues, unanalyzed elements, termination reason, completed steps, evidence, manual_review_required.

## Outputs
- `GuidedRunResult` with:
  - `issues`: string descriptions of failures.
  - `unanalyzed_elements`: targets not evaluated.
  - `terminated_reason`: Completed, MissingReference, AssertionFailed, KeyboardTrap, Timeout, NavigationError, ExecutionError, InvalidOrdering.
  - `completed_steps`: count of successful steps.
  - `evidence`: `EvidenceRef` (kind, path, sha256).
  - `manual_review_required`: boolean.

## Constraints
- Steps are bounded (max 3 retries per step, timeout 30s).
- Termination reason preserved even on failure.
- Evidence MUST be PNG for screenshots, valid AX refs (`ax:<number>` or `ax-role=...;name=...`).
- Keyboard trap detection on Tab press.
- Invalid step ordering → `InvalidOrdering` termination.

## Failure Modes
- Unknown test ID → exit code 2 (`INVALID_INPUT`).
- Browser unavailable → exit code 3 (`UNSUPPORTED_CONFIGURATION`).
- Assertion failed → `AssertionFailed` termination, evidence captured.