# remediate — RGAA Remediation Proposals

## Purpose

Generate approval-gated source-level patch proposals for accessibility findings. Produces framework-specific fixes (React, Vue, Angular, Next) with diffs, rationale, risk assessment, and validation commands.

**Key principle: Proposals require explicit approval before any source changes.**

## When This Skill Activates

- User asks "fix these findings", "generate remediation", "how do I fix [issue]"
- User selects findings from triage report for remediation
- User asks about a specific accessibility violation and wants a fix

## Workflow

1. **Load findings** — From triage report or direct input (max 25 per batch)
2. **Detect framework** — Auto-detect from source files (React, Vue, Angular, Next) or use override
3. **Generate proposals** — Call `remediate` MCP tool via `/remediate` command, or `rgaa audit verify` CLI
4. **Present approval request** — Each proposal includes diff, rationale, risks, approval token
5. **Wait for approval** — User confirms before any source changes
6. **Apply on approval** — Apply only after user confirms approval token

## Inputs

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `findings` | array | Yes | Findings to remediate (1-25) |
| `framework` | string | No | Override: `react`, `next`, `vue`, `angular` |
| `require_approval` | boolean | No | Default: `true` — always require approval |

## Finding Format

```json
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
```

## Proposal Response

```json
{
  "outcome": "ok",
  "issue_id": "fng_abc123",
  "explanation": "Add alt attribute with descriptive text",
  "steps": [
    "Locate the img element in src/components/Hero.tsx:42",
    "Add alt=\"Hero image showing team collaboration\" to the img tag"
  ],
  "confidence": "high",
  "criteria": ["1.1"],
  "proposal": {
    "proposal_id": "prop-abc123",
    "diff": "--- a/src/components/Hero.tsx\n+++ b/src/components/Hero.tsx\n@@ -40 +40 @@\n-<img src=\"hero.png\">\n+<img src=\"hero.png\" alt=\"Hero image showing team collaboration\">",
    "files": ["src/components/Hero.tsx"],
    "rationale": "Alt text provides textual alternative for screen readers and passes RGAA 1.1",
    "risks": ["None — purely additive change"],
    "validation_commands": ["npm run a11y:test", "npm test -- --testNamePattern=accessibility"],
    "expected_effect": "Passes RGAA 1.1",
    "approval_state": { "kind": "required" },
    "approval_token": "rgaa-approval-v1-sha256:abc123:required"
  }
}
```

## Approval States

| State | Meaning | Action |
|-------|---------|--------|
| `required` | Human review needed | Present diff, wait for approval |
| `not_required` | Safe, low-risk change | Auto-apply or offer one-click apply |
| `approved` | Previously approved | Apply immediately |

## Approval Flow

```
Claude: "I found 3 missing alt attributes. Here's the proposal for the first one:

PROPOSAL (prop-abc123)
File: src/components/Hero.tsx:42
Diff: <img src="hero.png"> → <img src="hero.png" alt="Hero image showing team collaboration">
Rationale: Alt text provides textual alternative for screen readers
Risk: None — purely additive change
Validation: npm run a11y:test

Approval token: rgaa-approval-v1-sha256:abc123:required

Approve? (yes/no)"
```

## Framework Adapters

| Framework | Handles | Examples |
|-----------|---------|----------|
| **React/Next** | JSX, `alt`, `aria-label`, semantic HTML | `<img alt={...}>` |
| **Vue** | Template syntax, `v-bind:alt`, ARIA | `<img :alt="description">` |
| **Angular** | Property bindings, ARIA attributes | `<img [alt]="description">` |
| **Ambiguous** | Cannot detect framework | Returns `NeedsReview` error |

## Batch Processing

- Process findings in batches of 1-25
- Split larger sets into multiple approval rounds
- Track proposal status across batches

## Error Handling

| Error | Meaning | Response |
|-------|---------|----------|
| `INSUFFICIENT_CONTEXT` | Cannot locate source element | Return error, suggest manual fix |
| `UNSUPPORTED_FRAMEWORK` | Framework not detected | Return `NeedsReview` per finding |
| `POLICY_DENIED` | Approval required but not granted | Block application |
| `MODEL_FAILURE` | LLM failed to generate proposal | Return error, preserve finding |

## Related Skills

- `triage` — Source of categorized findings
- `verify` — Validate fixes after application
- `report` — Document remediation work
