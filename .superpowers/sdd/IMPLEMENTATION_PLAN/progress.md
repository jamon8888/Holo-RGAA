# SDD ledger — plan: IMPLEMENTATION_PLAN.md

Branch: feat/rig-agentic-loop
BASE: ea072a378cabbe614cb5e0d2ca42da3720745bf9

## Pre-flight scan

| Tasks | Shared file/interface | Finding | Ruling |
|-------|----------------------|---------|--------|
| M1.1 + M1.2 | pipeline.rs | Different functions (calculate_compliance vs audit_one else-if). No conflict. | Clean |
| M1.4 + M1.5 | holo client.rs | Same file, related changes (new + evaluate signatures). Both need RgaaError import. | Batch together |
| M1.4 | rgaa-agent models.rs | new_placeholder callers of HoloClient::new. No other task touches. | Clean |
| M1.3 | obscura lib.rs | build_gap_fix_script discards snippet return value. Independent of other tasks. | Clean |
| M1.6 | holo prompts.rs | build_for_criterion uses wrong split('-'). Independent. | Clean |
| M1.7 | rules axe_mapper.rs | map returns HashMap, needs Result. Independent. | Clean |

All tasks compatible. No plan-text contradictions found.

## Implementation

Task 1: M1 correctness fixes (all subtasks)
- Status: complete
- Commits: subagent + controller fixes
- Review: controller verified

### Additional fixes (controller)
- E0597 borrow error in pipeline.rs: held MutexGuard across bridge calls instead of trying to clone ObscuraBridge (which doesn't implement Clone)
- Prompt test assertions: updated to expect untrusted data delimiters
- cargo clean: fixed corrupted target directory (idna_adapter, half, icu_normalizer missing crates)

### Test results
- rgaa-core: 15/15 ✅
- rgaa-holo: 31/31 ✅ (including prompt tests)
- rgaa-rules: 5/5 ✅
- rgaa-orchestrator: compiles clean, tests slow to compile (heavy deps on i5 hardware)
- rgaa-agent: not tested (heavy rig-core deps, compilation timeout)

---

Task 2: M2 pipeline wiring (M2.2, M2.4, M2.5, M2.6)
- Status: complete
- Subagent completed all 4 tasks
- Test results: 17/17 pass (rgaa-core + rgaa-agent via check)

### M2 changes
- M2.4: `From<AuditResult> for AuditBundle` + `From<&CrawlConfig> for AuditConfig` + 2 tests
- M2.5: `tactical_rpm`/`reasoning_rpm` in AgentConfig with env var support
- M2.6: Removed dead `ModelRouter`, `ModelInfo`, `SelectedTier`
- M2.2: Rewired CLI `analyze.rs` to call `Orchestrator::run` instead of `ObscuraBridge::analyze`

---

Task 3: M3.1 catalog
- Status: complete
- Test results: 22/22 pass (rgaa-core)

### M3.1 changes
- Created `rgaa-core/src/catalog.rs`: `RgaaCatalog` with `OnceLock`, embeds `criteres.json`
- 106 criteria, 13 themes, `by_id()`, `title()`, `tests()`, `test_count()`
- 5 catalog tests passing
