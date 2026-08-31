# `/generate-report` — Generate Compliance Report

## Description

Generate a formatted RGAA compliance report from an existing audit bundle. Output in multiple formats for different audiences: JSON for tooling, Markdown for documentation, SARIF for CI/GitHub, JUnit for test systems, HTML for stakeholders.

## Prerequisites

- An existing audit bundle (from `/audit-site` or `/audit-project`)
- `rgaa-cli` for local generation, OR
- `rgaa-api` server for HTTP-based generation

## Usage

```
/generate-report [--format markdown|json|sarif|html|junit] [--input audit.json] [--output report]
```

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `--format` | string | No | Output format (default: `markdown`) |
| `--input` | string | No | Audit bundle file (default: latest audit) |
| `--output` | string | No | Output file path (default: stdout) |

## Examples

```
/generate-report
/generate-report --format html --output compliance.html
/generate-report --format sarif --output rgaa-results.sarif
/generate-report --format junit --output test-results.xml
/generate-report --input my-audit.json --format markdown
```

## Format Details

### Markdown (default)
Human-readable report with:
- Executive summary
- Compliance rate and status
- Findings by severity
- Evidence references
- Remediation recommendations

### JSON
Structured data for automation:
- Full audit bundle
- All findings with fingerprints
- Evidence file paths
- Metadata and timestamps

### SARIF 2.1.0
GitHub Code Scanning compatible:
```json
{
  "runs": [{
    "tool": { "driver": { "name": "RGAA", "version": "1.0.0" }},
    "results": [
      { "ruleId": "RGAA-1.1", "level": "error", ... }
    ]
  }]
}
```

### JUnit XML
CI system compatible:
```xml
<testsuite name="RGAA" tests="106" failures="18">
  <testcase name="RGAA-1.1" classname="images" status="passed"/>
  <testcase name="RGAA-11.1" classname="forms">
    <failure message="Form field missing label"/>
  </testcase>
</testsuite>
```

### HTML
Styled stakeholder report:
- Charts and visualizations
- Interactive filtering
- Exportable evidence
- Print-friendly layout

## Report Sections

A complete compliance report includes:

1. **Header** — Audit date, URL, tool version
2. **Executive Summary** — Compliance rate, status, critical issues
3. **Methodology** — How tests were run (deterministe/ia-assistee/manuel)
4. **Criteria Results** — Table of all 106 criteria with pass/fail
5. **Findings Detail** — Each failure with source location and evidence
6. **Evidence Index** — List of attached screenshots, AXTree dumps
7. **Remediation Plan** — Prioritized fix recommendations
8. **Compliance Statement** — Formal conformity declaration

## Compliance Statement

For formal reports, include:

```
Le site [URL] a obtenu un taux de conformité RGAA de [X]%.
L'audit a été réalisé selon la méthode et les outils définis par la procédure
de test RGAA 4.1.2. Les résultats sont valides pour la date du [DATE].
```

## CI/CD Integration

```yaml
# GitHub Actions
- name: Generate RGAA Report
  run: |
    rgaa audit analyze --url ${{ env.AUDIT_URL }} --output audit.json
    rgaa audit report --input audit.json --format sarif --output rgaa.sarif

- name: Upload SARIF to GitHub
  uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: rgaa.sarif
    category: RGAA-audit
```

## Output File Naming

| Format | Suggested Filename |
|--------|-------------------|
| Markdown | `rgaa-compliance-YYYY-MM-DD.md` |
| JSON | `rgaa-audit-YYYY-MM-DD.json` |
| SARIF | `rgaa-results.sarif` |
| JUnit | `rgaa-test-results.xml` |
| HTML | `rgaa-compliance-YYYY-MM-DD.html` |
