# Rust Best Practices for AI Coding Agents

Comprehensive rules for writing idiomatic, fast, and safe Rust code. Current for Rust 1.80+ (2024 edition).

## Core Principles

- **Measure first, optimize hot paths, validate every change** — never optimize without profiling evidence
- **Treat allocations as a primary performance metric** — allocation churn is the most common, most fixable bottleneck
- **Return `Result<T, E>` instead of panicking** for recoverable errors
- **Borrow over clone** — prefer `&T` and `&[T]` to avoid unnecessary heap allocations

---

## Project: rgaa-rs

### Workspace Structure

```
rgaa-rs/
  Cargo.toml              # Workspace root (10 crates)
  crates/
    rgaa-core/            # Domain types + criteria catalog (106 RGAA criteria)
    rgaa-rules/           # axe-core violation mapping + gap-fix JS snippets
    rgaa-holo/            # Holo3 LLM client for AI-assisted evaluation
    rgaa-browser-tools/   # Browser automation via CDP (AXTree, BrowserSession, MCP server)
    rgaa-agent/           # Rig-based agentic evaluator (dual model routing, rate limiter)
    rgaa-orchestrator/    # Pipeline orchestration (wires agent + browser + rules)
    rgaa-storage/         # PostgreSQL storage — EMPTY
    rgaa-api/             # Axum HTTP API — EMPTY
    rgaa-mcp/             # MCP server — EMPTY
    rgaa-cli/             # CLI interface — EMPTY
```

### Key Dependencies

- `tokio` (async runtime), `reqwest` (HTTP), `serde`/`serde_json` (serialization)
- `anyhow` (app errors), `thiserror` (library errors), `tracing`/`tracing-subscriber` (logging)
- `regex-lite` (lightweight regex for JSON extraction)

### Conventions

- Unit-struct pattern for stateless services: `struct AxeMapper;` with associated functions
- `RgaaError` enum (thiserror) with `type Result<T>` alias for library errors
- Serde derives on all data types for JSON round-tripping
- French domain terminology for RGAA concepts, English for code identifiers
- Structured tracing with context fields: `info!(attempt, max_retries, "message")`

---

## CRITICAL: Ownership & Borrowing

- Prefer `&T` borrowing over `.clone()` — cloning is cheap until it isn't
- Accept `&[T]` not `&Vec<T>`, `&str` not `&String`
- Use `Cow<'a, T>` for conditional ownership (clone only when needed)
- Use `Arc<T>` for thread-safe shared ownership, `Rc<T>` for single-threaded
- Use `RefCell<T>` for interior mutability single-threaded, `Mutex<T>` across threads
- Use `RwLock<T>` when reads significantly outnumber writes
- Implement `Copy` for small, simple types; use explicit `Clone` for costly copies
- Move large types instead of copying; use `Box` if moves are expensive
- Rely on lifetime elision rules; add explicit lifetimes only when required

### Codebase-Specific

- **Double clone in `AxeMapper::map()`** (`crates/rgaa-rules/src/axe_mapper.rs`): Each insertion clones the key twice — once for HashMap key, once for struct field. Restructure to clone once or use references.
- **Clone in violation matching** (`axe_mapper.rs:38-40`): `violation.id`, `violation.impact`, `violation.description` cloned per match. Consider borrowing or restructuring.

---

## CRITICAL: Error Handling

- Use `thiserror` for library error types, `anyhow` for application error handling
- Return `Result<T, E>` instead of panicking for recoverable errors
- Add context with `.context()` or `.with_context()` for error chains
- Avoid `unwrap()` in production code; use `?`, `expect()`, or handle errors
- Use `expect()` only for invariants that indicate bugs, not user errors
- Use `?` operator for clean propagation
- Implement `From<E>` for error conversions to enable `?` operator
- Preserve error chains with `#[source]` or `source()` method
- Start error messages lowercase, no trailing punctuation
- Document error conditions with `# Errors` section in doc comments

### Codebase-Specific

- **`HoloClient::new()` uses `.expect()`** (`crates/rgaa-holo/src/client.rs:68`): `Client::builder().build().expect("Failed to create HTTP client")` will panic if TLS init fails. Change to return `Result<Self, RgaaError>`.
- **`HoloClient::evaluate()` returns `Result<_, String>`** (`client.rs`): Should return `Result<HoloResponse, RgaaError>` using the defined error type. The `RgaaError::Holo3` variant exists but is unused.
- **`AxeMapper::map()` uses `.unwrap_or_default()`** (`axe_mapper.rs:13`): `serde_json::from_str(violations_json).unwrap_or_default()` silently returns empty vec on malformed JSON. Add logging or return `Result`.
- **Add `From<reqwest::Error> for RgaaError`** to enable `?` in HoloClient methods.

---

## CRITICAL: Memory Optimization

- Use `with_capacity()` when size is known — avoids repeated reallocation
- Use `SmallVec<[T; N]>` for usually-small collections (inline N, then heap)
- Use `ArrayVec<T, N>` for fixed-capacity collections that never heap-allocate
- Box large enum variants to reduce overall enum size
- Use `clone_from()` to reuse allocations when repeatedly cloning
- Clear and reuse collections instead of creating new ones in loops
- Avoid `format!()` when string literals work
- Use `write!()` into existing buffers instead of `format!()` allocations
- Use `mem::take` / `mem::replace` to move values out of `&mut` without cloning
- Know and control drop order: struct fields drop top-to-bottom, locals in reverse

### Codebase-Specific

- **`RgaaCriteria::all()` allocates 106 `Criterion` structs every call** (`crates/rgaa-core/src/criteria.rs:14-123`): This is called by `find()`, `deterministic()`, `ia_assiste()`, `count()`. Use `lazy_static!` or `OnceLock<Vec<Criterion>>` to allocate once.
- **`get_base_criterion()` allocates Strings per match arm** (`crates/rgaa-holo/src/prompts.rs:216-222`): Single integer match vs dotted IDs ("1.1", "13.12") appears incompatible. Verify this function is correct.

---

## CRITICAL: Unsafe Code

- Write a `// SAFETY:` comment above every `unsafe` block
- Keep `unsafe` blocks as small as possible — only the operation that requires unsafety
- Run `cargo miri test` in CI for every crate that contains `unsafe` code
- Use `MaybeUninit<T>` for uninitialized memory; never `mem::uninitialized()`
- In Rust 2024, wrap `extern` blocks in `unsafe extern { }`
- Document invariants when manually implementing `Send` or `Sync`

---

## HIGH: Async/Await

- Never hold `Mutex`/`RwLock` across `.await` points
- Use `spawn_blocking` for CPU-intensive work in async contexts
- Use `tokio::fs` instead of `std::fs` in async code
- Use `CancellationToken` for graceful shutdown and task cancellation
- Use `join!` or `try_join!` for concurrent independent futures
- Use bounded channels to apply backpressure and prevent unbounded memory growth
- Clone Arc/Rc data before await points to avoid holding references across suspension
- Use native `async fn` in traits (stable 1.75) instead of `async_trait` macro
- Ensure futures used in `tokio::select!` branches are cancellation-safe

### Codebase-Specific

- **Only `rgaa-holo/client.rs` is async** — the rest of the codebase is sync. Keep the async boundary clean.
- **Retry logic** (`client.rs:95-159`): Exponential backoff up to 5 attempts, 429 rate limit handling. Pattern is correct but should use `RgaaError` instead of `String` errors.

---

## HIGH: Concurrency

- Use rayon's `par_iter()` for CPU-bound data parallelism (sufficient work per element required)
- Use `std::thread::scope` to borrow stack data across threads
- Use the weakest correct memory `Ordering` for every atomic operation
- Prefer `thread_local!` with `Cell`/`RefCell` over `static mut`
- Always use bounded channels in long-running services

---

## HIGH: Performance

### Build Configuration

```toml
[profile.release]
opt-level = 3          # Maximum optimization
lto = "fat"            # Whole-program LTO
codegen-units = 1      # Single codegen unit, better optimization
panic = "abort"        # Smaller binary, no unwind tables
strip = "symbols"      # Remove symbols, smaller binary

[profile.profiling]
inherits = "release"
debug = "line-tables-only"   # Minimal debug info for profiler
strip = "none"               # Preserve symbols for profiler
```

### Quick Wins

| Optimization | Effort | Impact |
|---|---|---|
| Switch global allocator (mimalloc/jemalloc) | 2 lines | 0-15% |
| Enable LTO + `codegen-units = 1` | 3 lines | 0-20% |
| `target-cpu=native` | 1 env var | Variable |
| Replace `HashMap` hasher (FxHash/AHash) | Drop-in | 0-20% |
| `Vec::with_capacity` / `reserve` | Trivial | Variable |
| Reuse collections with `.clear()` | Small refactor | Variable |
| `clone_from` instead of `= clone()` | Trivial | Variable |
| Iterators instead of indexed access | Trivial | Variable |

### Allocation Reduction

- Preallocate `Vec`/`String`/`HashMap` when size is known
- Avoid allocating intermediate collections just to iterate them again
- Avoid `format!` in hot paths — use `write!` into existing buffer
- Use `read_line()` with reusable `String` instead of `BufRead::lines()`
- Declare collections outside loops and `.clear()` between iterations

### Hot Loop Optimization

1. Use iterators — `for x in slice` has no bounds checks
2. Use `chunks_exact()` — tells compiler chunk size, enables vectorization
3. Slice once, index the sub-slice — helps optimizer reason about lengths
4. Add assertions — `assert!(idx < slice.len())` before loop lets compiler elide checks
5. Last resort: `get_unchecked()` — requires `// SAFETY:` comment and measured win

### Profiling Tools

| Tool | Best for |
|---|---|
| `samply` | Sampling profiler, Firefox Profiler output, easiest to start |
| `cargo flamegraph` | Flame graph visualization of CPU hotspots |
| `perf` | Linux hardware counters, detailed CPU analysis |
| DHAT / `dhat-rs` | Heap allocation profiling, hot allocation sites |

**Always profile release builds** — dev builds are 10-100x slower.

---

## MEDIUM: Type Safety

- Wrap IDs in newtypes: `UserId(u64)` — parse don't validate
- Use `Option<T>` for values that might not exist, `Result<T, E>` for operations that can fail
- Use `PhantomData` to express type relationships without runtime cost
- Avoid stringly-typed APIs — use enums, newtypes, or validated types
- Use `Display` for user-facing output and `Debug` for diagnostics

### Codebase-Specific

- **Criterion IDs**: RGAA uses dotted notation ("1.1", "13.12"). `get_base_criterion()` uses single integers. Verify compatibility or create a proper `CriterionId` newtype.

---

## MEDIUM: Collections

- Default to `Vec`; use `VecDeque` for queue/deque; avoid `LinkedList`
- Pick map by access pattern: `HashMap` (fast, unordered), `BTreeMap` (sorted), `IndexMap` (insertion order)
- Use `HashSet`/`BTreeSet` for membership tests, not linear `Vec::contains`
- Use `BinaryHeap` for priority queue or repeated max-extraction
- Use `Entry` API for map insert-or-update to avoid repeated lookups

### Codebase-Specific

- **`AxeMapper::map()` uses `HashMap`** — iteration order is non-deterministic. For reproducible test output and audit results, use `IndexMap` (insertion order) or `BTreeMap` (sorted by key).

---

## MEDIUM: Testing

- Put unit tests in `#[cfg(test)] mod tests { }` within each module
- Use `use super::*;` in test modules to access parent module items
- Put integration tests in the `tests/` directory
- Use descriptive test names that explain what is being tested
- Structure tests with clear Arrange, Act, Assert sections
- Use `criterion` for benchmarking, `proptest` for property-based testing
- Use `#[tokio::test]` for async tests
- Keep documentation examples as executable doctests

### Codebase-Specific

- **Only `rgaa-holo` has tests** (`client.rs` and `prompts.rs`). Add tests for:
  - `rgaa-core`: `RgaaCriteria::all()`, `find()`, `deterministic()`, `ia_assiste()`
  - `rgaa-rules`: `AxeMapper::map()`, `GapFixRules::snippets()`
  - `rgaa-holo`: `PromptBuilder::build()` end-to-end tests
- **No async tests exist**: `HoloClient::evaluate()` is async but has no `#[tokio::test]` tests
- **No benchmarks**: Add `criterion` benchmarks for `AxeMapper::map()` and `RgaaCriteria::all()`
- **No integration tests**: Create `tests/` directories for cross-crate integration

---

## MEDIUM: Documentation

- Document all public items with `///` doc comments
- Use `//!` for module-level documentation
- Include `# Examples` with runnable code
- Include `# Errors` section for fallible functions
- Include `# Panics` section for functions that can panic
- Include `# Safety` section for unsafe functions
- Use `?` in examples, not `.unwrap()`
- Use intra-doc links to reference types and items

---

## MEDIUM: Observability

- Use `tracing` for structured, span-aware diagnostics instead of `println!`
- Libraries emit through the tracing/log facade and never install a subscriber
- Record structured key-value fields, not values interpolated into the message string
- Use `#[tracing::instrument]` and spans to attach context to async tasks
- Log errors with their full source chain, and log each error exactly once
- Never log secrets or PII — redact or skip them

### Codebase-Specific

- **Good**: `rgaa-holo/client.rs` uses structured tracing with `info!(attempt, max_retries, "message")`.
- **Add**: `#[tracing::instrument]` on `evaluate()` and `PromptBuilder::build()`.

---

## LOW: Clippy & Linting

- Enable `#![deny(clippy::correctness)]` for correctness lints
- Enable `clippy::suspicious`, `clippy::style`, `clippy::complexity`, `clippy::perf`
- Run `cargo fmt --check` in CI
- Configure lints at workspace level for consistent enforcement
- Enable `unexpected_cfgs` and declare known cfgs to catch feature-gate typos

### Codebase-Specific

- Add `#[must_use]` on `AxeMapper::map()`, `GapFixRules::snippets()`, `HoloClient::evaluate()`.
- Run `cargo clippy --workspace --all-targets` in CI.

---

## Anti-Patterns to Avoid

| Anti-pattern | Fix |
|---|---|
| `.unwrap()` in production | Use `?`, `expect()`, or handle errors |
| `.clone()` when borrowing works | Pass `&str`/`&[T]`, use `Cow`, or `Arc<str>` |
| `format!()` / `.to_string()` in hot paths | `write!()` into reusable buffer, or `itoa`/`ryu` |
| `Box<dyn Trait>` in hot paths | Generics, `impl Trait`, or enum dispatch |
| `collect()` mid-iterator-chain | Chain more iterators |
| `Vec::new()` inside hot loops | Declare outside, `.clear()` |
| `HashMap` with default hasher on int keys | `FxHashMap` or `AHashMap` |
| Blocking sync I/O in async tasks | `spawn_blocking`, async I/O, or dedicated runtime |
| Unbounded queues in servers | Bounded channels + backpressure |
| `#[inline]` without measurement | Always benchmark before/after |
| `RgaaCriteria::all()` called repeatedly | Use `OnceLock` or `lazy_static` for static data |
| `Result<_, String>` in libraries | Use `RgaaError` with `thiserror` |
| `.unwrap_or_default()` on parse errors | Log or return error; don't silently drop failures |
| Non-deterministic `HashMap` ordering | Use `IndexMap` or `BTreeMap` for reproducible output |

---

## Rule Application by Task

| Task | Primary Rule Prefixes |
|------|----------------------|
| New function | `own-`, `err-`, `name-`, `pat-` |
| New struct/API | `api-`, `type-`, `conv-`, `doc-` |
| Async code | `async-`, `own-` |
| Concurrency | `conc-`, `async-`, `own-` |
| Unsafe code | `unsafe-`, `type-`, `test-` |
| Error handling | `err-`, `api-`, `pat-` |
| Memory optimization | `mem-`, `own-`, `perf-` |
| Performance tuning | `opt-`, `mem-`, `perf-` |
| Code review | `anti-`, `lint-` |

---

## Sources

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [The Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Rust 2024 Edition Guide](https://doc.rust-lang.org/edition-guide/rust-2024/)
- [The Rustonomicon](https://doc.rust-lang.org/nomicon/)
- [Rust Design Patterns](https://rust-unofficial.github.io/patterns/)
- Real-world codebases: ripgrep, tokio, serde, polars, axum, cargo
