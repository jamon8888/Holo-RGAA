# Plugin Backend Implementation Plan

## Phase 1: MCP Tool Stubs (Unblocking)

**Target issues**: #86, #88, #89, #90  
**Duration**: 1-2h  
**Goal**: Make `rgaa-mcp-http` dispatchable; unblock #159 tests and release.yml

### Current State
- ✅ `rgaa-mcp-http` HTTP/SSE transport is production-ready
- ❌ 3 tools are dispatched but not implemented: `lint_static`, `verify_fix`, `source_map`
- ❌ Stubs must exist in `rgaa_mcp::ToolServer` or dispatch panics

### Implementation

#### 1.1 Create tool stubs in `rgaa-mcp/src/tools/`

```rust
// rgaa-mcp/src/tools/lint_static.rs
pub struct LintStaticRequest {
    pub paths: Vec<String>,
    pub config: Option<LintConfig>,
    pub source_files: Option<Vec<SourceFile>>,
}

#[derive(Default, Serialize)]
pub struct LintStaticResponse {
    pub results: Vec<LintResult>,
    pub summary: LintSummary,
}

// rgaa-mcp/src/tools/verify_fix.rs
pub struct VerifyFixRequest {
    pub audit_id: String,
    pub files: Vec<(String, String)>, // (path, content)
}

#[derive(Default, Serialize)]
pub struct VerifyFixResponse {
    pub fixed: Vec<FindingId>,
    pub remaining: Vec<FindingId>,
    pub new: Vec<FindingId>,
}

// rgaa-mcp/src/tools/source_map.rs
pub struct SourceMapRequest {
    pub findings: Vec<DomFinding>,
    pub source_files: Vec<SourceFile>,
}

#[derive(Default, Serialize)]
pub struct SourceMapResponse {
    pub mapped: Vec<MappedFinding>,
    pub unmapped: Vec<UnmappedFinding>,
}
```

#### 1.2 Register in `ToolServer`

```rust
// rgaa-mcp/src/lib.rs
#[tool_router]
impl ToolServer {
    /// Static linter MCP tool (STUB v0)
    pub fn lint_static(&self, req: LintStaticRequest) -> Result<LintStaticResponse, Error> {
        tracing::info!("lint_static stub called");
        Ok(LintStaticResponse::default())
    }

    /// Fix verification MCP tool (STUB v0)
    pub async fn verify_fix(&self, req: VerifyFixRequest) -> Result<VerifyFixResponse, Error> {
        tracing::info!("verify_fix stub called");
        Ok(VerifyFixResponse::default())
    }

    /// Source mapping MCP tool (STUB v0)
    pub async fn source_map(&self, req: SourceMapRequest) -> Result<SourceMapResponse, Error> {
        tracing::info!("source_map stub called");
        Ok(SourceMapResponse::default())
    }
}
```

#### 1.3 Verify dispatch in `rgaa-mcp-http`

- `tools/list` announces all 3 tools (should already work via macro)
- `tools/call` dispatches without panic (already wired in lib.rs:443-469)
- Add test: `curl -X POST http://localhost:3000/mcp -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"lint_static","arguments":{}}}'`

### Acceptance Criteria

- [ ] `cargo build -p rgaa-mcp` succeeds
- [ ] `cargo build -p rgaa-mcp-http` succeeds
- [ ] Tests: `cargo nextest run -p rgaa-mcp`
- [ ] Manual test: spawn `rgaa-mcp-http`, call each tool via curl, get valid JSON-RPC response
- [ ] No `.expect()` or `.unwrap()` in tool dispatch paths
- [ ] Tool schemas documented in API table (tools.rs module docs)

### Blockers for merge

- [ ] Integration test: spawn HTTP server, call 1 tool, receive valid response
- [ ] CORS test passes
- [ ] CI passes (build + test)

---

## Phase 2: Verify Fix Tool Implementation

**Target issue**: #89  
**Depends on**: Phase 1  
**Duration**: 2-3h  
**Goal**: Re-audit fixed files, diff findings

### Implementation

Reuse existing CLI code:
```bash
rgaa audit verify --audit-bundle audit.json --files src/**/*.tsx
```

Wrap it:
```rust
pub async fn verify_fix(&self, req: VerifyFixRequest) -> Result<VerifyFixResponse, Error> {
    // Write req.files to temp dir
    // Call rgaa audit verify command
    // Parse output JSON
    // Return diff: {fixed, remaining, new}
}
```

---

## Phase 3: Static Lint Tool Implementation

**Target issue**: #88  
**Depends on**: Phase 1  
**Duration**: 4-6h  
**Goal**: WCAG 2.1 AA static rules, ~5 high-confidence fixers

### Scope

- ~200 LOC: Rules engine + registry (5 rules: image-alt, label, button-name, link-name, lang)
- ~300 LOC: Parsers (HTML, TSX, Vue via tree-sitter or regex)
- ~100 LOC: Fix hints (AST transforms)
- ~100 LOC: Source mapping (optional)

### Dependencies

```toml
tree-sitter = "0.21"
regex = "1.10"
```

---

## Phase 4: Source Map Tool Implementation

**Target issue**: #90  
**Depends on**: Phase 3, Phase 1  
**Duration**: 2-3h  
**Goal**: Browser findings → source AST location

### Scope

- Element signature extraction (tag + attrs + text)
- Fuzzy matching in source
- AST location via tree-sitter

---

## Release Path

Once Phase 1 merges (stubs + tests):
1. ✅ #159 can test HTTP transport
2. ✅ #170 can smoke-test release (all tools present)
3. ✅ #169 can document full API

Phases 2-4 are parallel to release (not blocking).

---

## Testing Strategy

### Unit Tests
```rust
#[tokio::test]
async fn lint_static_with_no_files_returns_empty_result() {
    let req = LintStaticRequest { paths: vec![], ..Default::default() };
    let resp = server.lint_static(req).await.unwrap();
    assert_eq!(resp.results.len(), 0);
}
```

### Integration Tests
```bash
cargo build -p rgaa-mcp-http
./target/debug/rgaa-mcp-http --port 9876 &
sleep 1

curl -X POST http://localhost:9876/mcp \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'
# Should list: analyze, remediate, igt, audit_url, lint_static, verify_fix, source_map, …

curl -X POST http://localhost:9876/mcp \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"lint_static","arguments":{}}}'
# Should return valid JSON-RPC response
```

---

## Rollout

| Week | Phase | Deliverable |
|------|-------|-------------|
| W1   | 1     | Stubs PR #XXX, merge to main |
| W2   | 2, 3  | verify_fix + lint_static PRs |
| W3   | 4     | source_map PR |
| W4   | Docs  | Plugin integration guide + types |

