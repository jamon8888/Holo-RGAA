# Policy Configuration Runbook

This runbook explains how to configure and enforce accessibility policies.

## Understanding Policy

A policy defines:

1. **Minimum compliance threshold** - What percentage of criteria must pass
2. **Required criteria** - Specific criteria that must pass regardless of overall score
3. **Evidence requirements** - What proof is needed for compliance

## Configuration File

Create `.rgaa/config.yaml`:

```yaml
policy:
  # Minimum compliance percentage (0-100)
  min_compliance: 80.0
  
  # Criteria that MUST pass
  required_criteria:
    - "1.1"    # Images have alt text
    - "5.1"    # Language is specified
    - "8.1"    # No duplicate content
    - "8.2"    # Page has language
    - "9.1"    # Information not conveyed by color alone
```

## Policy Levels

### Development Policy (Lenient)

```yaml
policy:
  min_compliance: 60.0
  required_criteria: []
```

Use during development to track progress without blocking builds.

### Staging Policy (Moderate)

```yaml
policy:
  min_compliance: 80.0
  required_criteria:
    - "1.1"
    - "5.1"
    - "8.2"
```

Use for staging deployments to ensure basic accessibility.

### Production Policy (Strict)

```yaml
policy:
  min_compliance: 90.0
  required_criteria:
    - "1.1"
    - "1.2"
    - "2.1"
    - "5.1"
    - "5.2"
    - "8.1"
    - "8.2"
    - "9.1"
    - "10.1"
    - "11.1"
```

Use for production releases to enforce high accessibility standards.

## Checking Compliance

### CLI

```bash
# Run audit and save bundle
rgaa audit analyze --url https://example.test --output audit-bundle.json

# Check against policy
rgaa audit policy --input audit-bundle.json
```

**Output:**

```
compliance 85.50% (minimum 80.00%): PASS
```

### Exit Codes

| Exit Code | Meaning |
|-----------|---------|
| `0` | Compliant |
| `1` | Non-compliant |

### Using in Scripts

```bash
#!/bin/bash
rgaa audit analyze --url "$1" --output bundle.json
if rgaa audit policy --input bundle.json; then
    echo "Compliant - proceeding with deployment"
else
    echo "Non-compliant - blocking deployment"
    exit 1
fi
```

## Required Criteria

### Critical Criteria by Category

**Images & Media:**
- `1.1` - Images have text alternatives
- `1.2` - Decorative images ignored

**Language & Text:**
- `5.1` - Page language specified
- `5.2` - Language changes identified

**Navigation:**
- `8.1` - No duplicate content
- `8.2` - Page has language

**Color & Contrast:**
- `9.1` - Information not by color alone

**Forms:**
- `10.1` - Form fields have labels

**Structure:**
- `11.1` - HTML structure correct

## Per-Environment Configuration

### Multiple Config Files

```bash
# .rgaa/config.dev.yaml
policy:
  min_compliance: 50.0

# .rgaa/config.staging.yaml
policy:
  min_compliance: 80.0

# .rgaa/config.prod.yaml
policy:
  min_compliance: 90.0
  required_criteria:
    - "1.1"
    - "5.1"
```

### Usage

```bash
# Development
rgaa audit analyze --config .rgaa/config.dev.yaml --url https://dev.example.test

# Staging
rgaa audit analyze --config .rgaa/config.staging.yaml --url https://staging.example.test

# Production
rgaa audit analyze --config .rgaa/config.prod.yaml --url https://example.test
```

## CI/CD Integration

### Blocking Deployments

```yaml
# GitHub Actions
- name: Check policy compliance
  run: |
    rgaa audit analyze --url ${{ env.AUDIT_URL }} --output bundle.json
    rgaa audit policy --input bundle.json
```

The workflow will fail if policy check fails.

### Non-Blocking (Warnings Only)

```yaml
- name: Run RGAA audit
  continue-on-error: true
  run: |
    rgaa audit analyze --url ${{ env.AUDIT_URL }} --output bundle.json
    rgaa audit policy --input bundle.json || true
```

## Policy Reporting

### Generate Compliance Report

```bash
rgaa audit report --input audit-bundle.json --format markdown --output compliance-report.md
```

### Policy Violations

Check which specific criteria failed:

```bash
# Extract failed criteria
cat audit-bundle.json | jq '.pages[].criteria[] | select(.status == "fail") | .criterion_id'
```

### Common Violations and Fixes

| Criterion | Common Issue | Fix |
|-----------|--------------|-----|
| `1.1` | Missing alt on images | Add `alt` attribute |
| `5.1` | Missing lang attribute | Add `lang="fr"` to `<html>` |
| `8.2` | Language change not marked | Use `<span lang="en">` |
| `9.1` | Color-only information | Add text/icon indicator |
| `10.1` | Missing form labels | Add `<label>` elements |
| `11.1` | Poor heading structure | Fix h1-h6 hierarchy |

## Policy Exceptions

For cases where full compliance isn't possible:

```yaml
policy:
  min_compliance: 85.0
  exceptions:
    - criterion: "1.3"
      reason: "Complex diagram - detailed description impractical"
      waiver_until: "2025-12-31"
```

> **Note:** Exceptions should be rare and time-limited.

## Monitoring Trends

Track compliance over time:

```bash
#!/bin/bash
# audit-trend.sh
DATE=$(date +%Y-%m-%d)
rgaa audit analyze --url "$1" --output "audit-$DATE.json"

RATE=$(cat "audit-$DATE.json" | jq '.taux_global')
echo "$DATE,$RATE" >> compliance-trend.csv
```

## Policy Checklist

- [ ] Define minimum compliance threshold
- [ ] Identify required criteria for your sector
- [ ] Set up per-environment configs
- [ ] Integrate policy checks in CI/CD
- [ ] Document exceptions with waivers
- [ ] Review and update quarterly
