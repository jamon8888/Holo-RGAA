# `/audit-project` — Audit a Local Project

## Description

Run an RGAA accessibility audit against a local project directory. Performs static analysis on source files (HTML, JSX, Vue, Angular templates) to find accessibility issues without requiring a running server. Falls back to live URL audit if project has a dev server.

## Prerequisites

- `rgaa-cli` installed (`cargo install --path rgaa-rs/crates/rgaa-cli`)
- Project in current directory or subdirectory

## Usage

```
/audit-project [path] [--framework react|vue|angular|next] [--scope glob]
```

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `path` | string | No | Project path (default: current directory) |
| `--framework` | string | No | Override framework detection |
| `--scope` | string | No | Limit to glob pattern (e.g., `src/components/*.tsx`) |

## Examples

```
/audit-project
/audit-project ./my-site
/audit-project --framework react
/audit-project --scope "src/**/*.jsx"
```

## What It Tests

| Test Type | Criteria | Description |
|-----------|----------|-------------|
| Static HTML | 1.1, 3.1, 3.3 | HTML structure, lang attribute, headings |
| JSX/TSX | 1.1, 4.1, 11.1 | React component accessibility |
| Vue templates | 1.1, 4.1, 11.1 | Vue SFC accessibility |
| Angular templates | 1.1, 4.1, 11.1 | Angular template accessibility |
| CSS/Focus | 12.1, 12.2 | Keyboard navigation, focus styles |

## Output

Same as `/audit-site` plus source locations:
```
RGAA Static Analysis Results
═══════════════════════════════════════════
Project:    ./my-site (React + TypeScript)
Files:      142 source files
Taux Global: 68.2%
──────────────────────────────────────────
Critical (5)
────────────
• RGAA 11.1 — FormField.tsx:42 — Missing label association
• RGAA 1.1 — Hero.tsx:15 — Image missing alt attribute
• RGAA 12.1 — Modal.tsx:78 — onClick without keyboard handler
```

## Framework Detection

Auto-detects from:
- `package.json` → `react`, `vue`, `angular`, `next`
- `*.jsx`, `*.tsx` → React
- `*.vue` → Vue
- `*.component.ts` → Angular

## CI/CD Usage

```bash
# Exit code 0 if passing, non-zero if failing
rgaa audit analyze --project ./my-site --threshold 85

# SARIF output for GitHub
rgaa audit analyze --project ./my-site --format sarif --output results.sarif
```

## Limitations

Static analysis cannot verify:
- Runtime behavior (JavaScript execution)
- Network-dependent content
- Server-rendered pages (use `/audit-site` instead)
- Browser-specific rendering

For full coverage, run both `/audit-project` and `/audit-site`.
