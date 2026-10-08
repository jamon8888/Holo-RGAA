# MyIA LLM as the default RGAA evaluator

## Goal

Replace Holo3 as the application's default hosted LLM with the OpenAI-compatible
MyIA endpoint for every RGAA evaluation path. Use `swift-1.5-27b` as the
default model; accept `qwen3.6-35b-a3b` as an operator-selected alias. Preserve
the existing ability to select other providers explicitly.

## Current architecture

`rgaa-core::provider::LlmSettings` is the shared configuration resolver used
by both `rgaa-agent` (Rig) and `rgaa-holo` (chat transport). This is the right
place to change the default route so both paths receive the same provider,
base URL, key, model, timeout, and completion parameters. Separate legacy
paths remain in the direct `HoloClient` constructor and in the TUI's keyring
setup/status commands; documentation and integration instructions also still
describe Holo3 as the default.

The RGAA audit phases and criteria coverage remain unchanged. Only the remote
LLM service configuration and its user-facing identity change.

## Design

1. Add a first-class `myia` provider preset to the shared provider registry:
   - Base URL: `https://api.medium.text-generation-webui.myia.io/v1`
   - Provider key variable: `MYIA_API_KEY`
   - Default model: `swift-1.5-27b`
   - Hosted API behavior and timeout/rate-limit defaults.
2. Make `myia` the fallback provider when `RGAA_LLM_PROVIDER` is unset. Keep
   the model and base URL overridable through the existing generic variables.
   `RGAA_LLM_MODEL` may explicitly select either the primary model or the
   supplied alias; no alias rewriting is needed because the endpoint receives
   the configured model identifier.
3. Update the direct legacy `HoloClient` defaults and provider identity so no
   runtime evaluation path silently contacts H Company. Keep public Rust type
   and crate names where they are compatibility-facing, and preserve explicit
   configuration of other providers.
4. Update the TUI setup, keyring lookup/storage compatibility, environment
   examples, CLI/MCP help, integration-test skip messages, and current user
   documentation so new installations configure `MYIA_API_KEY` and show
   MyIA/model defaults. Existing `HOLO3_*` configuration remains a migration
   fallback only where it is needed to avoid breaking already configured
   installations; it must not override an explicitly selected provider.
5. Never place the supplied API key in source, examples, logs, or a tracked
   file. Provide configuration through the existing keyring flow and
   `MYIA_API_KEY` environment variable. Do not change or inspect the existing
   project `.env` file as part of the implementation.

## Scope boundaries

- Do not change RGAA criteria, prompts, routing policy, result merging, or
  non-LLM audit mechanisms.
- Do not rename the `rgaa-holo` crate or public `HoloClient` type in this
  change; those names are internal compatibility surfaces, not the selected
  runtime provider.
- Do not remove other provider presets.
- Do not embed the API key in the repository or commit it.

## Acceptance checks

- An unset `RGAA_LLM_PROVIDER` resolves to MyIA with the endpoint and default
  model above, and requires `MYIA_API_KEY` (or the generic primary
  `RGAA_LLM_API_KEY`).
- The alias can be selected through `RGAA_LLM_MODEL` and reaches the API
  unchanged.
- Both the Rig agent path and chat transport path use the resolved MyIA
  settings; the direct legacy constructor defaults to the same endpoint/model.
- The TUI can store and retrieve the MyIA credential without confusing it
  with a legacy Holo3 credential, while existing user configuration remains
  readable according to the migration rule above.
- No runtime defaults or current setup instructions still direct a fresh
  installation to Holo3.
- The supplied credential is absent from tracked files and logs.

## Verification approach

Review changed files and configuration diffs for consistency across the
provider resolver, agent, transport, TUI, examples, and current documentation.
No live API request is part of the change. Automated tests or build checks will
only be run if separately requested.
