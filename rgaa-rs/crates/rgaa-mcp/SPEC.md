# rgaa-mcp Specification: Parity with deque axe-mcp-server

## Context

The rgaa-rs MCP server provides accessibility auditing via three tools: `analyze`, `remediate`, and `igt`. The implementation partially overlaps with deque's axe-mcp-server but diverges in several important ways. This spec defines the target state that reproduces deque's complete feature set while maintaining rgaa-rs's RGAA-centric domain model.

## Overview

The MCP server is a facade over:
- **ObscuraBridge**: Browser automation via CDP (playwright-style interactions)
- **RemediationService**: AI-powered fix guidance
- **Orchestrator**: Full audit pipeline (crawl + analyze + report)

## Tool Specifications

### 1. `analyze` Tool

Performs comprehensive accessibility analysis on web pages by running a scan through the Obscura CDP bridge in a real browser environment.

#### Parameters

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `url` | `string` | Yes | — | Target URL (local or remote) |
| `config` | `object` | No | see below | Scan configuration |

##### Config Object

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `profile` | `string` | No | `"default"` | Accessibility testing profile |
| `viewport_width` | `u32` | No | `1000` | Viewport width in pixels (1-7680) |
| `viewport_height` | `u32` | No | `1080` | Viewport height in pixels (1-7680) |
| `selector` | `string \| string[]` | No | `None` | CSS selector or array for iframe/shadow-DOM targeting |
| `pre_scan_actions` | `PreScanAction[]` | No | `[]` | Interactions before scan |
| `cookies` | `CookieInput[]` | No | `[]` | Cookies to set before navigation |
| `screenshot` | `ScreenshotInput` | No | `None` | Screenshot capture options |
| `advanced_rules` | `string \| null` | No | `null` | Override advanced rules preset |
| `igt_tools` | `string[] \| null` | No | `null` | IGTs to run after scan (e.g., `["keyboard"]`) |
| `timeout_ms` | `u64 \| null` | No | `30000` | Per-step timeout in ms |
| `retry_limit` | `u8 \| null` | No | `0` | Retry attempts on failure |

#### Pre-Scan Actions (`before`)

Supported actions executed **after page load** but **before accessibility scan**:

| Action | Required Fields | Optional Fields | Description |
|--------|-----------------|-----------------|-------------|
| `click` | `selector` | — | Click element matching CSS selector |
| `fill` | `selector`, `value` | — | Fill input with value (sensitive - redacted) |
| `waitFor` | `selector` | `state` | Wait for element state (default: `"visible"`) |

**State values**: `"visible"` | `"attached"` | `"hidden"` | `"detached"`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PreScanActionInput {
    Click { selector: String },
    Fill { selector: String, value: String }, // value is sensitive
    WaitFor { selector: String, #[serde(default)] state: WaitForState },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum WaitForState {
    #[default]
    Visible,
    Attached,
    Hidden,
    Detached,
}
```

**Limits**:
- Maximum 20 steps per scan
- Each step timeout: `BROWSER_TIMEOUT_MS` (default 30,000 ms)

#### Cookie Injection (`cookies`)

Cookies set on browser context **before navigation** — they ride the very first request.

| Field | Required | Type | Description |
|-------|----------|------|-------------|
| `name` | Yes | `string` | Cookie name (appears in logs - no secrets here) |
| `value` | Yes | `string` | Cookie value (sensitive - redacted) |
| `domain` | Yes | `string` | Cookie domain (leading dot for subdomains) |
| `path` | No | `string` | Cookie path (default: `/`) |
| `same_site` | No | `string` | `"Strict"` \| `"Lax"` \| `"None"` |
| `secure` | No | `bool` | Secure flag (default: false) |
| `http_only` | No | `bool` | HTTPOnly flag (default: false) |
| `expires` | No | `i64` | Unix timestamp expiry (omit for session) |

**Limits**:
- Maximum 20 cookies per scan

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CookieInput {
    pub name: String,
    pub value: String, // sensitive - redacted in logs
    pub domain: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub same_site: Option<SameSiteInput>,
    #[serde(default)]
    pub r#secure: Option<bool>,
    #[serde(default)]
    pub http_only: Option<bool>,
    #[serde(default)]
    pub expires: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SameSiteInput {
    Strict,
    Lax,
    None,
}
```

#### Screenshot Options

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct ScreenshotInput {
    #[serde(default)]
    pub format: Option<ScreenshotFormat>, // "png" | "jpeg"
    #[serde(default)]
    pub save_to: Option<String>,          // absolute path
    #[serde(default)]
    pub save: Option<bool>,              // auto-generate filename
    #[serde(default)]
    pub inline: Option<bool>,             // return as MCP image block (default: true)
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ScreenshotFormat {
    Png,
    Jpeg,
}
```

**Behavior**:
- Returns image as MCP image content block (when `inline: true`)
- Writes to `save_to` path or `AXE_SCREENSHOT_DIR` (default: OS temp)
- Screenshots are best-effort — scan never fails due to screenshot failure
- On scan failure after screenshot capture, image returned with error

#### Advanced Rules

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AdvancedRulesInput {
    pub value: String,  // "thorough", "standard", "disabled"
    pub source: String, // "tool_arg" | "config"
}
```

#### Intelligent Guided Tests (IGT)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IgtToolsInput {
    pub tools: Vec<String>, // ["keyboard"]
}
```

When `igtTools` is set, the response shape changes:

```json
{
  "url": "http://localhost:3000",
  "data": {
    "axe": [...],  // axe-core violations
    "igt": {
      "keyboard": {
        "status": "complete",
        "issues": [...],
        "igtElements": [...],
        "terminatedReason": "keyboard-trap"
      }
    }
  }
}
```

Without `igtTools`, `data` is the flat axe issues array (backward compatible).

### 2. `remediate` Tool

Generates AI-powered remediation guidance for accessibility issues.

#### Parameters

| Field | Required | Type | Description |
|-------|----------|------|-------------|
| `issues` | Yes | `RemediationIssueInput[]` | 1-25 issues from analyze/igt |

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RemediationIssueInput {
    pub id: String,          // caller-chosen unique identifier
    pub rule: String,        // axe rule ID
    pub element_html: String,// violating HTML snippet
    pub remediation: String, // description of what's wrong
    #[serde(default)]
    pub page_url: Option<String>, // optional (deque: required, we make optional)
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub source_locations: Vec<SourceLocationInput>,
    #[serde(default)]
    pub criteria: Vec<String>,
    #[serde(default)]
    pub framework: Option<FrameworkInput>,
}
```

#### Response

```json
{
  "outcomes": [
    {
      "outcome": "ok",
      "issue_id": "color-contrast-0",
      "explanation": "...",
      "steps": ["...", "..."],
      "confidence": "high",
      "criteria": ["10.1.1", "10.3.4"],
      "proposal": { ... }
    },
    {
      "outcome": "error",
      "issue_id": "image-alt-1",
      "code": "LLM_ERROR",
      "message": "..."
    }
  ]
}
```

### 3. `igt` Tool (Deprecated)

**Status**: Deprecated. Use `analyze` with `igtTools` instead.

The standalone `igt` tool remains functional for backward compatibility but emits a deprecation notice in response metadata.

### 4. `audit_url` Tool

Runs a full RGAA audit on a URL using the orchestrator pipeline.

#### Parameters

| Field | Required | Type | Description |
|-------|----------|------|-------------|
| `url` | Yes | `string` | Target URL |
| `config` | No | `CrawlConfigInput` | Crawl configuration |

### 5. `get_audit_result` Tool

Retrieves a previously run audit by ID.

### 6. `list_criteria` Tool

Lists all 106 RGAA criteria with IDs, titles, and classifications.

## Security: Secret Redaction

Sensitive values are redacted from logs and error messages:

| Parameter | Redacted |
|-----------|----------|
| `fill.value` | ✅ Always |
| `cookies[].value` | ✅ Always |
| `cookies[].name` | ❌ Appears in logs (no secrets here) |

```rust
const SECRET_KEYS: &[&str] = &[
    "password", "passwd", "pwd", "secret", "token", "cookie",
    "authorization", "api_key", "apikey", "api-key",
    "access_key", "access_token", "client_secret", "session",
];

pub(crate) fn redact(input: &str) -> String {
    // 1. Redact URL userinfo (user:pass@host)
    // 2. Redact key=value pairs where key matches SECRET_KEYS
    // 3. Handle bearer/basic auth schemes
}
```

## Error Codes

| Code | HTTP Status | Use Case |
|------|-------------|----------|
| `INVALID_INPUT` | 400 | Malformed request, validation failures |
| `POLICY_DENIED` | 403 | Policy violation (auth, scope) |
| `UNSUPPORTED_CONFIGURATION` | 422 | Unsupported feature/option |
| `EXECUTION_FAILED` | 500 | Browser/CDP errors |
| `INCOMPLETE_RESULT` | 500 | Partial results with errors |
| `EMPTY_RESULT` | 422 | Empty issue set (not an error per se) |

## Response Envelope

### AnalyzeResponse

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeResponse {
    pub url: String,
    pub findings: Vec<FindingDto>,
    pub evidence: Vec<EvidenceRefDto>,
    pub errors: Vec<PageErrorDto>,
    pub completed: bool,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advanced_rules: Option<AdvancedRulesInput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<String>>,
    // When igtTools is set:
    #[serde(skip_serializing_if = "Option::is_none")]
    pub igt_results: Option<IgtResultsDto>,
}
```

### IgtResultsDto (nested under `data.igt` when igtTools is set)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IgtResultsDto {
    pub keyboard: IgtResultDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IgtResultDto {
    pub status: IgtStatus,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub issues: Vec<IgtIssueDto>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub igt_elements: Vec<IgtElementDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminated_reason: Option<TerminationReasonDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IgtStatus {
    Complete,
    Error,
}
```

## Implementation Notes

### Backward Compatibility

1. **Without `igtTools`**: `data` is the axe issues array (current behavior)
2. **With `igtTools`**: `data` contains `axe` and `igt` as coequal keys
3. **Deprecated `igt` tool**: Still works, returns flat structure, logs deprecation warning

### Domain Mapping

| Deque Concept | rgaa-rs Concept |
|--------------|-----------------|
| WCAG 2.2 AA | RGAA 106 criteria |
| axe-core rules | RGAA rule mappings via `AxeMapper` |
| axe DevTools | Obscura CDP bridge |
| Advanced Rules | Custom RGAA rules via `rgaa-rules` |
| IGT | Guided tests via `rgaa-obscura` guided module |

### Limits Summary

| Feature | Limit |
|---------|-------|
| Pre-scan actions | 20 steps |
| Cookies | 20 cookies |
| Selector array (iframe/shadow) | 10 segments |
| Viewport dimension | 1-7680 pixels |
| Remediation issues | 1-25 per batch |
| Screenshot value length | Up to 10,000 chars |
| Cookie value length | Up to 10,000 chars |

## File Structure

```
rgaa-mcp/src/
├── main.rs              # Entry point, service setup
├── lib.rs               # Re-exports
├── server.rs            # ToolServer, AnalyzeRequest, redaction, error types
└── tools/
    ├── mod.rs           # ErrorCode, shared types
    ├── analyze.rs       # AnalyzeRequest, AnalyzeResponse, FindingDto, etc.
    ├── remediate.rs     # RemediationIssueInput, RemediationResponse, etc.
    └── igt.rs           # GuidedTestInput, GuidedTestResponse, IgtResultDto
```

## Testing Requirements

1. **Unit tests** for redaction logic
2. **Unit tests** for PreScanAction parsing (especially waitFor state machine)
3. **Unit tests** for CookieInput validation
4. **Integration tests** for analyze → remediate flow
5. **Contract tests** for MCP protocol compliance
