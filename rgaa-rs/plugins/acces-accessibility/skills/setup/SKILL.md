---
name: setup
description: This skill should be used when the user asks to "set up rgaa-mcp", "install the RGAA accessibility tooling", "configure the RGAA MCP server", "connect rgaa-mcp", "why is analyze failing", "obscura not found", or runs /acces-accessibility:setup. It installs the rgaa-mcp server and the pinned obscura browser binary, wires them into the user's MCP client (Claude Code, opencode, Cursor, VS Code, Claude Desktop), and verifies the installation in three stages that each isolate one failure.
argument-hint: "[client] (optional: claude-code | opencode | cursor | vscode | claude-desktop)"
allowed-tools: AskUserQuestion, Bash, Read, Edit, Write, Glob, Grep
---

# setup — install and verify rgaa-mcp

## What has to be true

Three tiers of dependency, not one install. Say which tier the user has reached;
tier 0 is a useful degraded mode, **not** a failure.

| Tier | Requires | Tools that work |
|---|---|---|
| 0 | the `rgaa-mcp` binary | `list_criteria`, `lint_static` |
| 1 | + `obscura` **exactly 0.2.2** | `analyze`, `igt`, `source_map`, `verify_fix` |
| 2 | + an LLM key *or* a local provider | `audit_url` |

Tier 1 is what the remediation loop needs. Tier 2 is only for full
106-criteria audits, and it is the only tier that costs anything.

## Install

**Prefer the repository's installer.** If `install.sh` exists at the repository
root, run it — it installs `rgaa-mcp`, `rgaa-cli`, the pinned `obscura`, and
the plugin, and it version-checks `obscura` itself. Do **not** restate its steps
in your own words: a second description of the install drifts from the real one,
and the cost is an inexplicable "it won't start".

```bash
./install.sh
```

**Only when there is no `install.sh`** — the plugin can be installed on its own,
without the repository — fall back to installing by hand:

```bash
cargo install --path rgaa-rs/crates/rgaa-mcp
```

and install `obscura` 0.2.2 from `h4ckf0r0day/obscura`, then either put it on
`PATH` or set `RGAA_OBSCURA_BIN` to its absolute path.

The version is not advisory. `rgaa-obscura` compares the binary's reported
version against a pinned constant and **fails fast on mismatch**, so a drifted
`obscura` reports itself instead of misbehaving quietly.

## Verify in three stages

Run them in order. Each isolates one failure, so a failure names its own cause.
Stop at the first one that fails and fix that, rather than reporting "it doesn't
work".

**Stage 1 — `list_criteria`.** No arguments, no browser, no network, no LLM.
Proves the server starts and its catalogue loads. Expect 106 criteria.
A failure here is the binary: not installed, not on `PATH`, or not starting.

**Stage 2 — `analyze`** on any URL the user can reach. Free: `analyze` makes no
LLM call. Proves the `obscura` binary is present *and* at the pinned version.
A failure here is `obscura`: missing, not on `PATH`, `RGAA_OBSCURA_BIN` pointing
somewhere stale, or a version mismatch — the error says which.

Without this stage, a missing `obscura` surfaces only at the first loop round,
far from its cause. Do not skip it because it looks redundant.

**Stage 3 — `audit_url`. Opt-in, and priced.** This is the only call that spends
tokens: 22 batched LLM calls per page. Tell the user the cost and get their
agreement before running it. Skip it entirely if they only want the remediation
loop, which never touches the LLM.

## The LLM key is not ambiguous

Do not ask the user to diagnose this, and do not treat a blank key as an error
by default. Read `RGAA_LLM_PROVIDER` and decide:

- **A local provider** (`ollama`, `lmstudio`, `vllm`, `custom`) needs **no key**.
  A blank key is the correct configuration. Local routes also default to
  unlimited RPM and a 600-second per-request timeout, because CPU inference of a
  7B–14B model takes minutes — a slow local run is not a failure.
- **A remote provider** needs one, in `RGAA_LLM_API_KEY` or the provider's own
  variable (`MYIA_API_KEY`, `OPENAI_API_KEY`, `GROQ_API_KEY`, …).

An empty value is read as unset for every variable, so a blank plugin setting
falls through to `.env` rather than shadowing it. Setting the plugin's
`llm_api_key` to nothing is safe.

If the user only wants the remediation loop, they need **no key at all**: say so
instead of walking them through credentials they will never use.

## Clients

Inside **Claude Code** there is almost nothing to do: this plugin's own
`.mcp.json` wires the server when the plugin is installed. Verify with stage 1
and move on.

The skill's real work is the other clients. For each, the server is a stdio
process invoking `rgaa-mcp` with no arguments; what differs is the file and the
key names.

- **opencode** — `opencode.json`, under `mcp`, `{"type": "local", "command": ["rgaa-mcp"]}`
- **Cursor** — `.cursor/mcp.json`
- **VS Code** — `.vscode/mcp.json`, under `servers`
- **Claude Desktop** — the `claude_desktop_config.json` for the user's platform

Read the client's existing config before writing: these files usually already
hold other servers, and replacing one is worse than failing to add one. Add a
key, never rewrite the file.

Pass `RGAA_OBSCURA_BIN` in the server's `env` when `obscura` is not on `PATH`.

## If the sibling plugin is also installed

`rgaa-accessibility` declares its own `rgaa-mcp`, so with both plugins installed
two namespaced servers run (`plugin_acces-accessibility_rgaa-mcp` and
`plugin_rgaa-accessibility_rgaa-mcp`).

**Report it; do not act on it.** Disabling another plugin's server is not this
skill's business, and the duplicate is harmless — the same binary, started
lazily. Tell the user it is expected, and that the two plugins divide the work:
this one drives the remediation loop, the sibling owns the 106-criteria audit
and compliance reports.

## What to tell the user when you are done

Name the tier reached, and what it does and does not buy. If they stopped at
tier 0 or 1, do not describe the installation as incomplete — say which tools
work now and what the next tier would add.

Never imply that a working installation means the site is RGAA conformant.
That claim needs all 106 criteria, and the loop reaches 66.
