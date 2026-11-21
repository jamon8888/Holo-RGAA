---
name: guided-test
description: Run bounded, reproducible manual accessibility tests — keyboard navigation, focus management, contrast, touch targets, reading order — and capture screenshots and AXTree evidence. Use when criteria classified Manuel or IaAssiste need human verification, or the user asks for a keyboard, focus or contrast test.
---

# guided-test — RGAA Manual Testing Protocol

## Purpose

Execute bounded, reproducible guided accessibility tests for criteria that require human judgment: keyboard navigation, focus management, color contrast verification, touch target sizing, and reading order. Produces structured evidence with screenshots and AXTree captures.

**Key principle: Guided tests are reproducible — same steps produce same results.**

> **Méthode officielle par test** : pour la méthode de vérification officielle du RGAA d'un test (sa clé, ex. `7.3.1`), appeler l'outil MCP `get_test_methodology` avec la clé du test. Il retourne la méthode officielle tirée de `RGAA/methodologies.json` (DISIC/accessibilite.numerique.gouv.fr, commit épinglé). Cette skill décrit le protocole de test guidé de Holo-RGAA ; elle ne réécrit pas la méthode officielle de mémoire.

## When This Skill Activates

- User asks "run keyboard navigation test", "check focus visibility", "verify contrast"
- Audit reveals `Manuel` or `IaAssiste` criteria that need manual verification
- User needs to complete manual testing portion of RGAA audit
- Regression testing for keyboard/accessibility functionality

## Supported Test Types

| Test | Criteria | What It Checks |
|------|----------|----------------|
| `keyboard-navigation` | 7.3, 12.8, 12.9 | Tab through page: every script control reachable and operable (7.3), coherent tab order (12.8), no keyboard trap (12.9) |
| `focus-visibility` | 10.7 | Focus indicator visible on every element receiving focus |
| `color-contrast` | 3.2, 3.3 | Text contrast ≥ 4.5:1, or ≥ 3:1 for large text (3.2); interface components and informative graphics ≥ 3:1 (3.3). Measured from computed colours and pixels, never estimated by vision |
| `touch-targets` | — (not an RGAA 4.1.2 criterion) | Target size is WCAG 2.2 (2.5.8). Report it as an *additional observation*, never as an RGAA non-conformity |
| `reading-order` | 10.3, 12.8 | Content understandable in DOM order with styles off (10.3); tab order coherent with visual order (12.8) |
| `forms-manual` | 11.1–11.13 | Labels, errors, autocomplete attributes |

## Test Definition Format

```yaml
id: keyboard-navigation
version: 1
preconditions:
  - Browser at target URL
steps:
  - navigate: https://example.test
  - accessibility_tree
  - press_key: Tab
  - assert_state:
      expected:
        focus_visible: true
        focus_order: 1
  - press_key: Tab
  - assert_state:
      expected:
        focus_order: 2
criterion_mapping:
  - 7.3
  - 12.8
  - 12.9
evidence_requirements:
  - screenshot-on-focus
  - axtree-on-step
```

## Step Types

| Step | Description |
|------|-------------|
| `navigate` | Navigate to URL |
| `accessibility_tree` | Capture AXTree snapshot |
| `press_key` | Press keyboard key (Tab, Shift+Tab, Enter, Space, Escape, Arrow keys) |
| `click_ref` | Click element by AXTree reference |
| `fill_ref` | Fill input by AXTree reference |
| `screenshot` | Capture screenshot |
| `assert_state` | Assert expected browser state |

## Test Response

```json
{
  "test_id": "keyboard-navigation",
  "outcome": "completed",
  "issues": [],
  "terminated_reason": "completed",
  "completed_steps": 6,
  "evidence": [
    {
      "kind": "screenshot",
      "path": "/evidence/keyboard-nav-001.png",
      "sha256": "abc123..."
    },
    {
      "kind": "accessibility_tree",
      "path": "/evidence/axtree-001.json",
      "sha256": "def456..."
    }
  ],
  "manual_review_required": false
}
```

## Termination Reasons

| Reason | Meaning |
|--------|---------|
| `completed` | All steps executed successfully |
| `missing_reference` | Referenced element not found in AXTree |
| `assertion_failed` | Expected state did not match |
| `keyboard_trap` | Keyboard trap detected (focus cannot leave area) |
| `timeout` | Step exceeded time limit |
| `navigation_error` | Navigation failed |
| `execution_error` | Step execution failed |

## Keyboard Navigation Test (Example)

```
Test: keyboard-navigation
Steps:
  1. Navigate to URL
  2. Capture AXTree
  3. Press Tab — verify focus moves to first focusable
  4. Press Tab — verify focus moves to second focusable
  5. Press Shift+Tab — verify focus moves back
  6. Screenshot each focus state

Expected:
  - Focus visible on each element
  - Logical focus order (skip links → navigation → main content)
  - No keyboard traps
```

## Color Contrast Test (Manual Verification Required)

```
Test: color-contrast
Steps:
  1. Navigate to URL
  2. Screenshot of text elements
  3. Claude evaluates contrast ratios from screenshot

Manual verification needed for:
  - Dynamic color changes
  - User-overridden styles
  - Browser zoom > 200%
```

## Evidence Collection

| Evidence Type | Captured By | Use |
|---------------|-------------|-----|
| Screenshots | `screenshot` step | Visual verification |
| AXTree | `accessibility_tree` step | Structure verification |
| Action trace | All steps | Audit trail |
| Focus state | `assert_state` step | Proof of focus |

## Creating Custom Tests

Define in `.rgaa/config.yaml`:

```yaml
guided_tests:
  - keyboard-navigation
  - focus-visibility
  - my-custom-test
```

Test file `.rgaa/tests/my-custom-test.yaml`:

```yaml
id: my-custom-test
version: 1
steps:
  - navigate: https://example.test
  - accessibility_tree
  - click_ref: menu-button
  - assert_state:
      expected:
        dialog_open: true
  - screenshot
criterion_mapping:
  - 7.1
```

## Example Interactions

```
User: "Run a keyboard navigation test on the checkout flow"
Claude: → Loads keyboard-navigation test definition
Claude: → Executes steps against checkout URL
Claude: → Captures screenshots at each focus change
Claude: → Reports: "All 12 focusable elements reached in logical order. No keyboard traps detected."

User: "Check color contrast on the contact form"
Claude: → Loads color-contrast test
Claude: → Navigates to form
Claude: → Runs the contrast measurement (computed colours; pixel sampling over images and gradients)
Claude: → Highlights each failing element in the page and shows ratio, threshold and crop
Claude: → Reports failures against RGAA 3.2 / 3.3 — the ratio comes from the measurement, never from Claude's vision
```

## Limitations

Guided tests cannot verify:
- Actual screen reader experience (requires a real screen reader: the RGAA reference environment, e.g. NVDA + Firefox, VoiceOver + Safari). Computer use sees the screen but does not hear speech output: never conclude on restitution (7.1, 7.5, accessible names) from screenshots alone
- Browser-specific assistive technology behavior
- User-installed browser extensions
- Touch device behavior on non-touch environments

For these, manual testing with actual assistive technology is required.

## Related Skills

- `audit` — Identifies which criteria need manual testing
- `triage` — Routes to guided-test when manual verification needed
- `verify` — Combines with automated tests for complete verification
