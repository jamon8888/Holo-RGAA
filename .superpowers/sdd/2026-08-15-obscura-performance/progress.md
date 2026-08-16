# SDD ledger — plan: .superpowers/sdd/2026-08-15-obscura-performance/plan.md

Task 1: complete (rgaa-obscura fixes, independent verification: cargo test -p rgaa-obscura 5 passed/0 failed)

## Fix note — rgaa-obscura CDP defects (2026-08-15)

Applied all 7 fixes to `crates/rgaa-obscura/src/lib.rs` (build green, `cargo test -p rgaa-obscura` passes: 1 unit + 4 integration tests).

1. Navigation sync: replaced fixed 3s sleep with `wait_for_load` (lib.rs ~L300) that observes `Page.loadEventFired` / `Page.lifecycleEvent` name=="load" events and polls `document.readyState` until "complete", bounded by a 15s timeout.
2. No false-clean: `validate_axe_result` (lib.rs ~L? ) treats missing result object, `exceptionDetails`, `subtype=="error"`, null value, missing/invalid `violations` as `Err`. `axe.run()` now uses `returnByValue:true` + `awaitPromise:true` and the decoded value must be an array. Removed `unwrap_or("[]")`.
3. Target/session leak: `run_axe_with_script` now always detaches (`Target.detachFromTarget`) and closes (`Target.closeTarget`) the target, then closes the socket, on every exit path via best-effort cleanup after `run_axe_core`.
4. Batch CLI now passes ALL urls (`.args(urls.iter())`) and parses the real single-object `results` array format (`parse_scrape_results`); non-conforming output is an `Err` (no silent drops).
5. `run_axe_batch` bounded by `tokio::sync::Semaphore` sized `max(1, concurrency)`; `concurrency==0` treated as 1.
6. axe-core fetched once per `run_axe_batch` (and per single `run_axe`) via `fetch_axe_source` + internal `run_axe_with_script(&self, url, axe_source: &str)`; batch calls reuse one `Arc<String>` under the semaphore.
7. Tests strengthened: axe test parses result as JSON and asserts it is an array; page-context test asserts object with `title`; added `test_run_axe_with_broken_script_surfaces_error` (unit test) proving a throwing eval surfaces `Err`; added batch axe + batch page-context network tests asserting per-URL entries.

Env note: `obscura scrape` requires the `obscura-worker` binary at `~/.local/bin/obscura-worker` (extracted from `obscura-x86_64-linux.tar.gz`) to run the batch CLI tests; single-url `fetch`/CDP paths need no worker.

Concerns: none blocking. `extract_page_context`/`run_gap_fix` (single-url) still use `obscura fetch` and were left unchanged as out of scope.

Task 2: complete (orchestrator run_batch + audit_one<B: AuditBridge>; builds under default and browser-obscura; workspace `cargo test --workspace --no-run` green)

Task 5: complete (verification done: `cargo test -p rgaa-obscura` 5 passed/0 failed; `cargo build -p rgaa-orchestrator` default + browser-obscura; full workspace `--no-run` green. Note: had to free ~/.cargo/registry/src (2G) due to disk-full on /dev/sda2; recoverable from registry/cache.)

Task 6: complete (final review)

## Final notes
- No code changes committed (user has unrelated uncommitted work).
- Single-URL audit behavior is preserved via generic `audit_one<B: AuditBridge>`; `run` delegates to `run_batch`.
- Obscura CDP server started once per batch and stopped via `Drop`.
- `obscura scrape` batch path requires `~/.local/bin/obscura-worker` (extracted from tarball). Single-URL `fetch`/CDP paths need no worker.
- Remaining recommendation (not done): add a criterion-based benchmark for `run_axe_batch`; fix the pre-existing hardcoded HOLO3_API_KEY in pipeline.rs if it should not be committed.

## Holo3 call-path optimization (follow-up, 2026-08-16)

User asked to optimize Holo3 calls for fast performance. Root cause: `audit_one` (pipeline.rs) evaluated the 27 `ia_assiste` criteria **sequentially**, each a network round-trip to `api.hcompany.ai` with up to 5 retries + backoff — measured bottleneck (orchestrator e2e test exceeded 240s). Implemented steps 1-4:

1. Parallelized the 27 evaluations with bounded concurrency: `tokio::sync::Semaphore` (HOLO3_CONCURRENCY = 12) + `tokio::task::JoinSet`. `HoloClient` is `Send+Sync`, so concurrent `evaluate` is safe.
2. Reuse one `HoloClient` for the whole `run_batch` (built once, passed `&Arc<HoloClient>` into `audit_one`) instead of rebuilding per URL — reuses the reqwest connection pool.
3. Client trim: `max_tokens` 2048 -> 512; request timeout 60s -> 30s.
4. Retry tuning: `MAX_RETRIES` 5 -> 3, `INITIAL_BACKOFF_MS` 1000 -> 500, added jitter (`jitter_for`) to both 429 and request-failure backoff to avoid thundering herd.

Files: `crates/rgaa-holo/src/client.rs` (consts, `with_base_url` test hook, jitter, token/timeout), `crates/rgaa-orchestrator/src/pipeline.rs` (parallel Holo3 loop, shared client, imports + HOLO3_CONCURRENCY).

Tests: added `crates/rgaa-holo/src/client.rs` mock-server tests `test_evaluate_parses_via_mock_server` and `test_evaluate_concurrent_send` (network-free, validate parse + concurrent Send). `cargo test -p rgaa-holo` -> 10 passed/0 failed. Orchestrator builds under default and `browser-obscura`; `cargo test -p rgaa-orchestrator --features browser-obscura --no-run` green. rgaa-obscura tests still 5 passed (unchanged).

Note: the e2e orchestrator audit (`tests/obscura_audit.rs`) still depends on live Holo3 network/API; not executed here (would require valid API key + is slow). The parallel path is verified via compile + unit tests.

## Playwright bridge removal — Obscura is now the sole backend (2026-08-16)

User chose option B: delete `rgaa-browser` entirely, make Obscura the only browser backend.

- `crates/rgaa-orchestrator/src/pipeline.rs`: removed the `#[cfg(...)]` Playwright imports, the `AuditBridge` trait + both `impl AuditBridge` blocks (Playwright + Obscura); collapsed the two feature-gated `run_batch` variants into a single Obscura `run_batch`; changed `audit_one<B: AuditBridge>` to `audit_one(bridge: &ObscuraBridge, …)`.
- `crates/rgaa-orchestrator/tests/obscura_audit.rs`: removed the `#![cfg(feature = "browser-obscura")]` gate (test now always compiled).
- `rgaa-rs/Cargo.toml`: removed `"crates/rgaa-browser"` from `members`.
- `crates/rgaa-orchestrator/Cargo.toml`: deleted the `[features]` block; dropped the `rgaa-browser` optional dep; made `rgaa-obscura` a non-optional dependency.
- Deleted `crates/rgaa-browser/` (Cargo.toml + src/lib.rs + src/playwright.rs + src/js/interaction.js) via `git rm`.

Verification: `cargo build -p rgaa-orchestrator` green; `cargo test -p rgaa-obscura` 6 passed/0 failed; `cargo test -p rgaa-orchestrator --test obscura_audit` 1 passed; `cargo build --workspace` + `cargo test --workspace --no-run` green (no dangling references to the removed feature). `full_audit.rs` now runs an Obscura audit by default.

Out of scope: hardcoded `HOLO3_API_KEY` in `pipeline.rs` left as-is (pre-existing).
