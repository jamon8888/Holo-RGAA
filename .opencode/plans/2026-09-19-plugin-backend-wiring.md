# Plugin Backend Wiring — Implementation Plan

**Status:** Updated (2026-09-19) — Core batch/crawl implemented in `aefe7d9`  
**Target:** Enable Claude Code / Codex plugins to drive RGAA audits, remediation, and verification  
**Transport:** MCP over HTTP/WS + REST API for batch/async operations  
**Timeline:** 3-4 weeks (2 engineers) — **Week 1-2 already done for core batch/crawl**

---

## 0. Already Implemented (commit `aefe7d9`)

The following capabilities are **already in `rgaa-orchestrator`** and exposed via `rgaa-api::run_audit`:

| Capability | Implementation | Details |
|------------|----------------|---------|
| **Multi-page crawl** | `run_crawl_and_audit()` | `sample_mode=true`: 7 mandatory RGAA pages (Accueil, Contact, Mentions légales, Accessibilité, Aide, Plan du site, Authentification) via HEAD + spider fallback. `sample_mode=false`: full crawl up to `max_pages`/`max_depth`. |
| **Site-wide RGAA compliance** | `aggregate_site_compliance()` | Official RGAA rule: criterion = **NonConforme** for entire site if **ANY** page in sample has Fail/Error (not average). Updates `taux_global`, `coverage_percent`, `etat_conformite`. |
| **Concurrent audits** | `Semaphore` (3 permits) | `run_batch()` runs up to 3 audits in parallel. |
| **RGAA 7-page sampling** | `discover_rgaa_sample_pages()` | HEAD requests for known paths, spider fallback if <7 pages found. |
| **Single AuditResult per site** | Aggregates all pages | Returns one `AuditResult` with all `PageResult`s and site-wide metrics. |

**API impact**: `POST /api/v1/audit` now calls `run_crawl_and_audit()` — returns full site audit with aggregated compliance.

**What this means for the plan**:
- ✅ Batch orchestration core is done (Issue #6 partially complete)
- ✅ Crawl + sampling logic done
- ✅ Site-wide aggregation done
- 🔄 Still need: HTTP/WS transport, auth, static lint, verify_fix, source_map, docs, CLI integration, CI/CD

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Plugin (Claude Code / Codex)                  │
└─────────────────────────────────┬───────────────────────────────────┘
                                   │
               ┌───────────────────┼───────────────────┐
               ▼                   ▼                   ▼
       ┌───────────────┐   ┌───────────────┐   ┌───────────────┐
       │  MCP-HTTP/WS  │   │   REST API    │   │   SSE/WS      │
       │  (tools, RPC) │   │ (batch, auth) │   │ (progress)    │
       └───────┬───────┘   └───────┬───────┘   └───────┬───────┘
               │                   │                   │
               ▼                   ▼                   ▼
       ┌───────────────────────────────────────────────────────────┐
       │                    rgaa-mcp-http (new)                     │
       │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐  │
       │  │ Analyze* │ │Remediate │ │ Lint     │ │ License/Auth │  │
       │  │ IGT      │ │VerifyFix │ │SourceMap │ │ Middleware   │  │
       │  └──────────┘ └──────────┘ └──────────┘ └──────────────┘  │
       └────────────────────────────┬────────────────────────────────┘
                                    │
          ┌─────────────────────────┼─────────────────────────┐
          ▼                         ▼                         ▼
┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐
│ rgaa-obscura     │    │ rgaa-remediation │    │ rgaa-linter      │
│ (browser, CDP)   │    │ (AST + string)   │    │ (static, fast)   │
└──────────────────┘    └──────────────────┘    └──────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────────┐
│  rgaa-orchestrator (IMPLEMENTED in aefe7d9)                     │
│  • run_crawl_and_audit() — multi-page crawl + sampling          │
│  • run_batch() — 3 concurrent audits via Semaphore              │
│  • aggregate_site_compliance() — RGAA official rule (any fail=NC)│
│  • discover_rgaa_sample_pages() — 7 mandatory pages + fallback  │
└─────────────────────────────────────────────────────────────────┘
```

* `analyze` tool now returns full site-wide audit with aggregated compliance

---

## 2. Component Specifications

### 2.1 rgaa-mcp-http (New Crate)

**Purpose:** Expose existing MCP tools over HTTP + WebSocket for remote plugin access

#### 2.1.1 HTTP JSON-RPC Endpoint
```
POST /mcp
Content-Type: application/json
Authorization: Bearer <api_key>

Request (JSON-RPC 2.0):
{
  "jsonrpc": "2.0",
  "id": "req-1",
  "method": "tools/call",
  "params": {
    "name": "analyze",
    "arguments": { "url": "https://example.com", "config": {...} }
  }
}

Response:
{
  "jsonrpc": "2.0",
  "id": "req-1",
  "result": { ...AnalyzeResponse... }
}
```

#### 2.1.2 WebSocket Transport (for streaming)
```
GET /mcp/ws?token=<api_key>
Upgrade: websocket

Messages: JSON-RPC 2.0 over WS frames
- Supports server-initiated notifications (progress, logs)
- Bidirectional: plugin can cancel long-running operations
```

#### 2.1.3 SSE Progress Stream
```
GET /mcp/events?audit_id=xxx&token=<api_key>
Accept: text/event-stream

Events:
event: progress
data: {"audit_id":"xxx","stage":"crawling","pages_done":3,"pages_total":10}

event: finding
data: {"audit_id":"xxx","finding":{...FindingDto...}}

event: complete
data: {"audit_id":"xxx","result":{...AnalyzeResponse...}}

event: error
data: {"audit_id":"xxx","error":{"code":"BROWSER_CRASH","message":"..."}}
```

#### 2.1.4 Tool Registry (extends existing stdio tools)
| Tool | Input | Output | Streaming? |
|------|-------|--------|------------|
| `analyze` | `AnalyzeRequest` | `AnalyzeResponse` | Yes (SSE) |
| `remediate` | `RemediationRequest` | `RemediationResponse` | No |
| `run_guided_test` | `GuidedTestRequest` | `GuidedTestResponse` | Yes |
| `audit_crawl` | `AuditUrlInput` | `AuditUrlResult` | Yes |
| **`lint_static`** | `LintStaticRequest` | `LintStaticResponse` | No |
| **`verify_fix`** | `VerifyFixRequest` | `VerifyFixResponse` | No |
| **`source_map`** | `SourceMapRequest` | `SourceMapResponse` | No |
| **`list_criteria`** | `{}` | `ListCriteriaResponse` | No |
| **`get_audit`** | `GetAuditInput` | `AuditResultDto` | No |

---

### 2.2 Static Lint Tool (`lint_static`)

**Purpose:** Fast (<100ms/file) structural analysis without browser — for real-time editor feedback

#### 2.2.1 Request/Response Schema

```json
// LintStaticRequest
{
  "paths": ["src/App.tsx", "src/components/"],
  "config": {
    "rules": "wcag21aa",           // or "rgaa41", "section508", "custom"
    "custom_rules_path": null,     // optional path to custom rules TOML
    "include": ["**/*.tsx", "**/*.vue", "**/*.html"],
    "exclude": ["**/node_modules/**", "**/*.test.tsx"],
    "fail_on": "error",            // "error" | "warning" | "never"
    "max_file_size_kb": 500
  },
  "source_files": [                // optional: pre-send source for source mapping
    { "path": "src/App.tsx", "content": "...", "framework": "react" }
  ]
}

// LintStaticResponse
{
  "results": [
    {
      "file": "src/App.tsx",
      "line": 42,
      "column": 10,
      "end_line": 42,
      "end_column": 35,
      "rule_id": "image-alt",
      "rule_name": "Images must have alternate text",
      "severity": "error",         // "error" | "warning" | "info"
      "message": "Image element missing alt attribute",
      "fix_hint": "Add alt=\"description\" or alt=\"\" for decorative",
      "wcag_criteria": ["1.1.1"],
      "rgaa_criteria": ["1.1"],
      "confidence": "high",        // "high" | "medium" | "low"
      "snippet": "<img src=\"hero.png\">",
      "suggested_fix": {           // only when confidence=high
        "type": "insert_attribute",
        "attribute": "alt",
        "value": ""
      }
    }
  ],
  "summary": {
    "files_scanned": 12,
    "errors": 3,
    "warnings": 7,
    "info": 2,
    "duration_ms": 45
  }
}
```

#### 2.2.2 Implementation Approach
- Reuse `rgaa-rules::GapFixRules` + `axe_mapper` logic as static checks
- Add `html5ever`/`scraper` based DOM traversal for structural rules
- Rule config: TOML file at `~/.config/rgaa/lint-rules.toml` + built-in defaults
- No browser spawn — pure Rust, suitable for LSP integration

---

### 2.3 Verify Fix Tool (`verify_fix`)

**Purpose:** Re-audit specific files after plugin applies remediation — confirms fix + detects regressions

#### 2.3.1 Request/Response Schema

```json
// VerifyFixRequest
{
  "original_audit_id": "audit-abc123",
  "fixed_files": [
    { "path": "src/App.tsx", "content": "...fixed content..." }
  ],
  "rules_to_verify": ["image-alt", "label", "button-name"],  // optional: only re-check these
  "config": { ...AnalyzeConfig... }                           // optional: override viewport, etc.
}

// VerifyFixResponse
{
  "fixed": [
    { "rule": "image-alt", "file": "src/App.tsx", "element": "<img src=\"hero.png\" alt=\"\">" }
  ],
  "remaining": [
    { "rule": "color-contrast", "file": "src/App.tsx", "element": "...", "reason": "needs_runtime" }
  ],
  "new_violations": [],
  "summary": {
    "verified": 1,
    "still_failing": 1,
    "new_issues": 0
  }
}
```

---

### 2.4 Source Map Tool (`source_map`)

**Purpose:** Map browser DOM findings (axe violations) back to source file locations

#### 2.4.1 Request/Response Schema

```json
// SourceMapRequest
{
  "findings": [
    { "id": "f1", "rule": "image-alt", "target": "img", "html": "<img src=\"hero.png\">", "url": "https://example.com/" }
  ],
  "source_files": [
    { "path": "src/App.tsx", "content": "import React from \"react\";\n<img src=\"hero.png\">", "framework": "react" }
  ]
}

// SourceMapResponse
{
  "mapped": [
    {
      "finding_id": "f1",
      "source_location": {
        "file": "src/App.tsx",
        "line": 10,
        "column": 4,
        "end_line": 10,
        "end_column": 30,
        "snippet": "<img src=\"hero.png\">"
      }
    }
  ],
  "unmapped": []
}
```

---

### 2.5 Auth & License Middleware

**Purpose:** Enforce tiered access, track usage, support offline grace

#### 2.5.1 License Tiers (per distribution spec)

| Tier | Static Lint | Full Audit | Remediation | IGT | Batch | Rate Limit |
|------|-------------|------------|-------------|-----|-------|------------|
| Free | ✅ | ❌ | ❌ | ❌ | ❌ | 10/day |
| Professional | ✅ | ✅ | ✅ | ✅ | ✅ | 500/day |
| Team | ✅ | ✅ | ✅ | ✅ | ✅ | 5000/day |

#### 2.5.2 API Key Format
```
rgaa_sk_<tier>_<random>    // e.g., rgaa_sk_pro_abc123def456
```

#### 2.5.3 Middleware Behavior
```rust
// On each MCP/HTTP request:
1. Extract Authorization: Bearer rgaa_sk_...
2. Parse tier from prefix (sk_free, sk_pro, sk_team)
3. Check local cache (~/.config/rgaa/license.toml):
   - Valid signature? → allow
   - Expired but within grace_days (7)? → allow + warn header
   - Expired beyond grace? → 402 Payment Required
4. Increment usage counter (local + async SaaS sync)
5. If tier limit exceeded → 429 with retry-after
```

#### 2.5.4 Response Headers
```
X-License-Tier: professional
X-License-Grace-Days: 3
X-RateLimit-Limit: 500
X-RateLimit-Remaining: 487
X-RateLimit-Reset: 1726800000
```

---

### 2.6 Batch/Async API

**Purpose:** Handle multi-URL audits without blocking plugin

**✅ Already Implemented in Orchestrator (commit `aefe7d9`):**
- `run_batch()` — runs up to 3 concurrent audits via `Semaphore`
- `run_crawl_and_audit()` — multi-page crawl per URL with RGAA 7-page sampling
- `aggregate_site_compliance()` — site-wide RGAA compliance (any fail = NonConforme)
- Each URL returns full `AuditResult` with all pages + site-wide metrics

**🔄 Still Needed (REST API layer):**
- `POST /api/v1/batch/audit` — accepts multiple URLs, returns `batch_id`
- `GET /api/v1/batch/:id/status` — JSON status
- `GET /api/v1/batch/:id/events` — SSE progress stream
- `GET /api/v1/batch/:id/results` — aggregated results
- Webhook delivery with HMAC signature verification
- Persistent state in `rgaa-storage` (SQLite for local, PG for SaaS)
- Cleanup: auto-expire batches after 24h

#### 2.6.1 Endpoints (to implement)

```bash
# Start batch (each URL gets full site audit via run_crawl_and_audit)
POST /api/v1/batch/audit
Authorization: Bearer rgaa_sk_pro_xxx
Content-Type: application/json

{
  "urls": ["https://a.com", "https://b.com", "https://c.com"],
  "config": { 
    "profile": "default", 
    "viewport_width": 1280,
    "sample_mode": true,      // NEW: uses 7-page RGAA sampling
    "max_pages": 50,
    "max_depth": 5
  },
  "webhook": "https://plugin.example.com/rgaa/callback",
  "webhook_secret": "whsec_..."
}

# Response
{
  "batch_id": "batch_xyz789",
  "status": "pending",
  "status_url": "/api/v1/batch/batch_xyz789/status",
  "created_at": "2026-09-19T10:00:00Z"
}

# SSE events (per-URL progress):
GET /api/v1/batch/batch_xyz789/events
event: batch_progress
data: {"batch_id":"batch_xyz789","completed":2,"total":3,"current_url":"https://b.com"}

event: batch_item_complete
data: {"batch_id":"batch_xyz789","url":"https://a.com","audit_id":"audit_abc","status":"success","taux_global":85.2}

event: batch_complete
data: {"batch_id":"batch_xyz789","summary":{"success":2,"failed":1}}

# Final result (each result = full site audit)
GET /api/v1/batch/batch_xyz789/results
{
  "batch_id": "batch_xyz789",
  "results": [
    { "url": "https://a.com", "audit_id": "audit_abc", "status": "success", "taux_global": 85.2, "etat_conformite": "partielle" },
    { "url": "https://b.com", "audit_id": "audit_def", "status": "success", "taux_global": 92.1, "etat_conformite": "totale" },
    { "url": "https://c.com", "audit_id": null, "status": "failed", "error": "timeout" }
  ]
}
```

---

## 3. Implementation Issues

### Epic: Plugin Backend Wiring

#### Issue #1: rgaa-mcp-http Crate — HTTP/WS Transport
**Labels:** `area:mcp`, `area:plugin`, `priority:P0`, `size:L`
**Dependencies:** None

**Acceptance Criteria:**
- [ ] `cargo new rgaa-mcp-http` in workspace
- [ ] Axum server with `/mcp` (JSON-RPC), `/mcp/ws` (WebSocket), `/mcp/events` (SSE)
- [ ] Reuses `rgaa-mcp` tool handlers via shared trait `McpToolHandler`
- [ ] Request validation + structured error responses (JSON-RPC error format)
- [ ] CORS configured for plugin origins
- [ ] Unit tests for each transport mode
- [ ] Integration test: plugin connects via HTTP, calls `analyze`, receives SSE progress

**Files to create:**
```
rgaa-rs/crates/rgaa-mcp-http/
├── Cargo.toml
├── src/
│   ├── main.rs              # Binary entry (rgaa-mcp-server)
│   ├── server.rs            # Axum router + state
│   ├── transport/
│   │   ├── http.rs          # JSON-RPC over HTTP
│   │   ├── ws.rs            # WebSocket transport
│   │   └── sse.rs           # SSE progress stream
│   ├── handlers/            # Thin wrappers over rgaa-mcp tools
│   └── error.rs             # JSON-RPC error mapping
└── tests/
    └── integration_test.rs
```

---

#### Issue #2: Auth/License Middleware (Shared)
**Labels:** `area:auth`, `area:plugin`, `priority:P0`, `size:M`
**Dependencies:** `rgaa-license` crate (new, per distribution spec)

**Acceptance Criteria:**
- [ ] `rgaa-license` crate with `LicenseValidator` trait
- [ ] Local cache: `~/.config/rgaa/license.toml` (signed JWT or PASETO)
- [ ] Offline grace period (7 days default, configurable)
- [ ] Tier enforcement: Free/Pro/Team limits
- [ ] Usage tracking with async SaaS sync (non-blocking)
- [ ] Middleware for Axum (`tower::ServiceBuilder` layer)
- [ ] Response headers: `X-License-*`, `X-RateLimit-*`
- [ ] 402/429 responses with `Retry-After`

**Files:**
```
rgaa-rs/crates/rgaa-license/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── validator.rs
│   ├── cache.rs
│   ├── tiers.rs
│   └── middleware.rs        # Axum layer
└── tests/
    └── license_test.rs
```

---

#### Issue #3: Static Lint MCP Tool (`lint_static`)
**Labels:** `area:linter`, `area:plugin`, `priority:P0`, `size:L`
**Dependencies:** `rgaa-linter` (planned) or extend `rgaa-rules`

**Acceptance Criteria:**
- [ ] New MCP tool `lint_static` registered in `rgaa-mcp-http`
- [ ] Input: `LintStaticRequest` (paths, config, optional source_files)
- [ ] Output: `LintStaticResponse` with `LintResult[]` + `LintSummary`
- [ ] Rules: WCAG 2.1 AA, RGAA 4.1, Section 508 profiles
- [ ] Source mapping: when `source_files` provided, include `source_location` in results
- [ ] Fix hints for high-confidence rules (image-alt, label, button-name, link-name)
- [ ] <100ms/file for typical TSX/Vue/HTML
- [ ] Config file: `~/.config/rgaa/lint-rules.toml`

**Files:**
```
rgaa-rs/crates/rgaa-linter/          # new crate (or extend rgaa-rules)
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── engine.rs
│   ├── rules/
│   │   ├── structural.rs      # AttributePresence, ElementPresence, etc.
│   │   ├── semantic.rs        # Compound, Nesting, etc.
│   │   └── registry.rs        # RuleSet loading
│   ├── parser.rs              # html5ever/scraper integration
│   ├── source_map.rs          # finding → source location
│   └── fix_hints.rs           # suggested fixes
└── tests/
    └── linter_test.rs
```

---

#### Issue #4: Verify Fix MCP Tool (`verify_fix`)
**Labels:** `area:remediation`, `area:plugin`, `priority:P1`, `size:M`
**Dependencies:** Issue #1, `rgaa-obscura` analyze

**Acceptance Criteria:**
- [ ] MCP tool `verify_fix` registered
- [ ] Input: `VerifyFixRequest` (original_audit_id, fixed_files[], rules_to_verify?)
- [ ] Re-runs audit on fixed files (serves via data: URI or localhost)
- [ ] Compares findings: fixed vs remaining vs new
- [ ] Output: `VerifyFixResponse` with categorized results
- [ ] Reuses `rgaa-obscura` analyze pipeline
- [ ] Timeout: 30s per file max

---

#### Issue #5: Source Map MCP Tool (`source_map`)
**Labels:** `area:linter`, `area:plugin`, `priority:P1`, `size:M`
**Dependencies:** Issue #3 (linter engine)

**Acceptance Criteria:**
- [ ] MCP tool `source_map` registered
- [ ] Input: browser findings + source files with framework hints
- [ ] Matching algorithm: element signature (tag + attrs + text) → source AST
- [ ] Framework-aware: React JSX, Vue SFC, Angular templates, vanilla HTML
- [ ] Output: mapped findings with `source_location` (file, line, col, snippet)
- [ ] Unmapped findings returned separately with reason

---

#### Issue #6: Batch/Async REST API + SSE (Reduced Scope)
**Labels:** `area:api`, `area:plugin`, `priority:P1`, `size:S`  ← **was M, now S**
**Dependencies:** Issue #2 (auth middleware)
**Note:** Orchestrator core (`run_batch`, `run_crawl_and_audit`, `aggregate_site_compliance`) ✅ done in `aefe7d9`

**Acceptance Criteria (REST API layer only):**
- [ ] `POST /api/v1/batch/audit` — accepts URLs + config + optional webhook
- [ ] `GET /api/v1/batch/:id/status` — JSON status
- [ ] `GET /api/v1/batch/:id/events` — SSE stream (per-URL progress)
- [ ] `GET /api/v1/batch/:id/results` — final aggregated results
- [ ] Webhook delivery with HMAC signature verification
- [ ] Persistent state in `rgaa-storage` (SQLite for local, PG for SaaS)
- [ ] Cleanup: auto-expire batches after 24h

**Files (simplified):**
```
rgaa-rs/crates/rgaa-api/
├── src/
│   ├── routes/
│   │   └── batch.rs           # new — thin wrapper over Orchestrator::run_batch
│   ├── batch/
│   │   ├── manager.rs         # batch state machine
│   │   ├── store.rs           # extends rgaa-storage
│   │   └── webhook.rs
│   └── sse.rs                 # shared SSE utilities
```

**Files:**
```
rgaa-rs/crates/rgaa-api/
├── src/
│   ├── routes/
│   │   └── batch.rs           # new
│   ├── batch/
│   │   ├── manager.rs
│   │   ├── store.rs           # extends rgaa-storage
│   │   └── webhook.rs
│   └── sse.rs                 # shared SSE utilities
```

---

#### Issue #7: Plugin-Facing Documentation + Examples
**Labels:** `area:docs`, `area:plugin`, `priority:P1`, `size:S`
**Dependencies:** Issues #1-6

**Acceptance Criteria:**
- [ ] `docs/plugin-integration.md` — complete guide
- [ ] TypeScript types for all MCP tools + REST endpoints
- [ ] Example plugin: `examples/claude-code-rgaa-plugin/`
- [ ] Example: `examples/codex-rgaa-plugin/`
- [ ] Authentication flow documented
- [ ] Error code reference table

---

#### Issue #8: rgaa Binary — MCP Server Mode
**Labels:** `area:cli`, `area:distribution`, `priority:P0`, `size:S`
**Dependencies:** Issue #1

**Acceptance Criteria:**
- [ ] `rgaa mcp-server` subcommand starts HTTP/WS server
- [ ] Flags: `--port`, `--host`, `--cors-origin`, `--stdio` (backward compat)
- [ ] Reads license from `~/.config/rgaa/license.toml`
- [ ] Graceful shutdown on SIGTERM (drain in-flight requests)
- [ ] Health endpoint: `GET /health` → `{ "status": "ok", "version": "..." }`

---

#### Issue #9: CI/CD for Plugin Artifacts
**Labels:** `area:ci`, `area:distribution`, `priority:P2`, `size:S`
**Dependencies:** Issues #1-8

**Acceptance Criteria:**
- [ ] GitHub Action: build `rgaa-mcp-server` for linux/amd64, linux/arm64, macos/amd64, macos/arm64
- [ ] Publish to GitHub Releases + npm (for TypeScript types package)
- [ ] Version sync: `rgaa-mcp-http` version = `rgaa` CLI version
- [ ] Smoke test: spawn server, call each MCP tool via HTTP

---

## 4. Dependencies & Sequencing (Updated)

**✅ Done (Week 0):** Batch/crawl core in orchestrator (`aefe7d9`)
- `run_crawl_and_audit()`, `run_batch()`, `aggregate_site_compliance()`, `discover_rgaa_sample_pages()`

```
Week 1:  #1 (mcp-http) + #2 (auth) in parallel
Week 2:  #3 (lint_static) + #4 (verify_fix) 
Week 3:  #5 (source_map) + #6 (batch REST API)  ← now size S
Week 4:  #7 (docs/examples) + #8 (CLI integration)
Week 5:  #9 (CI/CD) + integration testing + polish
```

**Critical Path:** #1 → #3, #4, #5, #6, #8  
**Can Parallelize:** #2, #3, #4, #5, #6 after #1 lands  
**Timeline reduced:** 4-5 weeks → **3-4 weeks** (1 week saved on batch core)

---

## 5. Open Questions

1. **rgaa-linter crate vs extend rgaa-rules?** 
   - Option A: New `rgaa-linter` crate (cleaner separation, matches distribution spec)
   - Option B: Add static lint module to `rgaa-rules` (less crates, but mixes concerns)
   - *Recommendation: Option A — distribution spec lists `rgaa-linter` as separate binary*

2. **MCP-HTTP binary: separate or embedded?**
   - Option A: `rgaa-mcp-server` separate binary (cleaner deployment, independent versioning)
   - Option B: `rgaa mcp-server` subcommand (single binary distribution)
   - *Recommendation: Option B — matches "single binary" distribution goal*

3. **Source mapping strategy:**
   - Option A: Plugin sends source files with each request
   - Option B: Plugin sends git repo + commit SHA, server clones/fetches
   - *Recommendation: Option A initially (simpler), Option B for Team tier later*

4. **WebSocket vs SSE for progress:**
   - SSE simpler, unidirectional, works through proxies
   - WebSocket bidirectional (plugin can cancel)
   - *Recommendation: Both — SSE for progress, WS for interactive tools (IGT)*

5. **License validation: local-only or SaaS sync required?**
   - Free tier: local-only validation (signed license file)
   - Pro/Team: periodic SaaS sync for usage tracking
   - *Recommendation: Implement local-first, SaaS sync async/non-blocking*

---

## 6. Success Metrics

| Metric | Target |
|--------|--------|
| `lint_static` latency | <100ms/file (median) |
| `analyze` cold start | <3s (browser spawn) |
| `verify_fix` round-trip | <10s |
| MCP-HTTP request throughput | >50 req/s (local) |
| Plugin auth check overhead | <5ms |
| Batch audit concurrency | 3 parallel browsers |

---

## 7. Next Steps

1. **Decide open questions** (above) — 30 min call
2. **Create GitHub issues** from this plan (use labels above)
3. **Assign owners** per issue
4. **Set up project board** with "Plugin Backend" column
5. **Kickoff Week 1** with Issues #1 + #2

---

*Plan author: [Your Name]*  
*Date: 2026-09-19*  
*Reviewers: [Team]*