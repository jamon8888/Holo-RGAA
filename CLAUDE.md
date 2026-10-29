# Claude Code - rgaa-rs Project Guidelines

RGAA 4.1.2 accessibility audit platform — Rust workspace with 7 crates.

## Project Overview

- **rgaa-core**: Domain types + 106 RGAA criteria catalog
- **rgaa-rules**: axe-core violation mapping + gap-fix JS snippets
- **rgaa-holo**: MyIA LLM client for AI-assisted evaluation
- **rgaa-browser**, **rgaa-orchestrator**, **rgaa-storage**, **rgaa-api**: Empty stubs

## Codebase-Specific Fixes

### Fix: `HoloClient::new()` panics on TLS failure

```rust
// BAD (crates/rgaa-holo/src/client.rs:68)
pub fn new(api_key: &str, api_url: &str) -> Self {
    let http_client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .expect("Failed to create HTTP client"); // PANICS!
    Self { http_client, api_key: api_key.to_string(), api_url: api_url.to_string() }
}

// GOOD: return Result
pub fn new(api_key: &str, api_url: &str) -> Result<Self, RgaaError> {
    let http_client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| RgaaError::Holo3(format!("HTTP client init failed: {e}")))?;
    Ok(Self { http_client, api_key: api_key.to_string(), api_url: api_url.to_string() })
}
```

### Fix: `HoloClient::evaluate()` returns `Result<_, String>`

```rust
// BAD (crates/rgaa-holo/src/client.rs)
pub async fn evaluate(&self, prompt: &str) -> Result<HoloResponse, String> { ... }

// GOOD: use the defined error type
pub async fn evaluate(&self, prompt: &str) -> Result<HoloResponse, RgaaError> { ... }
```

### Fix: `AxeMapper::map()` silently drops JSON errors

```rust
// BAD (crates/rgaa-rules/src/axe_mapper.rs:12-13)
let violations: Vec<AxeViolation> = serde_json::from_str(violations_json)
    .unwrap_or_default(); // silently returns empty vec on bad JSON

// GOOD: log or return error
let violations: Vec<AxeViolation> = serde_json::from_str(violations_json)
    .map_err(|e| {
        tracing::warn!(error = %e, "failed to parse axe violations JSON");
        e
    })?;
```

### Fix: Double clone in `AxeMapper::map()`

```rust
// BAD (crates/rgaa-rules/src/axe_mapper.rs:19-28)
results.insert(rgaa_id.clone(), CriterionResult {
    criterion_id: rgaa_id.clone(), // cloned twice!
    // ...
});

// GOOD: clone once for key, reference for value
let result = CriterionResult {
    criterion_id: rgaa_id.clone(),
    // ...
};
results.insert(rgaa_id, result);
```

### Fix: `RgaaCriteria::all()` allocates on every call

```rust
// BAD (crates/rgaa-core/src/criteria.rs:14-123)
pub fn all() -> Vec<Criterion> {
    vec![ /* 106 items */ ] // allocated every call
}

// GOOD: use OnceLock for static data
use std::sync::OnceLock;

pub fn all() -> &'static [Criterion] {
    static CRITERIA: OnceLock<Vec<Criterion>> = OnceLock::new();
    CRITERIA.get_or_init(|| vec![ /* 106 items */ ])
}
```

### Fix: Non-deterministic HashMap ordering

```rust
// BAD (crates/rgaa-rules/src/axe_mapper.rs)
use std::collections::HashMap;
let results: HashMap<String, CriterionResult> = HashMap::new(); // order varies

// GOOD: use IndexMap for deterministic output
use indexmap::IndexMap;
let results: IndexMap<String, CriterionResult> = IndexMap::new();
```

## Error Handling Pattern

```rust
// Library errors (rgaa-core/src/error.rs)
#[derive(Error, Debug)]
pub enum RgaaError {
    #[error("Crawl error: {0}")]
    Crawl(String),
    #[error("Browser error: {0}")]
    Browser(String),
    #[error("LLM API error: {0}")]
    Holo3(String), // legacy public variant name
    // ... 5 more variants
}
pub type Result<T> = std::result::Result<T, RgaaError>;

// Add this to enable ? in HoloClient
impl From<reqwest::Error> for RgaaError {
    fn from(e: reqwest::Error) -> Self {
        RgaaError::Holo3(e.to_string())
    }
}
```

## Unit-Struct Pattern

All stateless services use unit structs with associated functions:

```rust
pub struct AxeMapper;

impl AxeMapper {
    pub fn map(violations_json: &str) -> IndexMap<String, CriterionResult> {
        // ...
    }
}

pub struct PromptBuilder;

impl PromptBuilder {
    pub fn build(context: &EvaluationContext) -> Result<String, RgaaError> {
        // ...
    }
}
```

## Testing Layers

Four layers, all unconditional in CI (no repo variable, no secret): Rust
unit/integration (`Build & Test`), the Node DOM probe harnesses
(`Probe harness`), browser E2E (`E2E (navigateur)`) and the `rgaa-spider`
network tests (`E2E (spider, réseau)`) — plus a fifth,
`E2E (agent + orchestrator)`, which stays gated on `MYIA_API_KEY` and is
therefore still unverified. What each one proves,
how to run them locally, and why the E2E suite used to run nowhere at all:
`docs/testing-layers.md`. Two rules worth knowing before touching that suite:

- Never give an `ObscuraBridge` a fixed CDP port — use
  `ObscuraBridge::with_ephemeral_port()`. A fixed port is shared state between
  processes, and concurrent runs used to steal each other's server.
- A test that skips must never be able to pass silently in CI. The guard lives
  in `rgaa_test_corpus::e2e::enabled`; CI sets `RGAA_E2E_REQUIRED=1` so a lost
  `RUN_E2E` fails the job instead of reporting an empty green.

## Testing Gaps

| Crate | Has Tests | Needs |
|-------|-----------|-------|
| rgaa-core | No | `RgaaCriteria::all()`, `find()`, `deterministic()` |
| rgaa-rules | No | `AxeMapper::map()`, `GapFixRules::snippets()` |
| rgaa-holo | Yes (client, prompts) | `PromptBuilder::build()` e2e, async `evaluate()` |

```rust
// Add async tests
#[tokio::test]
async fn evaluate_returns_valid_response() {
    let client = HoloClient::new("test-key", "http://localhost:8080").unwrap();
    let result = client.evaluate("test prompt").await;
    assert!(result.is_ok());
}

// Add benchmarks
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_axe_mapper(c: &mut Criterion) {
    let json = std::fs::read_to_string("test_fixtures/axe_output.json").unwrap();
    c.bench_function("axe_mapper_map", |b| {
        b.iter(|| AxeMapper::map(&json));
    });
}

criterion_group!(benches, bench_axe_mapper);
criterion_main!(benches);
```

## Observability

```rust
// Use structured tracing (already done in rgaa-holo)
info!(attempt, max_retries = MAX_RETRIES, "Calling MyIA API");
warn!(attempt, backoff_ms = backoff, "Rate limited, backing off");
error!(status = status.as_u16(), body = %body, "API error");

// Add instrument spans
#[tracing::instrument(name = "evaluate", skip(self))]
pub async fn evaluate(&self, prompt: &str) -> Result<HoloResponse, RgaaError> { ... }
```

## Performance Checklist

1. [ ] Profiled in `--release` mode (dev builds are 10-100x slower)
2. [ ] `RgaaCriteria::all()` uses `OnceLock` not repeated allocation
3. [ ] `AxeMapper::map()` uses `IndexMap` not `HashMap` for deterministic output
4. [ ] `HoloClient` returns `RgaaError` not `String`
5. [ ] No `.unwrap()` or `.expect()` in production paths
6. [ ] `#[must_use]` on all `Result`-returning public functions

## Quick Commands

Run from `rgaa-rs/` (workspace root) — a `Makefile` wraps the common ones (`make build`, `make test`, `make check`, `make ci`).

```bash
# Build
cargo build --workspace
cargo build --workspace --release

# Test (nextest: parallel, better output; doctests still need plain `cargo test`)
cargo nextest run --workspace
cargo test --workspace --doc
cargo nextest run --workspace --no-capture

# Lint
cargo clippy --workspace --all-targets
cargo fmt --check

# E2E navigateur (nécessite le binaire Obscura 0.2.4 ; voir docs/testing-layers.md)
RUN_E2E=1 cargo nextest run -p rgaa-obscura -E 'binary(/^criterion_.+_coverage$/)'

# Harnais DOM des sondes JS (Node, sans navigateur, quelques secondes)
(cd crates/rgaa-rules/tests/js && npm ci && ./run-harnesses.sh)

# Profile
RUSTFLAGS="-C force-frame-pointers=yes" cargo build --profile profiling
cargo flamegraph --release
```

### Build acceleration

- **Toolchain**: pinned via `rgaa-rs/rust-toolchain.toml` (stable + clippy/rustfmt/rust-analyzer) so `rustup` auto-installs the right one.
- **sccache**: compilation cache, wired as `rustc-wrapper` in `~/.cargo/config.toml` (machine-global, not checked in) and via `mozilla-actions/sccache-action` in CI. Check hit rate with `sccache --show-stats`.
- **mold**: much faster linker than GNU `ld` for iterative local builds, wired via `rustflags` in `~/.cargo/config.toml` and `rui314/setup-mold` in CI. Requires the `mold` binary on `PATH` (see below).
- **cargo-nextest**: faster, parallel test runner (`cargo nextest run`) used locally and in CI in place of `cargo test` for non-doc tests.
- Machine-wide install (already run on this box): `rustup`, `cargo-binstall`, then `cargo binstall sccache cargo-nextest cargo-watch cargo-hack cargo-audit cargo-outdated cargo-edit`, plus the `mold` prebuilt release binary.
- System packages needed to build this workspace at all (matches CI): `sudo apt-get install -y protobuf-compiler cmake clang lld mold`.

## Agent skills

### Issue tracker

Issues live in GitHub Issues for jamon8888/Holo-RGAA. See `docs/agents/issue-tracker.md`.

### Triage labels

Default five-label vocabulary (needs-triage, needs-info, ready-for-agent, ready-for-human, wontfix). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: root `CONTEXT.md` + `docs/adr/`. See `docs/agents/domain.md`.

### Code review

Two bots review pull requests here, and only one of them runs on its own:

- **Sourcery** reviews automatically on every PR — a review verdict plus a "Reviewer's Guide" summarising the change.
- **CodeRabbit does *not* review automatically.** It declines on this repository because it has fewer than 10 stars ("This repository does not receive automatic reviews"), and posts only that notice. A CodeRabbit review has to be requested per PR, by commenting `@coderabbitai review`.

So the absence of CodeRabbit findings on a PR means nothing was asked, not that nothing was found. Decide per PR whether to request one; a small, well-tested change with a clean Sourcery review does not need it.

Whatever review does arrive, treat its inline comments and summary like feedback from a human reviewer: read them before merging, and address or explicitly dismiss each finding rather than ignoring it.

<!-- BEGIN basemind (managed by `basemind init`) -->

## basemind — prefer it over grep / read / git

basemind is this repo's indexed context layer. Prefer it BEFORE grep, before reading files to find structure, and before naked `git` — it's the default, not a preference. basemind returns paths, lines, and signatures at a fraction of the tokens of reading source.

### Routing

| Reach for | Instead of |
|---|---|
| `code` modes `symbols` / `definition` / `references` / `callers` / `implementations` / `grep`, `graph` mode `calls` | `grep` / `rg` / opening files to find a symbol |
| `code` mode `outline` (code files only) / `graph` mode `map` | reading whole files to learn their shape |
| `git` modes `recent` / `blame_symbol` / `touching` / `diff` | `git log` / `git blame` / `git diff` |
| `code` mode `find` (fuzzy path search) | `find` / `fd` / `ls -R` to locate a file by name |
| `agents` (modes `post` / `inbox` / `thread_list`) | assuming you're the only agent in the repo |
| `workspace` (modes `workspaces` / `worktrees` / `claim`) | editing a worktree another session may already own |
| `memory` mode `documents` (PDFs, Office, markdown, config and data files; the code map holds code only) / `web` (modes `scrape` / `crawl` / `map`) | manually reading PDFs / docs / config files or ad-hoc fetching |
| `code` mode `semantic` (then `chunk`) | keyword-only guessing at where a concept lives |

### Red flags — stop and re-route

- About to `grep` / `rg`? → `code` mode `grep`.
- About to open a file just to find a symbol? → `code` modes `outline` / `symbols`.
- About to `git log` / `git blame`? → `git` modes `recent` / `blame_symbol`.
- Already mapped a file with basemind? Don't re-read it.

### Setup & maintenance

- Install the basemind Claude Code plugin from its marketplace (`/plugin marketplace add Goldziher/basemind`, then install `basemind`).
- Keep basemind current: enable plugin auto-update, or update the binary regularly so the index format and tools stay in sync.
- Re-run `basemind init` (or `/bm-init`) after enabling new capabilities to refresh this block.

<!-- END basemind -->

### basemind: project-specific policy

The block above is managed by `basemind init` and gets overwritten on re-run; everything
below it is this project's own policy and survives that.

**Version.** Pin to the npm release, not whatever shipped at install time: a stale binary
silently loses features. We hit this directly — `basemind@0.23.1` was installed (7 minor
versions behind) and lacked the markdown/JSON/YAML/CSV → documents-tier routing that landed
in 0.29.0, so `reports/` and `research_notes/` were parsed as *code* (tree-sitter) and never
embedded or semantically searchable, with no error to say so. `npm install -g basemind@latest`
fixed it silently — no config change needed. Check `basemind --version` against `npm view
basemind version` occasionally; a mismatch this large is the kind of thing that breaks a whole
subsystem without ever raising.

**Config: `basemind.toml` (committed).** `[documents] embedding_preset = "balanced"`. Two
other presets were tried and rejected, and the reasons are worth keeping so nobody retries them
blind:
- `"quality"` (bge-large-en-v1.5, 335M params): ran 40+ minutes at 960%+ CPU on a CPU-only dev
  box for two small research notes. Correct output, wrong cost for this corpus.
- `"fast"`: hit a real bug in the then-installed 0.23.1 binary — `preset_dim("fast")` expects
  384 dims but the embedder actually produced 768, so every write was silently refused
  (`preset/runtime dim mismatch`) and the documents table stayed empty with no visible error.
  Confirmed fixed by the 0.30.0 upgrade; not re-verified since `"balanced"` already works.
- `[code_search] embed = false`: left off. BM25/keyword search over code is free with scanning;
  vector embeddings on code are a second ONNX model and a second full re-embed pass for a gain
  nobody has measured a need for here. Flip it on only with a concrete reason.

**Shared agent memory (`basemind memory`).** Visibility defaults to `group` — every agent
working this repo reads and writes the same notes; `individual` is this agent's own scratch,
invisible to others.
- `put` a durable fact nobody could re-derive from the code or git history in under a minute:
  an architectural decision and *why* (not just what), a measured number (a benchmark, a
  timeout that needed tuning), a dead end worth not retrying (a config that looked right and
  wasn't, a workaround for a tool bug). Tag it so `list`/`search` can filter later.
- Don't `put` what the repo already records (that's what `code`/`git` modes are for) or
  anything that only matters to the current task.
- Before starting unfamiliar work in this repo, `search` or `list` first — another agent (or a
  past session) may have already found the gotcha.
- Run `audit` occasionally (`dry_run: true` first) to catch notes that drifted from the code
  they describe; a note that no longer matches what it references is worse than no note.

**Keeping the index fresh.**
- `.claude/settings.json` (committed): a `PostToolUse` hook on `Write|Edit|MultiEdit` runs
  `basemind rescan <path>` incrementally after every file Claude touches, and a `SessionStart`
  hook runs a quiet full `basemind scan -q` to catch changes from outside Claude (`git pull`,
  another agent, a manual edit). Both are `async: true` (fire-and-forget, never block the tool
  call or session start) and silently no-op if `basemind` isn't on `PATH` — this repo must stay
  usable on a machine without it installed.
- `.git/hooks/pre-commit` (installed via `basemind hook install`, **not committed** — git
  hooks never are; re-run the install command on a fresh clone): runs `basemind scan --staged`
  before every commit, fail-open so a `basemind` problem never blocks a commit.
- None of this replaces running `basemind rescan` yourself after a large out-of-band change
  (a rebase, a branch switch) if you need the index current *right now* rather than on the
  next edit.
