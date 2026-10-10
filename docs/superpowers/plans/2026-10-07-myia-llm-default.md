# MyIA LLM Default Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make MyIA's OpenAI-compatible endpoint and `swift-1.5-27b` the default LLM route throughout the RGAA program, while accepting the supplied model alias and retaining explicit provider selection.

**Architecture:** Add MyIA to the shared `rgaa-core::provider` registry and change the common default, so both the Rig agent and generic chat transport resolve the same endpoint, model, key, timeout, and completion settings. Update the standalone legacy `HoloClient`, TUI setup, runtime provenance, and current installation/help documentation to match. Keep compatibility-facing Rust module/type names and explicit providers.

**Tech Stack:** Rust workspace, `rgaa-core::LlmSettings`, `rgaa-agent::AgentConfig`, `rgaa-holo::ChatTransport`, `rgaa-tui` keyring/TUI, Markdown and shell installation documentation.

**Spec:** `docs/superpowers/specs/2026-10-07-myia-llm-default-design.md`

## Global Constraints

- Use `https://api.medium.text-generation-webui.myia.io/v1` as the MyIA base URL.
- Use `swift-1.5-27b` as the default model and accept `qwen3.6-35b-a3b` unchanged when explicitly configured.
- Never place the supplied API key in source, examples, logs, or a tracked file.
- Do not inspect or change the existing project `.env` file.
- Preserve other provider presets and the compatibility-facing `rgaa-holo` crate and `HoloClient` type names.
- Keep changes limited to LLM configuration and its current user-facing identity; leave RGAA criteria and audit policy unchanged.
- Do not add new tests or run tests/build checks unless the user separately asks for verification. Existing assertions that encode the old default may be updated to reflect the new default.
- Do not modify historical research, completed plans, archived audits, or unrelated untracked files.

## Review Focus

- Explicit `RGAA_LLM_PROVIDER` always selects the requested provider and is not overwritten by a legacy `HOLO3_*` variable; align existing resolver assertions for this case without running them.
- An unset provider with `MYIA_API_KEY` resolves to MyIA, while the old provider-specific key remains a deliberate migration path only when no provider is explicitly selected; align existing resolver expectations without running them.
- The model alias is sent to the endpoint exactly as configured, with no silent normalization; verify this by inspecting the model-selection/transport path, without a live request.
- The TUI and direct `HoloClient` defaults point at `/v1` consistently, while the transport alone adds `/chat/completions`; inspect the endpoint construction path.
- No credential value is introduced into tracked content, logs, debug output, or the plan/spec; inspect the diff and changed-file list.

---

### Task 1: Make MyIA the shared provider default

**Files:**
- Modify: `rgaa-rs/crates/rgaa-core/src/provider.rs`
- Modify: `rgaa-rs/crates/rgaa-agent/src/config.rs`

**Interfaces:**
- Consumes: Existing `Provider`, `LlmSettings::from_env_prefixed`, and `AgentConfig::from_llm_settings` interfaces.
- Produces: A `myia` provider preset with base URL `https://api.medium.text-generation-webui.myia.io/v1`, key variable `MYIA_API_KEY`, and default model `swift-1.5-27b`; unset primary provider resolves to `myia` unless a legacy Holo3-only environment is being migrated.

- [ ] Add `myia` to `PROVIDERS` as a hosted, key-required provider with the approved endpoint, `MYIA_API_KEY`, and default model.
- [ ] Change the provider resolver's no-provider default to `myia`; keep an explicit `RGAA_LLM_PROVIDER` authoritative, and route a legacy-only `HOLO3_*` configuration to the historical `holo3` preset only when no provider was explicitly selected.
- [ ] Ensure legacy `HOLO3_BASE_URL` and `HOLO3_MODEL` values are consulted only for that migration route; they must not change an explicitly selected MyIA or other provider.
- [ ] Update resolver and `AgentConfig` documentation/default fields to describe MyIA and the two supported model identifiers.
- [ ] Update existing in-file default/migration assertions that encode the previous implicit provider, preserving assertions for deliberate explicit Holo3 compatibility; do not execute them.
- [ ] Review the resulting resolver branches against the existing primary/fallback route behavior and confirm the fallback route does not inherit primary credentials.

### Task 2: Align all runtime LLM clients and provenance

**Files:**
- Modify: `rgaa-rs/crates/rgaa-holo/src/client.rs`
- Modify: `rgaa-rs/crates/rgaa-holo/src/backend.rs`
- Modify: `rgaa-rs/crates/rgaa-holo/src/transport.rs`
- Modify: `rgaa-rs/crates/rgaa-holo/src/fallback.rs` (align existing direct-client fallback assertions)
- Modify: `rgaa-rs/crates/rgaa-holo/src/ollama.rs` (make the hosted-provider contrast generic)
- Modify: `rgaa-rs/crates/rgaa-agent/src/agent.rs`
- Modify: `rgaa-rs/crates/rgaa-agent/src/prompts.rs`
- Modify: `rgaa-rs/crates/rgaa-agent/src/lib.rs`
- Modify: `rgaa-rs/crates/rgaa-core/src/completion.rs`
- Modify: `rgaa-rs/crates/rgaa-core/src/error.rs`
- Modify: `rgaa-rs/crates/rgaa-agent/src/error.rs`
- Modify: `rgaa-rs/crates/rgaa-orchestrator/src/pipeline.rs`
- Modify: `rgaa-rs/crates/rgaa-report/src/report.rs`
- Modify: `rgaa-rs/crates/rgaa-report/src/sources.rs`
- Modify: `rgaa-rs/crates/rgaa-mcp/src/server.rs`

**Interfaces:**
- Consumes: Task 1's shared MyIA provider settings.
- Produces: The standalone `HoloClient`'s default endpoint/model and backend identity agree with MyIA; generic agent and transport paths keep using the shared resolved settings; generated results and MCP descriptions identify the configured LLM generically or as MyIA.

- [ ] Change only the standalone `HoloClient` defaults from the H Company URL/model to the approved MyIA `/v1` URL and `swift-1.5-27b`; make its backend provenance label `myia` and keep `with_base_url` behavior intact.
- [ ] Confirm generic `ChatBackend` and Rig client construction continue to consume `LlmSettings`; do not introduce a second hardcoded MyIA configuration in those paths.
- [ ] Replace runtime comments, prompt API descriptions, and user-facing error strings that falsely identify every provider as Holo3; retain public error variant/type names when renaming them would break API compatibility.
- [ ] Change result source labels and MCP help text from a hard-coded `holo3` source to the MyIA/provider-neutral identity used by the runtime.
- [ ] Inspect `rgaa-holo/src/fallback.rs` and cassette backend behavior; leave explicit Holo3 fixture/provider cases intact unless they describe the implicit default.
- [ ] Review endpoint assembly to ensure `/chat/completions` is appended exactly once for both the MyIA base URL and the legacy direct client.

### Task 3: Update TUI setup and credential naming

**Files:**
- Modify: `rgaa-rs/crates/rgaa-tui/src/keyring.rs`
- Modify: `rgaa-rs/crates/rgaa-tui/src/commands.rs`
- Modify: `rgaa-rs/crates/rgaa-tui/src/tui/setup.rs`

**Interfaces:**
- Consumes: Task 1's MyIA provider identity and `MYIA_API_KEY` naming.
- Produces: New TUI setup text and defaults identify MyIA; new credentials use MyIA-specific keyring/storage names; explicit environment variables remain the effective runtime configuration.

- [ ] Change the setup wizard's default base URL to `https://api.medium.text-generation-webui.myia.io/v1` and update prompts to ask for the MyIA API key.
- [ ] Store new TUI credentials under a MyIA-specific OS keyring item and use `MYIA_API_KEY`/MyIA URL names in the existing plain-text fallback format; preserve read access to legacy Holo3 storage only as an explicitly labeled migration path.
- [ ] Ensure a legacy Holo3 credential is never silently presented as a MyIA credential or used as `MYIA_API_KEY`.
- [ ] Update `config show` text to identify stored MyIA credentials accurately and continue masking all key values.
- [ ] Keep help text clear that audits resolve the active route from environment configuration; do not imply the keyring-backed value is automatically consumed by the audit pipeline when it is not.
- [ ] Inspect every display/error path in the setup flow to confirm no entered key is printed unmasked after confirmation.

### Task 4: Update current installation surfaces and aligned existing expectations

**Files:**
- Modify: `.env.example`
- Modify: `README.md`
- Modify: `AGENTS.md`
- Modify: `CLAUDE.md`
- Modify: `install.sh`
- Modify: `docs/rgaa-plugin-install.md`
- Modify: `rgaa-rs/docs/README.md`
- Modify: `rgaa-rs/docs/cli/README.md`
- Modify: `rgaa-rs/plugins/rgaa-consultant/CONNECTORS.md`
- Modify: `rgaa-rs/plugins/rgaa-consultant/README.md`
- Modify: `rgaa-rs/plugins/rgaa-consultant/skills/audit/SKILL.md`
- Modify: `rgaa-rs/plugins/rgaa-consultant/skills/criteria/SKILL.md`
- Modify: `rgaa-rs/crates/rgaa-holo/Cargo.toml`
- Modify: `rgaa-rs/crates/rgaa-agent/tests/completion_params.rs` (only if an existing fixture hardcodes the old implicit default)
- Modify: `rgaa-rs/crates/rgaa-agent/tests/integration.rs` (only update key-variable skip text/selection; do not run it)
- Modify: `rgaa-rs/crates/rgaa-orchestrator/tests/full_audit.rs` and `rgaa-rs/crates/rgaa-orchestrator/tests/obscura_audit.rs` (update only credential descriptions)
- Modify: `rgaa-rs/crates/rgaa-rules/examples/static_data_snapshot.rs` (update only its live-pipeline credential description)
- Modify: `rgaa-rs/grille-rgaa-confirmee-partielle.csv` (update current criterion notes that name the active LLM)

**Interfaces:**
- Consumes: Tasks 1–3's provider name, endpoint, model, and key environment variable.
- Produces: Fresh setup instructions consistently direct users to MyIA and `MYIA_API_KEY`; optional model override documents both accepted identifiers.

- [ ] Update `.env.example` with MyIA as the default provider, a blank `MYIA_API_KEY`, the model and endpoint defaults, and accurate notes on generic overrides and legacy migration. Leave `.env` untouched.
- [ ] Update the current project README, CLI/API docs, install instructions, plugin connector docs, and consultant skill text so a fresh installation no longer targets Holo3.
- [ ] Update top-level agent guidance where it describes current provider defaults; do not rewrite historical research or completed design/plan documents.
- [ ] Update package metadata and examples that describe the active remote backend.
- [ ] Update current criterion planning data that identifies the active model; leave historical research and archived audit artifacts unchanged.
- [ ] Keep explicit Holo3 provider compatibility examples and migration notes where technically meaningful; remove only statements that present Holo3 as the current implicit default.
- [ ] Align existing test fixture literals/skip messages only where they assume the old default or key name; add no tests and run none.
- [ ] Search current source, templates, installer and user documentation for `api.hcompany.ai`, the old model ID, and fresh-install Holo3 defaults; inspect each remaining match to distinguish intentional legacy support from stale active instructions.
- [ ] Review the final changed-file diff and status, ensuring no user-created untracked files or credential-bearing files were changed.

## Completion Notes

The user has not requested test/build verification. Completion is based on source/configuration diff review only; report that no tests or build checks were run. The repository's `.git` directory is read-only in this environment, so do not attempt to stage or commit changes.
