# Phase 1: Debug & Status Report

**Date**: 2026-10-02  
**Status**: 🔍 INVESTIGATION IN PROGRESS

## Discovery: Tools Already Implemented!

Contrary to the initial analysis, all 3 MCP tools are **fully implemented** in the codebase:

### ✅ Implementations Found

| Tool | File | Lines | Status |
|------|------|-------|--------|
| `lint_static` | `rgaa-mcp/src/tools/lint.rs` | ~95 | ✅ Complete |
| `verify_fix` | `rgaa-mcp/src/tools/verify_fix.rs` | ~245 | ✅ Complete |
| `source_map` | `rgaa-mcp/src/tools/source_map.rs` | ~1007 | ✅ Complete |

All 3 are **registered in ToolServer** with `#[tool(...)]` macro:
- Line 973: `lint_static` (wrapper around `rgaa-linter`)
- Line 857: `verify_fix` (browser re-analysis + diff)
- Line 830: `source_map` (DOM → source file mapping)

All 3 are **wired in rgaa-mcp-http** (lib.rs):
- Line 443-469: `tools/call` dispatch handles all three
- Line 355-357: `tools/list` announces them via router macro

---

## Phase 1 Actual Scope

Phase 1 is **NOT** "create stubs" — it's **verification and cleanup**:

### What needs to happen

1. **✓ Verify compilation** (in progress)
   - `cargo build -p rgaa-mcp`: must succeed
   - `cargo build -p rgaa-mcp-http`: must succeed
   - `cargo build -p rgaa-linter`: must succeed (lint_static depends on it)

2. **❓ Verify tests** (pending)
   - `cargo nextest run -p rgaa-mcp`: must pass
   - `cargo nextest run -p rgaa-mcp-http`: must pass
   - Focus on tool dispatch tests

3. **❓ Integration test** (to write)
   - Spawn HTTP server on `127.0.0.1:9999`
   - Call `tools/list` via HTTP POST/JSON-RPC
   - Call each tool once (lint_static, verify_fix, source_map)
   - Verify valid JSON-RPC responses

4. **❓ CI readiness** (to audit)
   - Does release.yml smoke test the tools?
   - Are there any `.expect()` or `.unwrap()` in dispatch paths?
   - Are error messages safe for unauthenticated HTTP?

---

## Codebase Structure

```
rgaa-rs/crates/
├── rgaa-linter/           # Static analysis engine
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs         (LintOptions, LintReport, lint_sources, lint_paths)
│       ├── engine.rs      (Rule registry, severity enum)
│       ├── rules/         (Per-rule implementations)
│       └── ...
├── rgaa-mcp/              # MCP tool wrappers + server
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs         (pub use tools, server)
│       ├── server.rs      (#[tool_router] impl ToolServer with 7 tools)
│       ├── tools/
│       │   ├── mod.rs     (ErrorCode enum, re-exports)
│       │   ├── lint.rs    (LintStaticRequest, LintStaticResponse)
│       │   ├── verify_fix.rs (VerifyFixRequest/Response, categorize)
│       │   ├── source_map.rs (SourceMapRequest/Response, element matching)
│       │   ├── analyze.rs (Existing, working)
│       │   ├── remediate.rs (Existing, working)
│       │   └── igt.rs     (Existing, working)
│       └── ...
├── rgaa-mcp-http/         # HTTP transport
│   ├── Cargo.toml         (✅ Clean)
│   ├── src/
│   │   ├── main.rs        (✅ Entry point, clean)
│   │   └── lib.rs         (✅ Axum + SSE + JSON-RPC, 702 lines)
│   └── tests/             (CORS, auth, dispatch tests exist)
└── ...
```

---

## Implementation Checklist

### Already Done
- ✅ Tool DTOs (request/response types)
- ✅ Tool implementations (logic, not stubs)
- ✅ ToolServer registration (#[tool(...)] macro)
- ✅ HTTP transport (JSON-RPC 2.0, SSE)
- ✅ CORS + Bearer auth
- ✅ Error mapping (JSON-RPC error codes)

### To Verify
- [ ] `cargo build -p rgaa-mcp` ← **currently running**
- [ ] `cargo build -p rgaa-mcp-http` 
- [ ] `cargo nextest run -p rgaa-mcp`
- [ ] `cargo nextest run -p rgaa-mcp-http`
- [ ] No `.unwrap()` in `tools/call` paths (code review)
- [ ] No `.expect()` in dispatch (code review)

### To Write
- [ ] Integration test: spawn HTTP server, call tools
  ```bash
  # Pseudo-code
  cargo build -p rgaa-mcp-http --release
  ./target/release/rgaa-mcp-http --port 9999 &
  sleep 1
  
  curl -X POST http://127.0.0.1:9999/mcp \
    -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'
  
  # Verify response: contains lint_static, verify_fix, source_map
  
  # Call each tool
  curl -X POST http://127.0.0.1:9999/mcp \
    -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"lint_static","arguments":{}}}'
  ```

- [ ] Add to release.yml CI (if not already there)

---

## Blocking Issues

### 🔴 Linker Error (Debug Mode)
**Status**: Trying RUSTFLAGS="" for workaround  
**Error**: `collect2: fatal error: cannot find 'ld'` when trying to link with mold  
**Cause**: mold not found in PATH; RUSTFLAGS forces use of broken mold  
**Fix**: Use `-C link=/usr/bin/ld` or build in release mode (mold skipped)

---

## Next Steps

1. **Wait for build to complete** (release mode, in background)
2. **Check for compilation errors** in src/ files
3. **Run test suite** if build succeeds
4. **Write integration test** for HTTP dispatch
5. **Code review dispatch paths** for panics
6. **Create minimal HTTP test** as proof-of-concept

---

## Questions for Code Review

When Phase 1 PR lands, these need verification:

1. **Panic-safety**: Any `.unwrap()` or `.expect()` in:
   - `tools/call` dispatch (lib.rs 359-476)?
   - `ToolServer` tool methods?
   - Request validation?

2. **Error messages**: Are all error messages safe for unauthenticated HTTP?
   - No file paths that leak system structure?
   - No secrets in error details?

3. **Timeout handling**: Does `source_map` handle large directory trees without hanging?
   - `MAX_DEPTH = 32` ✓
   - `MAX_FILES = 4_000` ✓
   - `MAX_TOTAL_BYTES = 64 MiB` ✓
   - All enforced ✓

4. **Concurrency**: Is `spawn_blocking` used for I/O-heavy tools?
   - `source_map` uses it ✓ (line 842)
   - `lint_static` doesn't (sync, <1ms/file) ✓
   - `verify_fix` doesn't (it calls `analyze_service.analyze` which is already async) ✓

---

## Debugging Artifacts

- Build log (running): `/tmp/claude-0/-home-user-Holo-RGAA/.../tasks/bh3b95iiq.output`
- This document: `docs/PHASE1_DEBUG.md`

