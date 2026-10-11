---
name: setup
description: This skill should be used when the user asks to "set up rgaa-mcp", "install the RGAA accessibility tooling", "configure the RGAA MCP server", "connect rgaa-mcp", "why is analyze failing", "obscura not found", or runs /acces-accessibility:setup. It installs the rgaa-mcp server and the pinned obscura browser binary, wires them into the user's MCP client (Claude Code, opencode, Cursor, VS Code, Claude Desktop), and verifies the installation in four stages that each isolate one failure.
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
root, run it — it installs `rgaa-mcp`, `rgaa-cli` and the pinned `obscura`, and
version-checks `obscura` itself. Do **not** restate its steps in your own words:
a second description of the install drifts from the real one, and the cost is an
inexplicable "it won't start".

Note what it does **not** do: `install.sh` symlinks only the sibling
`rgaa-accessibility` plugin into `~/.claude/plugins/` (it hardcodes
`rgaa-rs/plugins/rgaa-consultant`). It does not install **this** plugin, which
comes from the marketplace:

```
/plugin install acces-accessibility --marketplace jamon8888/Holo-RGAA
```

In practice a user running this skill already has the plugin — that is how the
skill reached them. What `install.sh` gives them is the **runtime**: the server
and the browser binary.

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

## Verify in four stages

Run them in order. Each isolates one failure, so a failure names its own cause.
Stop at the first one that fails and fix that, rather than reporting "it doesn't
work".

**Stage 0 — the advertised tool set.** Before anything else, check what the
server advertises. It must offer **nine** tools: `analyze`, `remediate`, `igt`,
`audit_url`, `get_audit_result`, `list_criteria`, `lint_static`, `source_map`,
`verify_fix`.

**A shorter list means a stale binary, and it is not a cosmetic problem.** An
installed server predating `source_map` cannot run the remediation loop at all:
`remediate` rejects every issue whose `source_locations` is empty, and
`source_map` is the only thing that fills it. The individual calls in the stages
below all still succeed on such a server, which is exactly why this check comes
first — a stale binary passes every other test.

**This happens, and a plain `cargo install` does not fix it.** There are two
install mechanisms that disagree, and they write to different directories:

| Mechanism | Writes to | Source of the binary |
|---|---|---|
| `install.sh` | `~/.local/bin` | a **prebuilt release tarball** |
| `cargo install --path …` | `~/.cargo/bin` | the **local source tree** |

On a typical setup `~/.local/bin` comes **first** on `PATH`, so if `install.sh`
has ever run, its release binary shadows anything `cargo install` builds. A
release that lags the source then stays in charge, and `cargo install --force`
appears to succeed while changing nothing.

So always check **which** binary answers:

```bash
command -v rgaa-mcp          # the one PATH resolves
ls -l ~/.local/bin/rgaa-mcp ~/.cargo/bin/rgaa-mcp 2>/dev/null
```

To replace the one that actually wins, install into that prefix explicitly:

```bash
cargo install --path rgaa-rs/crates/rgaa-mcp --root "$HOME/.local" --force
```

Then **re-run stage 0** rather than assuming it worked. Do not move or delete a
binary on the user's machine without asking — tell them what shadows what and
let them choose.

`tests/e2e-local.sh` in this plugin performs this check along with the rest of
the contract, and reports which binary it reached. Run it when a user says
something "worked yesterday", or when a reinstall seems to have had no effect.

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

## Running against a local project needs two environment variables

Neither is documented anywhere else, and the loop **cannot run locally** without
them. Both were established by driving the real loop, not by reading code.

**`OBSCURA_ALLOW_PRIVATE_NETWORK=1`** — `obscura` blocks loopback, RFC1918 and
link-local addresses by default, as a deliberate SSRF fix. Without it, `analyze`
on `http://localhost:…` returns `completed: false` and
`cdp_transport: Access to private/internal IP address 127.0.0.1 is not allowed`.

There is a trap on top of it. `rgaa-obscura` starts its CDP server on a
**hardcoded port 9222** and **reuses a server already listening there**, and it
never passes `--allow-private-network` itself — the flag only arrives through the
inherited environment of a server it spawns. So if any obscura server is already
running (they outlive their parent and get reparented to `init`), your variable
has **no effect at all** and the block persists with no hint as to why.

Diagnose it before concluding the variable is wrong:

```bash
ss -ltnp | grep 9222        # is a server already bound?
pgrep -ax obscura           # started with which flags, by whom?
```

An idle pool with no established connections can be retired, and the next call
spawns a fresh server that inherits the variable. Never kill one that is in use,
and never match on `obscura serve` with `pkill -f` — the pattern matches your own
shell.

**`RGAA_SOURCE_MAP_ROOTS`** — `source_map` confines `source_root` to a list of
allowed roots, defaulting to the **server's own current working directory**
(`source_map.rs:840-848`). A project outside it is refused with
`INVALID_INPUT: … resolves outside the allowed roots`. For a plugin-launched
server, that cwd is wherever the client started it, which is often not the
project you want to map.

## Confirm before you change anything

This plugin gates its own patches on human approval. Setup is held to the same
standard, and the actions below are not reversible by the user without effort:

- **Before running `install.sh`**, say what it will install and where. It writes
  binaries into `~/.local/bin`.
- **Before writing to an MCP client config**, show the exact change and ask.
  These files hold the user's other servers.
- **Never move, overwrite or delete a binary** on the user's machine. If one
  shadows another, say which shadows which and let the user choose.
- **Never edit `PATH`** or shell startup files.

Reading to diagnose — `command -v`, listing the two known binary paths, printing
a config file — needs no permission. Writing does.

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
