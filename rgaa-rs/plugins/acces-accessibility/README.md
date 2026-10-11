# Acces Accessibility

RGAA tooling discipline for coding agents. This plugin drives the `rgaa-mcp`
server through a remediation loop and teaches the field mappings that silently
return wrong answers when got wrong.

It is the **tooling** layer. The sibling plugin `rgaa-accessibility` is the
**domain** layer, and the boundary between them is a rule:

> The domain layer says what a criterion requires. The tooling layer says
> whether our instruments can decide it, and what their silence proves.

So criterion knowledge — what RGAA 4.1.2 demands, how a criterion is tested,
its WCAG 2.2 and EN 301 549 cross-references — lives in `rgaa-accessibility`.
Instrument knowledge — which tool decides which criterion, what a *missing*
finding proves, how the tools chain together — lives here.

## Install

On Claude Code 2.1.275 or later this works in one step — it is the line
`claude plugin validate` itself emits:

```
/plugin install acces-accessibility --marketplace jamon8888/Holo-RGAA
```

On an older client, register the marketplace first and install by qualified
name:

```
/plugin marketplace add jamon8888/Holo-RGAA
/plugin install acces-accessibility@holo-rgaa
```

Then run `/acces-accessibility:setup` to install and verify the server.

## What it covers, and what it does not

Counted from `engine_plan.json`, the 106 RGAA criteria divide by primary engine:

| Engine | Criteria | Reachable by this plugin's loop |
|---|---|---|
| `AxeCore` | 13 | yes |
| `Deterministic` | 53 | yes |
| `Holo` (LLM) | 32 | **no** — only `audit_url` evaluates these |
| `Human` | 8 | **no** — requires a person |

**A clean loop is not RGAA conformity**, and the real figure is sharper than
the table above. The loop *reaches* 66 criteria, but it can only conclude
**conforming** for **32** of the 106: another 34 are partially covered, where
these tools find violations but silence proves nothing, because a residual
judgement remains. So **74 of 106 cannot be called conforming** on the strength
of a clean run.

Every report this plugin produces names that residue rather than implying a
conformance verdict. This is a regulatory context; the distinction is not
pedantry. Per-criterion detail lives in a generated coverage map, which arrives
with the `usage` skill.

For the full 106-criteria audit and compliance reports, use the
`rgaa-accessibility` plugin.

## Cost

Only `audit_url` spends tokens — 22 batched LLM calls per page. Everything the
remediation loop uses (`analyze`, `source_map`, `remediate`, `verify_fix`,
`igt`, `lint_static`, `list_criteria`) makes **no LLM call at all**. The loop
therefore iterates at zero marginal cost, and the knob worth tuning
(`RGAA_TACTICAL_RPM`) buys **latency, not price**.

## Requirements, in three tiers

| Tier | Requires | Gives you |
|---|---|---|
| 0 | the `rgaa-mcp` binary | `list_criteria`, `lint_static` — source linting, sub-millisecond per file, no browser |
| 1 | + `obscura` **0.2.2** exactly | `analyze`, `igt`, `source_map`, `verify_fix` — the whole loop |
| 2 | + an LLM key *or* a local provider | `audit_url` — the 106-criteria audit |

Tier 0 is a useful degraded mode, not a failure. The `obscura` version is
checked at runtime and a mismatch fails fast, so a drifted binary reports
itself rather than misbehaving.

Run `/acces-accessibility:setup` to install and verify, or `install.sh` from
the repository root, which installs and version-checks all of it.

## Running the loop against a local project

Two environment variables are required, and nothing else documents them:

```bash
export OBSCURA_ALLOW_PRIVATE_NETWORK=1   # obscura blocks loopback by default (SSRF fix)
export RGAA_SOURCE_MAP_ROOTS=/path/to/project   # source_map is confined to the server's cwd
```

The first has a trap: `obscura`'s CDP server uses a hardcoded port 9222 and
**reuses one already listening there**, so a server started earlier without the
flag keeps blocking localhost and your variable has no effect. Check with
`ss -ltnp | grep 9222`. See `skills/setup/SKILL.md`.

## Skills

| Skill | Purpose |
|---|---|
| `setup` | Install and verify the server across Claude Code, opencode, Cursor, VS Code and Claude Desktop |

Three more land on top of this one: `usage` (the tool contract and its
references), `audit-loop` (the approval-gated remediation loop) and
`generate-instructions`.

## Verifying an installation

```bash
./tests/plugin-contract.sh          # structure, no network
```

## License

MIT, as the rest of this repository.
