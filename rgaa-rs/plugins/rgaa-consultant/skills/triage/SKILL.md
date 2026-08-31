# triage — RGAA Findings Triage

## Purpose

Filter, deduplicate, prioritize, and categorize audit findings for actionable remediation planning. Groups findings by root cause, estimates fix complexity, and routes to the appropriate fix path.

## When This Skill Activates

- Audit results contain 10+ findings
- User asks "prioritize these findings", "what should I fix first", "triage the audit"
- User wants findings grouped by severity or category

## Workflow

1. **Load findings** — From audit bundle or direct input
2. **Deduplicate** — Group by fingerprint (`rgaa-fp-v1-*`) across pages
3. **Classify severity** — Critical, Major, Minor based on criterion importance
4. **Estimate effort** — Quick (5min), Medium (30min), Large (2hr+)
5. **Group by category** — Images, Forms, Navigation, Language, Color, Structure
6. **Route to fix path** — `remediate` for code fixes, `guided-test` for manual checks

## Severity Framework

| Severity | Criteria | Examples |
|----------|----------|----------|
| **Critical** | Affects multiple user groups | Missing language, no skip links, form without labels |
| **Major** | Clear WCAG failure | Missing alt on informative images, low contrast text |
| **Minor** | Cosmetic or edge case | Decorative images without empty alt, redundant links |

## Effort Estimation

| Effort | Types |
|--------|-------|
| **Quick (5 min)** | Add `alt` text, `aria-label`, `lang` attribute |
| **Medium (30 min)** | Fix heading hierarchy, label associations, color contrast |
| **Large (2 hr+)** | Restructure navigation, rework forms, template changes |

## Output Format

```json
{
  "triage_report": {
    "total_findings": 23,
    "unique_issues": 18,
    "groups": [
      {
        "category": "images",
        "severity": "major",
        "effort": "quick",
        "count": 5,
        "criterion_ids": ["1.1", "1.2"],
        "sample_finding": {...},
        "fix_path": "remediate"
      },
      {
        "category": "forms",
        "severity": "critical",
        "effort": "medium",
        "count": 3,
        "criterion_ids": ["11.1", "11.2"],
        "sample_finding": {...},
        "fix_path": "remediate"
      },
      {
        "category": "navigation",
        "severity": "critical",
        "effort": "large",
        "count": 2,
        "criterion_ids": ["12.1", "12.2"],
        "sample_finding": {...},
        "fix_path": "guided-test"
      }
    ],
    "priority_order": [
      "forms (critical, medium effort)",
      "navigation (critical, large effort)",
      "images (major, quick)"
    ]
  }
}
```

## Triage Report Usage

After triage:
- **Quick wins first** — 5-minute fixes that resolve multiple findings
- **Critical blockers** — Issues blocking manual testing
- **Batch by effort** — Group similar fixes for efficient remediation
- **Route appropriately** — Code fixes via `remediate`, manual checks via `guided-test`

## Example Interactions

```
User: "Triage the last audit findings"
Claude: → Groups 23 findings into 6 categories
Claude: → Presents priority order:
         1. Forms (3 critical issues, 30 min)
         2. Images (5 major issues, 5 min each)
         3. Language (2 critical issues, 5 min each)
Claude: → "Start with forms — they block keyboard navigation testing"
```

## Related Skills

- `audit` — Source of findings to triage
- `remediate` — Fix code-based findings
- `guided-test` — Manual testing for complex findings
