# RGAA MCP Server Reference

The `rgaa-mcp` binary provides a Model Context Protocol (MCP) server that exposes RGAA accessibility auditing tools for AI assistants like Claude Code.

## Installation

```bash
cargo install --path crates/rgaa-mcp
```

## Configuration

### Claude Code

Add to your `mcp.json`:

```json
{
  "mcpServers": {
    "rgaa": {
      "command": "rgaa-mcp",
      "env": {
        "RGAA_OBSCURA_BIN": "/path/to/obscura"
      }
    }
  }
}
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RGAA_OBSCURA_BIN` | Path to Obscura browser binary | Required |

## Tools

| Tool | Purpose |
|------|---------|
| `analyze` | Analyze a URL for accessibility findings |
| `remediate` | Generate remediation guidance for issues |
| `igt` | Run a guided accessibility test |
| `audit_url` | Run a full RGAA audit on a URL |
| `get_audit_result` | Retrieve a stored audit by ID |
| `list_criteria` | List all 106 RGAA criteria |
| `verify_fix` | Re-verify corrected files against a reference audit |

---

### `analyze`

Analyze a single URL for accessibility findings.

**Parameters:**

```json
{
  "url": "https://example.test",
  "config": {
    "profile": "default",
    "viewport_width": 1000,
    "viewport_height": 1080,
    "screenshot_policy": "none",
    "timeout_ms": 30000
  }
}
```

**Response:**

```json
{
  "url": "https://example.test",
  "findings": [
    {
      "id": "finding-001",
      "rule": "image-alt",
      "criterion_id": "1.1",
      "status": "fail",
      "description": "Image missing alt attribute",
      "html": "<img src=\"hero.png\">"
    }
  ],
  "completed": true,
  "duration_ms": 5234
}
```

---

### `remediate`

Generate remediation guidance for accessibility issues.

**Parameters:**

```json
{
  "issues": [
    {
      "id": "issue-001",
      "rule": "image-alt",
      "element_html": "<img src=\"hero.png\">",
      "page_url": "https://example.test",
      "source_locations": [
        { "file": "src/components/Hero.tsx", "line": 42 }
      ],
      "summary": "Image missing alt attribute",
      "criteria": ["1.1"],
      "framework": "react"
    }
  ]
}
```

**Constraints:**
- Batch size: 1-25 issues per request

**Response:**

```json
{
  "outcomes": [
    {
      "outcome": "ok",
      "issue_id": "issue-001",
      "explanation": "Add alt attribute with descriptive text",
      "steps": [
        "Locate the img element in src/components/Hero.tsx:42",
        "Add alt=\"Hero image showing team collaboration\" to the img tag"
      ],
      "confidence": "high",
      "proposal": {
        "diff": "--- a/src/components/Hero.tsx\n+++ b/src/components/Hero.tsx\n@@ -40 +40 @@\n-<img src=\"hero.png\">\n+<img src=\"hero.png\" alt=\"Hero image...\">",
        "approval_state": { "kind": "required" }
      }
    }
  ]
}
```

---

### `igt`

Run a guided accessibility test.

**Parameters:**

```json
{
  "test": {
    "id": "keyboard-navigation",
    "version": 1,
    "steps": [
      { "kind": "navigate", "url": "https://example.test" },
      { "kind": "accessibility_tree" },
      { "kind": "press_key", "key": "Tab" },
      { "kind": "screenshot" }
    ],
    "criterion_mapping": ["12.1", "12.2"]
  }
}
```

**Step Types:**

| Kind | Description |
|------|-------------|
| `navigate` | Navigate to URL |
| `accessibility_tree` | Capture AXTree snapshot |
| `press_key` | Press keyboard key |
| `screenshot` | Capture screenshot |
| `assert_state` | Assert expected state |

**Response:**

```json
{
  "issues": [],
  "terminated_reason": "completed",
  "completed_steps": 4,
  "evidence": [
    { "kind": "screenshot", "path": "/evidence/xxx.png", "sha256": "abc123" }
  ],
  "manual_review_required": false
}
```

---

### `audit_url`

Run a full RGAA audit using the orchestrator pipeline.

**Parameters:**

```json
{
  "url": "https://example.test",
  "config": {
    "max_pages": 50,
    "max_depth": 5
  }
}
```

**Response:**

```json
{
  "audit_id": "aud_abc123xyz",
  "taux_global": 85.5,
  "etat_conformite": "partielle"
}
```

---

### `get_audit_result`

Retrieve a previously run audit by ID.

**Parameters:**

```json
{
  "audit_id": "aud_abc123xyz"
}
```

**Response:**

```json
{
  "audit_id": "aud_abc123xyz",
  "url": "https://example.test",
  "taux_global": 85.5,
  "passed": 45,
  "failed": 8,
  "na": 53
}
```

---

### `list_criteria`

List all 106 RGAA criteria.

**Parameters:** None

**Response:**

```json
{
  "criteria": [
    { "id": "1.1", "title": "Each image has an alternative", "classification": "Deterministe" },
    { "id": "1.3", "title": "Complex images have a detailed description", "classification": "IaAssiste" }
  ]
}
```

**Classifications:**

| Classification | Description |
|---------------|-------------|
| `Deterministe` | Automatically testable (axe-core + gap-fix) |
| `IaAssiste` | Requires LLM-assisted evaluation |
| `Manuel` | Manual testing required |

---

### `verify_fix`

Re-verify corrected files against the audit they were fixed against. The tool
re-runs the same analysis `analyze` uses on each corrected file's page and
diffs the result against the reference with the shared baseline comparison
(`rgaa_remediation::compare`) — there is no second re-audit engine.

**Parameters:**

| Field | Type | Description |
|-------|------|-------------|
| `reference_audit` | object | The `AuditBundle` the fixes were written against, as `audit_url` / `get_audit_result` emit one |
| `files` | array | 1–25 `{ "path", "url" }` entries; `url` is the page re-analysed for that file |
| `per_file_timeout_ms` | number | Optional, clamped to 30000 |

**Response:**

```json
{
  "fixed": [{ "finding": { "id": "f-alt", "...": "..." }, "citations": [] }],
  "remaining": [],
  "new": [],
  "unverified": [],
  "files": [{ "path": "src/Hero.tsx", "url": "https://example.test", "status": "verified", "message": null }]
}
```

`citations` carries the sources behind the reference verdict for that
finding's criterion, so a RAG-backed verdict keeps its evidence.

`unverified` holds reference findings on a page that could not be
re-analysed (`timed_out` / `failed`). They are kept out of `fixed` on
purpose: a page nobody re-scanned has not been shown to be remediated.

---

## Error Handling

| Code | Description |
|------|-------------|
| `INVALID_INPUT` | Request parameters are invalid |
| `POLICY_DENIED` | Action violates policy |
| `UNSUPPORTED_CONFIGURATION` | Configuration not supported |
| `EXECUTION_FAILED` | Execution failed |
| `INCOMPLETE_RESULT` | Result is incomplete |
| `EMPTY_RESULT` | No results returned |

## Claude Code Usage Examples

```javascript
// Analyze a URL
const analysis = await mcp.callTool('analyze', {
  url: 'https://example.test',
  config: { screenshot_policy: 'on_failure' }
});

// Generate remediation
const remediation = await mcp.callTool('remediate', {
  issues: [{ id: 'img-001', rule: 'image-alt', ... }]
});

// Run full audit
const audit = await mcp.callTool('audit_url', {
  url: 'https://example.test'
});

// List all criteria
const criteria = await mcp.callTool('list_criteria', {});
```

## Browser Requirements

Requires the Obscura browser binary:

```bash
curl -L -o obscura https://github.com/example/obscura/releases/latest
chmod +x obscura
export RGAA_OBSCURA_BIN=/path/to/obscura
```
