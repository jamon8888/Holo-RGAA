# `/run-igt` — Run Guided Accessibility Test

## Description

Execute a structured guided accessibility test (IGT) for criteria requiring human judgment: keyboard navigation, focus management, color contrast, touch targets. The test runs step-by-step through the browser via CDP.

## Prerequisites

- `rgaa-mcp` MCP server connected (recommended), OR
- `rgaa-cli` installed with `rgaa audit igt` subcommand

## Usage

```
/run-igt [--test <name>] [--url <url>] [--precondition <description>]
```

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `--test` | string | Yes* | Named test: `keyboard-navigation`, `focus-visibility`, `color-contrast`, `touch-targets`, `reading-order`, `forms-manual` (*or use `--url` for a one-shot test) |
| `--url` | string | Yes* | Target URL for a one-shot test (*or use `--test` for a named test) |
| `--precondition` | string | No | Precondition description (e.g., "user is logged in") |

## Supported Test Types

| Test | Criteria | What It Checks |
|------|----------|----------------|
| `keyboard-navigation` | 12.1, 12.2 | Tab through page, all focusables reachable, no traps |
| `focus-visibility` | 12.1 | Focus indicators visible on all interactive elements |
| `color-contrast` | 3.2, 3.3 | Text contrast meets 4.5:1 (normal) or 3:1 (large) |
| `touch-targets` | 12.5 | Touch targets at least 44×44px |
| `reading-order` | 9.1, 10.3 | Logical reading order in AXTree |
| `forms-manual` | 11.1–11.13 | Labels, errors, autocomplete attributes |

## Examples

```
/run-igt --test keyboard-navigation --url https://example.test
/run-igt --test focus-visibility --url https://example.test --precondition "user is logged in"
/run-igt --test color-contrast --url https://example.test/contact
```

## Test Steps

Each test runs through its defined steps:
1. Navigate to URL
2. Capture initial AXTree
3. Execute action (press key, click element, fill input)
4. Assert expected state
5. Capture evidence (screenshot, AXTree) at each step

## Output

```
IGT: keyboard-navigation
URL: https://example.test
──────────────────────────────────
Status:     completed
Steps:      12 completed
Issues:     0 keyboard traps detected
──────────────────────────────────
Focus Order:
  1. [skip link] → visible ✓
  2. [navigation] → visible ✓
  3. [search input] → visible ✓
  ...
──────────────────────────────────
Evidence: 6 screenshots, 6 AXTree dumps
Manual Review Required: no
```

## Termination Reasons

| Reason | Meaning |
|--------|---------|
| `completed` | All steps executed successfully |
| `keyboard_trap` | Focus cannot escape an area |
| `assertion_failed` | Expected state did not match |
| `timeout` | Step exceeded time limit |
| `missing_reference` | Referenced element not found |
| `navigation_error` | Navigation failed |

## Evidence

Evidence is saved to the evidence directory with SHA256 fingerprints:
- Screenshots: `igt-keyboard-nav-001.png`
- AXTree dumps: `igt-keyboard-nav-axtree-001.json`
