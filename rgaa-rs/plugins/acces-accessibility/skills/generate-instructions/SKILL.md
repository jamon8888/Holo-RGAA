---
name: generate-instructions
description: This skill should be used when the user asks to "generate accessibility instructions", "add RGAA instructions to my repo", "bake accessibility into my coding agent", "write copilot-instructions for accessibility", "set up CLAUDE.md for RGAA", "add Cursor rules for a11y", or runs /acces-accessibility:generate-instructions. It writes or updates a repository's own agent-instruction files (CLAUDE.md, .github/copilot-instructions.md, Cursor rules, AGENTS.md) with the rgaa-mcp trap rules that an agent gets wrong unaided.
argument-hint: "[targets] (optional: claude | copilot | cursor | agents | all)"
allowed-tools: AskUserQuestion, Glob, Read, Edit, Write
---

# generate-instructions — bake the trap rules into a repository

## What this writes, and what it deliberately does not

Write **only what an agent gets wrong unaided**: the trap rules below.

Do **not** restate the loop's steps. An instruction file spends context budget
in someone's repository, in every session, indefinitely. The loop's steps are
discoverable from the tool descriptions; the traps are not — which is exactly
what makes them dangerous. Writing down what is already discoverable charges
permanent rent for redundancy.

If the user asks for the full workflow anyway, say what it costs them and let
them decide.

## The gate you are writing is advisory. Say so.

These files are **text**, not a mechanism. A generated instruction saying "ask
before applying a patch" is worth exactly as much as the reading agent's
compliance, and no more.

Do not add a hook to enforce it, and do not imply the file enforces anything:

- A Claude Code hook reaches **one target in four**. Copilot and Cursor never
  read it, so the protection would cover a quarter of the surface while reading
  as general.
- A `PreToolUse` hook on `Edit|Write` gates **every** edit in the repository,
  not just RGAA remediation.
- Hooks have already failed silently in this project. `rgaa-consultant`'s
  `check-runtime.sh` carries a header documenting a bug where the hook never
  ran at all. A gate that silently no-ops is worse than an absent one, because
  it is believed.

**What to write instead:** require that `remediate`'s `approval_token` be shown
before a patch is applied. The token re-derives from the proposal's identity, so
a patch altered after approval is detectable and a skipped approval is
**visible**. That is weaker than a gate. State it as such in the generated text —
a reader who takes it for enforcement has been misled.

## The trap rules to write

Keep these tight. Each one is a rule an agent breaks by default, and each fails
**silently** — no error, a plausible-looking wrong answer.

1. **`remediate`'s `element_html` needs framework source, not rendered DOM.**
   Passing `analyze`'s `html` fails every time. Plain HTML and PHP templates
   cannot be remediated at all — fix those by hand.
2. **`source_map` is mandatory between `analyze` and `remediate`.**
   `source_locations` defaults to `[]`, and `[]` fails the issue.
3. **`analyze`'s response shape switches on `config.igt_tools`.**
   With it set, findings move from top-level `findings` to `data.axe`. Reading
   `findings` then yields nothing, with no error.
4. **Match findings across rounds on `rule` + `target` + `criterion_id`**, copied
   verbatim. Never on `id` — it is positional per scan. Never on the fingerprint
   — it carries a per-scan DOM hash, so it moves on any page change. All three
   parts are needed: one axe violation often maps to two RGAA criteria, so
   `rule` + `target` alone merges two findings silently.
5. **A finding that vanished from a positional or runtime-generated selector is
   unstable, not fixed.** `:nth-child`, carousels and build-generated class
   names move between scans with no edit at all.
6. **Reference findings go in top-level `bundle.findings` only.** Findings in
   `pages[].findings` are dropped from the diff; the call still succeeds and
   reports nothing.
7. **Never put `status: "pass"` findings in a reference bundle.** Omitting them
   keeps the "new" category meaning only genuinely-new.
8. **A clean loop is not RGAA conformity.** 40 of the 106 criteria are
   structurally out of reach — 32 need the LLM (`audit_url` only), 8 need a
   person. Report that residue; never imply a conformance verdict.
9. **Only `audit_url` spends tokens.** The whole remediation loop is free, so
   iterate it freely — and announce the cost before any `audit_url`.

## Targets

| Argument | File |
|---|---|
| `claude` | `CLAUDE.md` |
| `copilot` | `.github/copilot-instructions.md` |
| `cursor` | `.cursor/rules/rgaa-accessibility.mdc` |
| `agents` | `AGENTS.md` (the generic target; opencode and others read it) |

With no argument, detect which of these already exist and offer those, plus
`AGENTS.md` as the portable default. Do not create all four unasked — an
unread instruction file is pure context cost.

## Writing into a file that already exists

Assume the file exists and is hand-maintained. Most are.

**Use a delimited section, and make it idempotent:**

```markdown
<!-- BEGIN acces-accessibility -->
... generated rules ...
<!-- END acces-accessibility -->
```

- If the markers are present, **replace what is between them**. Never append a
  second copy.
- If they are absent, append the section at the end of the file.
- Never rewrite or reorder the rest of the file. The surrounding content is
  someone's hand-written work and is none of your business.

**Show a diff and get approval before writing.** This plugin gates its own
patches on human approval; writing into a repository's instruction files without
the same courtesy would be inconsistent. Refusing to write and printing a diff
for the user to apply themselves is *not* the safer option — it moves the work
without making anything safer.

## Before you finish

State, in one line each:

- which files you changed, and whether you created or amended them
- that the rules are advisory, not enforced
- that `rgaa-mcp` must be installed for any of it to apply — point at
  `/acces-accessibility:setup` if it is not
