# Runbooks Index

This directory contains workflow guides for common RGAA auditing tasks.

## Getting Started

| Runbook | Description |
|---------|-------------|
| [01-quickstart](01-quickstart.md) | Run your first audit in 5 minutes |
| [02-ci-integration](02-ci-integration.md) | Integrate audits into CI/CD |
| [03-policy-configuration](03-policy-configuration.md) | Set compliance thresholds |
| [04-remediation-workflow](04-remediation-workflow.md) | Fix accessibility issues |
| [05-guided-testing](05-guided-testing.md) | Run guided manual tests |

## Workflow Overview

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Audit     │────▶│  Analyze    │────▶│  Remediate  │
│   (automated)    │  Findings   │     │  Issues     │
└─────────────┘     └─────────────┘     └─────────────┘
       │                                       │
       ▼                                       ▼
┌─────────────┐                         ┌─────────────┐
│   Policy    │                         │  Verify     │
│   Check    │                         │  Fixes      │
└─────────────┘                         └─────────────┘
```

## Common Workflows

### 1. Full Audit Pipeline

```
audit analyze → audit report → audit policy
```

1. Run full audit: `rgaa audit analyze --url https://example.test`
2. Generate report: `rgaa audit report --input bundle.json`
3. Check policy: `rgaa audit policy --input bundle.json`

### 2. Fix and Verify

```
audit analyze → extract issues → audit verify → apply fixes → re-audit
```

1. Run audit to find issues
2. Extract failed criteria as remediation issues
3. Generate patches: `rgaa audit verify --issues issues.json`
4. Apply approved patches
5. Re-run audit to confirm fixes

### 3. CI/CD Pipeline

```
git push → run audit → check policy → (pass/fail)
```

1. Push triggers GitHub Actions
2. Action runs `rgaa audit analyze`
3. Policy check gates deployment
4. Non-compliant? Block and report

### 4. Guided Testing

```
audit igt --test keyboard-navigation
```

Run structured manual tests for criteria that require human judgment.

## Next Steps

- Read the [CLI Reference](../cli/README.md) for all commands
- Read the [API Reference](../api/README.md) for HTTP integration
- Read the [MCP Reference](../mcp/README.md) for AI assistant integration
