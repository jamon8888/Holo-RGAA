# Guided Test Runbook

This runbook explains how to run guided accessibility tests (IGT) for detailed manual testing with automation assistance.

## What is IGT?

Intelligent Guided Testing (IGT) combines browser automation with structured test steps to perform reproducible accessibility tests that require human judgment.

Unlike automated scans, IGT can:

- Verify keyboard navigation flow
- Test focus management
- Validate color contrast manually
- Check touch/tap targets
- Verify reading order

## Running a Guided Test

### CLI

```bash
rgaa audit igt --test keyboard-navigation
```

### MCP

```javascript
const result = await mcp.callTool('igt', {
  test: {
    id: 'keyboard-navigation',
    version: 1,
    preconditions: ['Browser opened at URL'],
    steps: [
      { kind: 'navigate', url: 'https://example.test' },
      { kind: 'accessibility_tree' },
      { kind: 'press_key', key: 'Tab' },
      { kind: 'screenshot' }
    ],
    criterion_mapping: ['12.1', '12.2'],
    evidence_requirements: ['screenshot-on-focus']
  }
});
```

## Predefined Tests

Tests are configured in `.rgaa/config.yaml`:

```yaml
guided_tests:
  - keyboard-navigation
  - focus-visibility
  - color-contrast
  - touch-targets
```

### Test: `keyboard-navigation`

```yaml
id: keyboard-navigation
version: 1
preconditions:
  - Browser at target URL
steps:
  - navigate: https://example.test
  - accessibility_tree
  - press_key: Tab
  - screenshot
  - assert_state:
      expected:
        focus_visible: true
criterion_mapping:
  - 12.1
  - 12.2
```

### Test: `focus-visibility`

```yaml
id: focus-visibility
version: 1
steps:
  - navigate: https://example.test
  - accessibility_tree
  - press_key: Tab
  - press_key: Tab
  - press_key: Tab
  - screenshot
  - assert_state:
      expected:
        focus_order: 3
criterion_mapping:
  - 12.1
```

### Test: `color-contrast`

```yaml
id: color-contrast
version: 1
steps:
  - navigate: https://example.test
  - accessibility_tree
  - screenshot
  - assert_state:
      expected:
        visible_text: true
criterion_mapping:
  - 3.2
  - 3.3
```

## Understanding the Response

```json
{
  "issues": [],
  "unanalyzed_elements": [],
  "terminated_reason": "completed",
  "completed_steps": 5,
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

## Step Types

### `navigate`

Navigate to a URL:

```json
{ "kind": "navigate", "url": "https://example.test" }
```

### `accessibility_tree`

Capture the accessibility tree:

```json
{ "kind": "accessibility_tree" }
```

Returns AXTree snapshot for review.

### `press_key`

Press a keyboard key:

```json
{ "kind": "press_key", "key": "Tab" }
```

**Common Keys:**

- `Tab` - Move forward through focusables
- `Shift+Tab` - Move backward
- `Enter` - Activate link/button
- `Space` - Activate button/toggle
- `Escape` - Close modal/menu
- Arrow keys - Navigate within widget

### `click_ref`

Click an element by reference:

```json
{ "kind": "click_ref", "reference": "button-submit" }
```

### `fill_ref`

Fill an input by reference:

```json
{ "kind": "fill_ref", "reference": "input-search", "value": "search term" }
```

### `screenshot`

Capture a screenshot:

```json
{ "kind": "screenshot" }
```

### `assert_state`

Assert expected state:

```json
{
  "kind": "assert_state",
  "expected": {
    "focus_visible": true,
    "focus_element": "button-submit"
  }
}
```

## Termination Reasons

| Reason | Meaning |
|--------|---------|
| `completed` | All steps executed successfully |
| `missing_reference` | Referenced element not found |
| `assertion_failed` | State assertion didn't match |
| `keyboard_trap` | Keyboard trap detected |
| `timeout` | Step took too long |
| `navigation_error` | Navigation failed |
| `execution_error` | Step execution failed |

## Creating Custom Tests

### 1. Define in Config

```yaml
# .rgaa/config.yaml
guided_tests:
  - keyboard-navigation
  - my-custom-test
```

### 2. Write Test Definition

Tests can be defined in a separate file:

```yaml
# .rgaa/tests/my-custom-test.yaml
id: my-custom-test
version: 1
preconditions:
  - User authenticated
steps:
  - navigate: https://example.test/dashboard
  - accessibility_tree
  - click_ref: menu-settings
  - assert_state:
      expected:
        visible: settings_panel
  - screenshot
criterion_mapping:
  - 12.1
  - 12.2
evidence_requirements:
  - screenshot-on-state-change
```

### 3. Run Custom Test

```bash
rgaa audit igt --test my-custom-test --config .rgaa/config.yaml
```

## Evidence Collection

### Automatic Evidence

Evidence is automatically captured based on `evidence_requirements`:

- `screenshot` - Always captures screenshots
- `screenshot-on-focus` - Screenshot when focus changes
- `screenshot-on-state-change` - Screenshot when state changes
- `axtree` - Accessibility tree dumps

### Manual Evidence

For manual review:

1. Run test with screenshot step
2. Review evidence in output directory
3. Complete manual checks not automatable

## Integration with Audit

### Pre-Audit Validation

Use IGT to validate critical paths before full audit:

```bash
# Test critical user flows
rgaa audit igt --test checkout-flow
rgaa audit igt --test navigation-menu
rgaa audit igt --test form-submission
```

### Post-Audit Verification

Use IGT to manually verify failed criteria:

```bash
# After finding contrast issues
rgaa audit igt --test color-contrast --url https://example.test/problem-page
```

## Troubleshooting

### Test Not Found

```
Error: unknown guided test 'test-name'
```

Solution: Add to `guided_tests` in config.

### Element Not Found

```
Termination: missing_reference
```

The referenced element wasn't found. Check:
- Element exists on page
- Correct reference name
- Element is visible/available

### Assertion Failed

```
Termination: assertion_failed
Expected: focus_visible: true
Actual: focus_visible: false
```

The page state didn't match expectations. This may indicate:
- Focus indicator not visible
- Wrong element focused
- Page loaded incorrectly

## Best Practices

1. **Test critical paths first** - Focus on user journeys
2. **Capture evidence** - Screenshots help with debugging
3. **Use accessibility tree** - Verify structure before manual test
4. **Check termination reasons** - Understand why test stopped
5. **Combine with automated scans** - IGT complements automated testing
