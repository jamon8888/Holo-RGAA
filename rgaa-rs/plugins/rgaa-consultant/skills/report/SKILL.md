# report — RGAA Compliance Reporting

## Purpose

Generate formatted compliance reports from audit bundles. Output in multiple formats for different audiences: JSON for tooling, Markdown for documentation, SARIF for CI, HTML for stakeholders, JUnit for test systems.

## When This Skill Activates

- User asks "generate a report", "export compliance documentation"
- Audit is complete and findings need to be documented
- User needs evidence for a client or regulatory submission
- CI/CD pipeline needs SARIF/JUnit output

## Output Formats

| Format | Audience | Use Case |
|--------|----------|----------|
| **JSON** | Dev tools, automation | Further processing, ticket creation |
| **Markdown** | Developers, documentation | GitHub PRs, Confluence, docs |
| **SARIF 2.1.0** | Security/GitHub | GitHub Code Scanning integration |
| **JUnit XML** | CI systems | Jenkins, CircleCI test results |
| **HTML** | Stakeholders | Shareable compliance reports |

## Report Sections

A complete compliance report includes:

1. **Executive Summary** — Compliance rate, status, critical issues
2. **Audit Metadata** — Date, URL, auditor, tool version
3. **Criteria Results** — All 106 criteria with pass/fail/needs-review
4. **Findings Detail** — Each failure with evidence references
5. **Evidence Attachments** — Screenshots, AXTree dumps, fingerprints
6. **Remediation Plan** — Prioritized fix recommendations
7. **Methodology Notes** — How each tier was tested

## SARIF Output (GitHub Code Scanning)

```json
{
  "version": "2.1.0",
  "runs": [{
    "tool": {
      "driver": {
        "name": "RGAA",
        "version": "1.0.0",
        "informationUri": "https://github.com/jamon8888/Holo-RGAA"
      }
    },
    "results": [
      {
        "ruleId": "RGAA-1.1",
        "level": "error",
        "message": { "text": "Image missing alt attribute" },
        "locations": [{
          "physicalLocation": {
            "artifactLocation": { "uri": "src/components/Hero.tsx" },
            "region": { "startLine": 42 }
          }
        }]
      }
    ]
  }]
}
```

## JUnit XML Output

```xml
<testsuite name="RGAA" tests="106" failures="18">
  <testcase name="RGAA-1.1" classname="images" status="passed"/>
  <testcase name="RGAA-11.1" classname="forms">
    <failure message="Form field missing label">
      Missing &lt;label&gt; for input id="email"
    </failure>
  </testcase>
</testsuite>
```

## Markdown Report Template

```markdown
# RGAA Compliance Report

**URL:** https://example.test  
**Date:** 2026-08-30  
**Taux Global:** 72.4%  
**Status:** Partiellement Conforme

## Summary

| Category | Passed | Failed | Needs Review |
|----------|--------|--------|--------------|
| Images | 7 | 2 | 0 |
| Tables | 5 | 0 | 1 |
| Forms | 8 | 3 | 1 |
| Navigation | 6 | 2 | 2 |

## Critical Findings

### RGAA 11.1 — Form Fields Have Labels

**Severity:** Critical  
**Status:** Failed

Form input `id="email"` has no associated label.

**Evidence:** screenshot-001.png  
**Source:** src/forms/ContactForm.tsx:23

### RGAA 1.1 — Images Have Text Alternatives

**Severity:** Major  
**Status:** Failed

Hero image `src="hero.png"` missing alt attribute.

**Evidence:** screenshot-002.png  
**Source:** src/components/Hero.tsx:42
```

## Compliance Statement

For formal reports, include the standard conformity statement:

```
Le site [URL] a obtenu un taux de conformité RGAA de [X]%.
L'audit a été réalisé selon la méthode et les outils définis par la procédure
de test RGAA 4.1.2. Les résultats sont valides pour la date du [DATE].
```

## Example Interactions

```
User: "Generate a SARIF report for our GitHub workflow"
Claude: → Runs audit if needed
Claude: → Converts bundle to SARIF 2.1.0
Claude: → Saves as rgaa-results.sarif
Claude: → "Report ready. Configure GitHub Code Scanning to ingest this file."

User: "Create an HTML compliance report for management"
Claude: → Runs full audit
Claude: → Generates styled HTML with charts
Claude: → "Report saved as compliance-2026-08-30.html"

User: "I need a full-site compliance report"
Claude: → Runs /audit-url on the starting URL
Claude: → Retrieves results via get_audit_result MCP tool
Claude: → Generates formatted report in requested format
```

## CI/CD Usage

```yaml
# GitHub Actions
- name: RGAA Compliance Report
  run: |
    rgaa audit analyze --url ${{ env.AUDIT_URL }} --output audit.json
    rgaa audit report --input audit.json --format sarif --output rgaa.sarif

- name: Upload SARIF
  uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: rgaa.sarif
```

## Related Skills

- `audit` — Source of audit bundle
- `triage` — Prioritized findings for the report
- `verify` — Post-fix verification reports
