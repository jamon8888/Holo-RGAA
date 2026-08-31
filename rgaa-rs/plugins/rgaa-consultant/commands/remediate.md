# `/remediate` — Generate Remediation Proposals

## Description

Generate approval-gated source-level patch proposals for accessibility findings. Produces framework-specific fixes (React, Vue, Angular, Next) with diffs, rationale, risk assessment, and validation commands.

**Key principle: Proposals require explicit approval before any source changes.**

## Prerequisites

- `rgaa-mcp` MCP server connected (recommended), OR
- `rgaa-cli` installed with `rgaa audit verify` subcommand

## Usage

```
/remediate [--findings <path>] [--framework <react|vue|angular|next>]
```

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `--findings` | string | Yes* | Path to JSON file with findings, OR inline JSON (*or pipe findings from triage) |
| `--framework` | string | No | Override framework detection: `react`, `next`, `vue`, `angular` |

## Finding Format

```json
[
  {
    "id": "fng_abc123",
    "rule": "image-alt",
    "element_html": "<img src=\"hero.png\">",
    "page_url": "https://example.test",
    "source_locations": [
      { "file": "src/components/Hero.tsx", "line": 42 }
    ],
    "summary": "Image missing alt attribute",
    "criteria": ["1.1"],
    "framework": "react"
  }
]
```

## Examples

```
/remediate --findings findings.json
/remediate --findings findings.json --framework react
/audit-site https://example.test → /triage → /remediate
```

## Workflow

1. **Load findings** — 1–25 per batch from file or prior triage
2. **Detect framework** — Auto-detect or use `--framework` override
3. **Generate proposals** — Call `remediate` MCP tool
4. **Present approval request** — Each proposal shows diff, rationale, risks
5. **Wait for approval** — User confirms before any source changes
6. **Apply on approval** — Apply only after user confirms approval token

## Proposal Output

```
PROPOSAL (prop-abc123)
══════════════════════════════════════
Finding:   RGAA 1.1 — Image missing alt
File:      src/components/Hero.tsx:42
──────────────────────────────────────
Diff:
- <img src="hero.png">
+ <img src="hero.png" alt="Hero image showing team collaboration">

Rationale: Alt text provides textual alternative for screen readers
Risk:      None — purely additive change
Validation: npm run a11y:test
──────────────────────────────────────
Approval required — token: rgaa-approval-v1-sha256:abc123

Approve? (yes/no/count)
```

## Batch Processing

Process 1–25 findings per batch. For larger sets:
1. Claude presents proposals in groups
2. You approve each group before the next
3. Track proposal status across batches

## Approval States

| State | Meaning | Action |
|-------|---------|--------|
| `required` | Human review needed | Present diff, wait for approval |
| `not_required` | Safe, low-risk change | Offer one-click apply |
| `approved` | Previously approved | Apply immediately |

## CI/CD Usage

```bash
# Generate proposals from audit
rgaa audit analyze --url https://example.test --output audit.json

# Apply approved fixes
rgaa audit verify --issues fixes.json
```

## Error Handling

| Error | Meaning | Response |
|-------|---------|----------|
| `INSUFFICIENT_CONTEXT` | Cannot locate source element | Suggest manual fix |
| `UNSUPPORTED_FRAMEWORK` | Framework not detected | Return NeedsReview |
| `POLICY_DENIED` | Approval required but not granted | Block application |
```