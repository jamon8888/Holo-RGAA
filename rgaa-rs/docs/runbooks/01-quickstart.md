# Quick Start Runbook

This runbook gets you from zero to your first RGAA audit in 5 minutes.

## Prerequisites

- [ ] `rgaa-cli` installed (see [Installation](../README.md))
- [ ] `obscur`a browser binary installed
- [ ] Web access to the target site

## 1. Verify Installation

```bash
rgaa --version
```

Expected output: `rgaa 0.1.0` or similar.

## 2. Run Your First Audit

```bash
rgaa audit analyze --url https://example.test
```

This runs a full RGAA audit against the URL and outputs results as JSON.

## 3. Understanding the Output

```json
{
  "audit_id": "aud_abc123",
  "url": "https://example.test",
  "taux_global": 85.5,
  "etat_conformite": "partielle",
  "passed": 45,
  "failed": 8,
  "na": 53
}
```

| Field | Description |
|-------|-------------|
| `taux_global` | Overall compliance percentage |
| `etat_conformite` | Compliance state: `conforme`, `partielle`, or `non_conforme` |
| `passed` | Criteria that passed |
| `failed` | Criteria that failed |
| `na` | Not applicable criteria |

## 4. Generate Different Report Formats

```bash
# HTML report
rgaa audit analyze --url https://example.test --format html --output report.html

# Table output
rgaa audit analyze --url https://example.test --format table

# SARIF for GitHub integration
rgaa audit analyze --url https://example.test --format sarif --output results.sarif
```

## 5. Check Compliance Against Policy

```yaml
# .rgaa/config.yaml
policy:
  min_compliance: 80.0
  required_criteria:
    - "1.1"
    - "5.1"
```

```bash
rgaa audit analyze --url https://example.test --output bundle.json
rgaa audit policy --input bundle.json
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| `browser unavailable` | Install Obscura and set `RGAA_OBSCURA_BIN` |
| `network error` | Check internet connectivity and URL |
| `timeout` | Increase timeout with `--timeout` flag |

## Next Steps

- [ ] Read the [CLI Reference](../cli/README.md) for all commands
- [ ] Set up [CI integration](./02-ci-integration.md) for automated audits
- [ ] Configure [policy thresholds](./03-policy-configuration.md) for your organization
