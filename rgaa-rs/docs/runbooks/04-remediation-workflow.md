# Remediation Workflow Runbook

This runbook explains how to use the remediation system to fix accessibility issues.

## Overview

The remediation system takes accessibility findings and generates approval-gated patch proposals with:

- Specific code changes
- Risk assessment
- Validation commands
- Implementation rationale

## Finding Issues

### Run an Audit

```bash
rgaa audit analyze --url https://example.test --output findings.json
```

### Extract Failed Criteria

```bash
# Get list of failed criteria
cat findings.json | jq '.pages[].criteria[] | select(.status == "fail")'

# Get detailed findings
cat findings.json | jq '.pages[].findings[] | select(.status == "fail")'
```

## Creating Remediation Issues

Format issues for the remediation system:

```json
[
  {
    "id": "img-001",
    "rule": "image-alt",
    "element_html": "<img src=\"hero.png\" class=\"hero\">",
    "page_url": "https://example.test",
    "source_locations": [
      { "file": "src/components/Hero.tsx", "line": 42 }
    ],
    "summary": "Image missing alt attribute",
    "remediation": "Add alt attribute with descriptive text",
    "criteria": ["1.1"],
    "framework": "react"
  }
]
```

**Required Fields:**

| Field | Description |
|-------|-------------|
| `id` | Unique identifier for this issue |
| `rule` | The accessibility rule violated |
| `element_html` | The HTML element that has the issue |
| `page_url` | URL where issue was found |
| `summary` | Brief description of the issue |
| `rememdiation` | How to fix it (from audit) |
| `criteria` | RGAA criteria this relates to |

**Optional Fields:**

| Field | Description |
|-------|-------------|
| `source_locations` | File/line where element is defined |
| `framework` | Framework: react, vue, angular, next |

## Generating Patches

### CLI

```bash
rgaa audit verify --issues remediation-issues.json --output patches.txt
```

### MCP

```javascript
const remediation = await mcp.callTool('remediate', {
  issues: [
    {
      id: 'img-001',
      rule: 'image-alt',
      element_html: '<img src="hero.png">',
      page_url: 'https://example.test',
      source_locations: [{ file: 'Hero.tsx', line: 42 }],
      summary: 'Image missing alt attribute',
      remediation: 'Add alt text',
      criteria: ['1.1'],
      framework: 'react'
    }
  ]
});
```

## Understanding the Response

```json
{
  "outcome": "ok",
  "issue_id": "img-001",
  "explanation": "Add alt attribute with descriptive text",
  "steps": [
    "Locate the img element in src/components/Hero.tsx:42",
    "Add alt=\"Hero image showing team collaboration\" to the img tag"
  ],
  "confidence": "high",
  "criteria": ["1.1"],
  "proposal": {
    "proposal_id": "prop-abc123",
    "finding_ids": ["finding-001"],
    "diff": "--- a/src/components/Hero.tsx\n+++ b/src/components/Hero.tsx\n@@ -40,2 +40,2 @@\n-<img src=\"hero.png\">\n+<img src=\"hero.png\" alt=\"Hero image showing team collaboration\">",
    "files": ["src/components/Hero.tsx"],
    "rationale": "Alt text provides textual alternative for screen readers",
    "risks": ["None - purely additive change"],
    "validation_commands": ["npm run a11y:test"],
    "expected_effect": "Passes RGAA 1.1",
    "proposal_hash": "sha256:abc123",
    "approval_state": {
      "kind": "required"
    },
    "approval_token": "rgaa-approval-v1-sha256:abc123:required"
  }
}
```

## Approval Workflow

### State: `required`

The patch needs human review before applying:

1. Review the `diff`
2. Check `rationale` and `risks`
3. Validate using `validation_commands`
4. If approved, apply patch manually or use approval token

### State: `not_required`

The patch can be applied automatically:

```bash
# Apply directly
git apply << 'EOF'
--- a/src/components/Hero.tsx
+++ b/src/components/Hero.tsx
@@ -40,2 +40,2 @@
-<img src="hero.png">
+<img src="hero.png" alt="Hero image showing team collaboration">
EOF
```

### State: `approved`

Already approved by someone:

```json
{
  "approval_state": {
    "kind": "approved",
    "approver": "john@example.com",
    "token": "rgaa-approval-v1-sha256:abc123:approved:john@example.com"
  }
}
```

## Framework-Specific Notes

### React

```json
{
  "framework": "react",
  "element_html": "<img src={heroImage} />"
}
```

Generated patches will use JSX syntax.

### Vue

```json
{
  "framework": "vue",
  "element_html": "<img :src=\"heroImage\">"
}
```

Generated patches will use Vue template syntax.

### Next.js

```json
{
  "framework": "next",
  "element_html": "<Image src={heroImage} />"
}
```

Generated patches will use Next.js Image component patterns.

### Angular

```json
{
  "framework": "angular",
  "element_html": "<img [src]=\"heroImage\">"
}
```

Generated patches will use Angular binding syntax.

## Validation

### Run Tests

```bash
# After applying patch, validate
npm run a11y:test

# Or specific test
npm test -- --testNamePattern="accessibility"
```

### Re-audit

```bash
# Re-run audit to confirm fix
rgaa audit analyze --url https://example.test --output post-fix.json

# Compare
diff <(jq '.findings' pre-fix.json) <(jq '.findings' post-fix.json)
```

## Batch Processing

Process multiple issues:

```bash
# Ensure batch size <= 25
rgaa audit verify --issues batch-issues.json
```

For larger sets, split into multiple batches:

```bash
# Split into batches of 25
split -l 25 issues.json batch_

# Process each
for batch in batch_*; do
  rgaa audit verify --issues "$batch" --output "${batch%.json}-patches.txt"
done
```

## Common Issues

### Element Not Found

If `source_locations` points to wrong file:

```json
{
  "issue_id": "img-001",
  "source_locations": [
    { "file": "ActualComponent.tsx", "line": 15 }
  ]
}
```

### Framework Detection Failure

If framework is auto-detected incorrectly:

```json
{
  "framework": "react",
  "element_html": "<img src=\"hero.png\">"
}
```

Explicitly specify framework.

### Multiple Fixes Needed

Split into separate issues:

```json
[
  { "id": "img-001", "rule": "image-alt", ... },
  { "id": "label-001", "rule": "form-label", ... }
]
```

## Best Practices

1. **Fix systematically** - Address issues by criterion
2. **Review each patch** - Don't auto-apply without review
3. **Run tests** - Validate before committing
4. **Re-audit** - Confirm issues are resolved
5. **Document exceptions** - Note any intentional non-compliance
