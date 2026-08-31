# Plan: rgaa-mcp deque axe-mcp parity

## Context

The rgaa-rs MCP server provides accessibility auditing via 5 tools (`analyze`, `remediate`, `igt`, `audit_url`, `get_audit_result`). The implementation overlaps partially with deque's axe-mcp-server but diverges in several key capabilities documented in SPEC.md.

## Objective

Implement full parity with deque axe-mcp-server v4.0.0 capabilities for the `analyze` and `remediate` tools.

---

## Phase 1: Pre-scan actions — `waitFor` support

**Gap**: Current `PreScanActionInput` only supports `Click` and `Fill`. Deque adds `waitFor` with states: `visible`, `attached`, `hidden`, `detached`.

### Changes

**File**: `rgaa-mcp/src/tools/analyze.rs`

```rust
// Add WaitForState enum
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum WaitForState {
    #[default]
    Visible,
    Attached,
    Hidden,
    Detached,
}

// Extend PreScanActionInput
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PreScanActionInput {
    Click { selector: String },
    Fill { selector: String, value: String }, // value is sensitive - redacted
    WaitFor { selector: String, #[serde(default)] state: WaitForState },
}
```

**File**: `rgaa-mcp/src/server.rs`

Update `AnalyzeRequest::to_domain()` to map `WaitFor` variant to `rgaa_obscura::PreScanAction`.

**File**: `rgaa-obscura/src/config.rs`

Check if `PreScanAction` enum needs `WaitFor` variant added.

---

## Phase 2: Cookie injection — full cookie values

**Gap**: Current implementation only supports cookie *references* (env var names). Deque supports full cookie objects with direct `value`, `domain`, `path`, `sameSite`, `secure`, `httpOnly`, `expires`.

### Changes

**File**: `rgaa-mcp/src/tools/analyze.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CookieInput {
    pub name: String,
    pub value: String, // sensitive - redacted
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

**File**: `rgaa-mcp/src/tools/analyze.rs` — `AnalyzeConfigInput`

Replace `cookie_references: Vec<CookieReferenceInput>` with `cookies: Vec<CookieInput>`.

**File**: `rgaa-mcp/src/server.rs` — `AnalyzeRequest::to_domain()`

Map `CookieInput` to `rgaa_obscura::CookieReference` (or create new domain type). The current mapping only captures `name` and `domain` — needs to pass full cookie data.

**File**: `rgaa-obscura/src/config.rs`

Check if `CookieReference` needs extension to carry `value`, `path`, `sameSite`, etc. If Obscura doesn't support full cookie injection, this may require CDP-level `Network.setCookie` calls in the bridge.

---

## Phase 3: Screenshot options

**Gap**: Current `ScreenshotPolicyInput` is a simple enum (`None`, `OnFailure`, `Always`). Deque adds `ScreenshotInput` with `format`, `saveTo`, `save`, `inline` fields.

### Changes

**File**: `rgaa-mcp/src/tools/analyze.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct ScreenshotInput {
    #[serde(default)]
    pub format: Option<ScreenshotFormat>,
    #[serde(default)]
    pub save_to: Option<String>,
    #[serde(default)]
    pub save: Option<bool>,
    #[serde(default)]
    pub inline: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ScreenshotFormat {
    Png,
    Jpeg,
}
```

**File**: `rgaa-mcp/src/tools/analyze.rs` — `AnalyzeConfigInput`

Replace `screenshot_policy: ScreenshotPolicyInput` with `screenshot: Option<ScreenshotInput>`.

**File**: `rgaa-mcp/src/server.rs` — `AnalyzeRequest::to_domain()`

Update mapping to pass screenshot config to obscura. Note: current implementation has screenshot policy but not the full options — obscura bridge may need extension to support `saveTo` path and `format`.

---

## Phase 4: `advancedRules` and `igtTools` parameters

**Gap**: Neither `advanced_rules` (per-scan override) nor `igt_tools` (keyboard IGT) is implemented.

### Changes

**File**: `rgaa-mcp/src/tools/analyze.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AdvancedRulesInput {
    pub value: String,  // "thorough", "standard", "disabled"
    pub source: String, // "tool_arg" | "config"
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IgtToolsInput {
    pub tools: Vec<String>, // ["keyboard"]
}
```

**File**: `rgaa-mcp/src/tools/analyze.rs` — `AnalyzeConfigInput`

Add fields:
- `advanced_rules: Option<String>` (deque's simplified form)
- `igt_tools: Option<Vec<String>>`

**File**: `rgaa-mcp/src/server.rs`

- Update `to_domain()` to pass `advanced_rules` and `igt_tools` to obscura
- `igt_tools` requires: (1) calling IGT after axe scan in same browser session, (2) nesting IGT results under `data.igt` alongside `data.axe`

---

## Phase 5: Response shape — nested `data.igt` alongside `data.axe`

**Gap**: When `igtTools` is set, response must change shape to:
```json
{
  "data": {
    "axe": [...],
    "igt": { "keyboard": { "status": "complete", ... } }
  }
}
```

Without `igtTools`, `data` remains the flat axe issues array (backward compatible).

### Changes

**File**: `rgaa-mcp/src/tools/igt.rs`

Add new response DTOs:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IgtResultsDto {
    pub keyboard: IgtResultDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IgtResultDto {
    pub status: IgtStatus,
    pub issues: Vec<IgtIssueDto>,
    pub igt_elements: Vec<IgtElementDto>,
    pub terminated_reason: Option<TerminationReasonDto>,
}
```

**File**: `rgaa-mcp/src/tools/analyze.rs` — `AnalyzeResponse`

When `igt_tools` is present, `findings` is renamed/moved to `data.axe` and `igt_results: Option<IgtResultsDto>` is added under `data.igt`.

---

## Phase 6: Flatten `viewport_width` / `viewport_height` (low effort, high usability)

**Gap**: Current: `config.viewport_width`. Deque: flat `viewportWidth` at top level of `analyze` request.

### Changes

**File**: `rgaa-mcp/src/server.rs` — `AnalyzeRequest`

```rust
pub struct AnalyzeRequest {
    pub url: String,
    #[serde(default)]
    pub config: AnalyzeConfigInput,
    #[serde(default)]
    pub viewport_width: Option<u32>,  // flat, overrides config
    #[serde(default)]
    pub viewport_height: Option<u32>, // requires viewport_width if set
}
```

Update `to_domain()` to use flat params as overrides.

---

## Implementation Order

| Phase | Effort | Impact | Reason |
|-------|--------|--------|--------|
| 1: waitFor | Low | High | Enables real login flows |
| 2: cookies full | Medium | High | Pre-auth sessions |
| 3: screenshot options | Medium | Medium | Visual debugging |
| 4: advancedRules + igtTools | High | Medium | AI-powered features |
| 5: nested response | High | Medium | Protocol change |
| 6: flat viewport | Low | Low | Usability only |

**Recommended**: 1 → 2 → 3 → 6 → 4 → 5

---

## Files Summary

| File | Changes |
|------|---------|
| `rgaa-mcp/src/tools/analyze.rs` | `CookieInput`, `SameSiteInput`, `ScreenshotInput`, `ScreenshotFormat`, `WaitForState`, `PreScanActionInput::WaitFor`, `AdvancedRulesInput`, `IgtToolsInput`, update `AnalyzeConfigInput` |
| `rgaa-mcp/src/tools/igt.rs` | `IgtResultsDto`, `IgtResultDto`, `IgtStatus`, `IgtIssueDto`, `IgtElementDto` |
| `rgaa-mcp/src/tools/analyze.rs` — `AnalyzeResponse` | Add conditional `igt_results` field |
| `rgaa-mcp/src/server.rs` | `AnalyzeRequest` (flat viewport), `to_domain()` (new field mappings), error handling for new failures |
| `rgaa-obscura/src/config.rs` | Extend `PreScanAction`? `CookieReference`? depending on what's needed |
| `rgaa-mcp/tests/contract.rs` | Add tests for new parameter combinations, redaction of `CookieInput.value`, `Fill.value` |

---

## Testing Checklist

- [ ] `waitFor` with `visible` state works
- [ ] `waitFor` with `attached`/`hidden`/`detached` states work
- [ ] `CookieInput.value` is redacted in logs/errors
- [ ] `CookieInput` with all fields (path, sameSite, secure, httpOnly, expires) serializes correctly
- [ ] `ScreenshotInput` with `save_to` generates file
- [ ] `ScreenshotInput` with `format: jpeg` returns jpeg
- [ ] `ScreenshotInput` with `inline: false` skips image block
- [ ] `viewport_width` / `viewport_height` flat params override config
- [ ] `igtTools: ["keyboard"]` returns nested `data.igt.keyboard` structure
- [ ] Without `igtTools`, response is unchanged (backward compat)
- [ ] `advancedRules: "thorough"` is passed through to obscura
