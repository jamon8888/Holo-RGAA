# rgaa-linter — Pure-Rust Accessibility Linter + GitHub Action

**Date:** 2026-08-24
**Status:** Draft
**Depends on:** 2026-08-24-rgaa-distribution-design.md (distribution spec)

---

## 1. Overview

A pure-Rust accessibility linter that checks HTML/CSS/JS source files against RGAA/wcag rules, inspired by Deque's axe-linter but running entirely locally. Two rule engines:

- **Static Engine**: Pure Rust, no browser — checks ~70% of axe rules (structural, semantic, ARIA attribute rules)
- **Runtime Engine**: Delegates to CDP browser via `rgaa-obscura` — checks remaining ~30% (color contrast, computed styles, ARIA name computation)

Plus a **GitHub Action** wrapper for CI/CD integration.

### Why Not Use Deque's axe-linter?

| Limitation | Impact |
|-----------|--------|
| Cloud-only API | No offline, no on-prem, no airgapped environments |
| Paid API key required | Cost per-consultant, no open-source option |
| No custom rules | Can't add RGAA-specific rules beyond axe-core |
| No local execution | Every file sent to Deque's servers (IP/data concern) |
| No CLI mode | Only works as GitHub Action, not standalone |
| No integration with existing rgaa-rs | Can't reuse your axe mapping, criteria catalog, gap-fix rules |

---

## 2. Architecture

```
rgaa-linter (new crate)
│
├── Static Engine (pure Rust, <100ms per file)
│   ├── html-linter (rule engine + DOM traversal)
│   ├── scraper (CSS selector querying via html5ever)
│   ├── axe-rule-translator (axe-core JSON → html-linter Rule format)
│   └── Static rules: ~70 accessibility rules
│
├── Runtime Engine (CDP browser required)
│   ├── rgaa-obscura (browser automation)
│   ├── axe-core injection (via CDP)
│   └── Runtime rules: color contrast, ARIA names, focus order
│
├── Unified Output
│   ├── LintResult { file, line, col, rule_id, severity, message, fix_hint }
│   ├── JSON output (machine-readable)
│   ├── Pretty terminal output (human-readable)
│   └── GitHub Actions annotations format
│
└── CLI / Integrations
    ├── rgaa-lint <path>              — static only (default)
    ├── rgaa-lint --browser <url>     — full (static + runtime)
    ├── rgaa-lint --format github     — GitHub Actions output
    └── GitHub Action (action.yml)
```

---

## 3. Static Rules Engine

### 3.1 Foundation: `html-linter` Crate

The `html-linter` crate (v0.1.1) provides:

- Rule-based architecture with JSON configuration
- 14+ rule types: `ElementPresence`, `AttributePresence`, `AttributeValue`, `ElementOrder`, `Nesting`, `Semantics`, `Compound`, `TextContent`, `DocumentStructure`, `ElementCount`, etc.
- CSS selector matching via `html5ever` (Servo's parser)
- Line/column location reporting
- Custom rule logic via `Custom` rule type

### 3.2 axe-core Rule Translation

The `axe-rules` crate already maps axe-core violations to RGAA criteria. We extend this with a translator that converts axe-core rule definitions into `html-linter` rules:

**axe-core rule format (simplified):**
```json
{
  "id": "image-alt",
  "selector": "img",
  "tags": ["cat.text-alternatives", "wcag2a", "wcag111"],
  "metadata": {
    "description": "Images must have alternate text",
    "help": "Alt text must be present for images",
    "helpUrl": "https://dequeuniversity.com/rules/axe/4.8/image-alt"
  }
}
```

**Translated to html-linter rule:**
```json
{
  "name": "image-alt",
  "rule_type": "AttributePresence",
  "severity": "Error",
  "selector": "img",
  "condition": "alt-missing",
  "message": "Images must have alternate text (WCAG 1.1.1)",
  "options": {}
}
```

### 3.3 Static Rule Categories

**Category A — Structural (easy, pure selector):**

| axe Rule ID | html-linter Type | What it checks |
|-------------|-----------------|----------------|
| `image-alt` | `AttributePresence` | `<img>` has `alt` |
| `html-has-lang` | `AttributePresence` | `<html>` has `lang` |
| `html-lang-valid` | `AttributeValue` | `lang` value is valid BCP 47 |
| `document-title` | `ElementPresence` | `<title>` exists in `<head>` |
| `meta-viewport` | `Compound` | No `user-scalable=no` |
| `bypass` | `ElementPresence` | Skip link or landmark present |
| `heading-order` | `ElementOrder` | No skipped heading levels |
| `duplicate-id` | `Custom` | No duplicate `id` attributes |
| `link-name` | `Compound` | `<a>` has text content or `aria-label` |
| `button-name` | `Compound` | `<button>` has text or `aria-label` |
| `label` | `Nesting` | `<input>` has associated `<label>` |
| `select-name` | `Compound` | `<select>` has accessible name |
| `tabindex` | `AttributeValue` | No positive `tabindex` values |
| `landmark-banner-is-top-level` | `Nesting` | `<header>` not inside article/section |
| `landmark-contentinfo-is-top-level` | `Nesting` | `<footer>` not inside article/section |
| `landmark-main-is-top-level` | `Nesting` | `<main>` is top-level |
| `landmark-no-duplicate-banner` | `ElementCount` | Single `<header>` (top-level) |
| `landmark-no-duplicate-contentinfo` | `ElementCount` | Single `<footer>` (top-level) |
| `landmark-one-main` | `ElementCount` | Exactly one `<main>` |
| `page-has-heading-one` | `ElementCount` | At least one `<h1>` |
| `video-caption` | `ElementPresence` | `<video>` has `<track kind="captions">` |
| `audio-caption` | `ElementPresence` | `<audio>` has text alternative |
| `td-has-header` | `Compound` | `<td>` has associated `<th>` |
| `th-has-data-cells` | `Compound` | `<th>` has associated `<td>` |
| `aria-allowed-attr` | `Compound` | ARIA attributes valid for role |
| `aria-required-attr` | `Compound` | Required ARIA attributes present |
| `aria-valid-attr-value` | `AttributeValue` | ARIA attribute values valid |
| `aria-valid-role` | `AttributeValue` | `role` value is valid |
| `avoid-inline-svg` | (custom) | Inline SVG has `role` and `aria-label` |

**Category B — Semantic (needs DOM traversal):**

| axe Rule ID | html-linter Type | What it checks |
|-------------|-----------------|----------------|
| `region` | `Compound` | All content in landmarks |
| `css-orientation-lock` | (custom) | No CSS forcing orientation |
| `scope-attr-valid` | `AttributeValue` | `scope` on `<th>` is valid |
| `td-headers-attr` | `Compound` | `headers` attribute references valid IDs |
| `aria-hidden-focus` | `Nesting` | `aria-hidden="true"` doesn't contain focusable elements |
| `form-field-multiple-labels` | `Custom` | No multiple `<label>` for single input |

**Category C — Runtime (requires browser, deferred to Runtime Engine):**

| axe Rule ID | Why it needs browser |
|-------------|---------------------|
| `color-contrast` | Needs computed background color + font rendering |
| `link-in-text-block` | Needs computed styles to detect link boundaries |
| `target-size` | Needs computed element dimensions |
| `meta-refresh` | Needs runtime behavior detection |
| `no-autoplay` | Needs runtime media state |
| `scrollable-region-focusable` | Needs computed `overflow` styles |
| `aria-required-children` | Needs ARIA role tree computation |
| `aria-required-parent` | Needs ARIA role tree computation |
| `image-alt` (partial) | SVG images need rendered content |

### 3.4 Configuration Format

**`.rgaa-lint.yml` (project root):**
```yaml
# Rule configuration
rules:
  # Override severity
  image-alt: error
  heading-order: warning
  
  # Disable specific rules
  css-orientation-lock: off
  
  # Custom rules
  custom:
    - name: "no-autoplay-video"
      rule_type: "ElementPresence"
      severity: "error"
      selector: "video[autoplay]"
      condition: "absent"
      message: "Videos must not autoplay without controls"

# Exclude patterns
exclude:
  - "test/**/*"
  - "vendor/**/*"
  - "*.min.html"
  - "node_modules/**"

# Include patterns (default: all HTML/CSS/JS files)
include:
  - "**/*.html"
  - "**/*.htm"
  - "**/*.jsx"
  - "**/*.tsx"
  - "**/*.vue"
  - "**/*.svelte"
  - "**/*.erb"
  - "**/*.php"

# Severity mapping
severity:
  error: 1    # Exit code 1 on errors
  warning: 0  # Warnings don't fail CI (configurable)
```

### 3.5 Rule Engine Flow

```
Input: file path(s) or directory
  │
  ├── Read file contents
  │
  ├── Detect file type (HTML, JSX, Vue template, etc.)
  │   └── Extract HTML from templates if needed
  │
  ├── Parse with html5ever (via scraper)
  │   └── Build DOM tree
  │
  ├── Load rules from .rgaa-lint.yml + default ruleset
  │
  ├── For each rule:
  │   ├── Match selector against DOM
  │   ├── Check condition (presence, value, order, nesting, etc.)
  │   └── If violated → emit LintResult with location
  │
  ├── Deduplicate (same element, same rule)
  │
  └── Output results
```

---

## 4. Runtime Rules Engine

### 4.1 Architecture

For rules that need a browser, the runtime engine:

1. Spins up a headless Chrome/Chromium instance (via `rgaa-obscura`)
2. Renders the HTML file
3. Injects axe-core
4. Runs axe-core evaluation
5. Collects violations
6. Maps back to source file locations

### 4.2 Flow

```
Input: file path(s) + "runtime" mode
  │
  ├── Start browser (if not already running)
  │
  ├── For each file:
  │   ├── Create data: URI or serve via localhost
  │   ├── Navigate browser to it
  │   ├── Inject axe-core
  │   ├── axe.run() → collect violations
  │   └── Map axe violations to source locations
  │
  ├── Merge with static results (deduplicate)
  │
  └── Output combined results
```

### 4.3 Source Mapping for Runtime Results

axe-core returns violations with DOM selectors. To map back to source lines:

1. Parse the original source with `scraper`
2. For each axe violation's target element:
   - Match by element signature (tag + attributes + text content)
   - Find matching node in source DOM
   - Report line/column from source parse tree

### 4.4 When to Use Runtime Mode

- `rgaa-lint <file>` → static only (default, instant)
- `rgaa-lint --browser <file>` → static + runtime (needs Chrome)
- `rgaa-lint --browser --url <live-url>` → runtime against live site
- GitHub Action: static by default, runtime optional via input flag

---

## 5. GitHub Action

### 5.1 action.yml

```yaml
name: 'RGAA Accessibility Linter'
description: 'Lint HTML/CSS/JS files for RGAA accessibility issues'
branding:
  icon: 'accessibility'
  color: 'blue'

inputs:
  path:
    description: 'Files or directories to lint'
    required: false
    default: '.'
  
  mode:
    description: 'Linting mode: static (fast, no browser) or full (includes runtime checks)'
    required: false
    default: 'static'
    type: choice
    options:
      - static
      - full
  
  format:
    description: 'Output format: pretty, json, github'
    required: false
    default: 'github'
  
  config:
    description: 'Path to .rgaa-lint.yml config file'
    required: false
    default: '.rgaa-lint.yml'
  
  fail-on:
    description: 'Fail on: error, warning, or never'
    required: false
    default: 'error'
    type: choice
    options:
      - error
      - warning
      - never
  
  only-changed:
    description: 'Only lint files changed in the PR'
    required: false
    default: 'true'

runs:
  using: 'composite'
  steps:
    - name: Install rgaa-lint
      shell: bash
      run: |
        # Download pre-built binary for the runner's OS
        curl -fsSL https://rgaa.dev/install-lint.sh | sh
    
    - name: Run RGAA Linter
      shell: bash
      run: |
        ARGS=""
        if [ "${{ inputs.only-changed }}" = "true" ]; then
          ARGS="$ARGS --changed"
        fi
        if [ "${{ inputs.mode }}" = "full" ]; then
          ARGS="$ARGS --browser"
        fi
        rgaa-lint ${{ inputs.path }} \
          --format ${{ inputs.format }} \
          --config ${{ inputs.config }} \
          --fail-on ${{ inputs.fail-on }} \
          $ARGS
```

### 5.2 Usage Example

```yaml
# .github/workflows/rgaa-lint.yml
name: RGAA Accessibility Lint
on:
  pull_request:
    paths:
      - '**/*.html'
      - '**/*.jsx'
      - '**/*.tsx'
      - '**/*.vue'
      - '**/*.svelte'

jobs:
  lint:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      pull-requests: write  # For annotations
    steps:
      - uses: actions/checkout@v4
      
      - name: RGAA Lint (static)
        uses: your-org/rgaa-lint-action@v1
        with:
          path: '.'
          mode: static
          fail-on: error
          only-changed: true
```

### 5.3 Output Format

**GitHub Actions annotations:**
```
::error file=src/index.html,line=42,col=10,endColumn=25::image-alt - Images must have alternate text (WCAG 1.1.1) https://rgaa.dev/rules/image-alt
::warning file=src/index.html,line=67,col=1,endColumn=50::heading-order - Heading levels should not skip (WCAG 1.3.1)
```

**Step summary:**
```markdown
## RGAA Accessibility Lint Results

### Errors (3)
| File | Line | Rule | Message |
|------|------|------|---------|
| src/index.html | 42 | image-alt | `<img>` missing alt attribute |
| src/index.html | 89 | label | `<input>` without associated label |
| src/components/Form.tsx | 23 | button-name | `<button>` has no accessible name |

### Warnings (5)
...

✅ Passed: 98/106 rules checked
❌ Failed: 3 errors, 5 warnings
```

---

## 6. CLI Interface

```
rgaa-lint [paths...] [options]

Options:
  --format <fmt>       Output format: pretty, json, github, sarif [default: pretty]
  --config <path>      Config file path [default: .rgaa-lint.yml]
  --fail-on <level>    Exit code 1 on: error, warning, never [default: error]
  --browser            Enable runtime rules (requires Chrome)
  --url <url>          Lint a live URL instead of files
  --changed            Only lint files changed since last commit (git)
  --fix                Auto-fix where possible (future)
  --exclude <pattern>  Exclude files matching pattern
  --include <pattern>  Include files matching pattern
  --rules <list>       Only run specific rules (comma-separated)
  --skip <list>        Skip specific rules (comma-separated)
  --output <file>      Write results to file
  --jobs <n>           Parallel file processing [default: num CPUs]
  -v, --verbose        Verbose output
  -h, --help           Print help
  -V, --version        Print version

Exit codes:
  0    No errors (warnings may be present)
  1    Errors found
  2    Runtime error (file not found, browser error, etc.)
```

---

## 7. Integration with Existing Crates

### 7.1 `rgaa-rules` Extension

Add to existing `AxeMapper`:

```rust
impl AxeMapper {
    /// Convert axe-core rules to html-linter rules for static checking
    pub fn to_html_linter_rules(axe_rules_json: &str) -> Result<Vec<HtmlLinterRule>> {
        // For each axe rule:
        // - Map selector
        // - Determine rule type (presence, attribute, nesting, etc.)
        // - Generate html-linter Rule
    }
    
    /// Get static-only rules (can be checked without browser)
    pub fn static_rules() -> Vec<&'static str> {
        // Returns list of rule IDs that are static-only
    }
    
    /// Get runtime-only rules (require browser)
    pub fn runtime_rules() -> Vec<&'static str> {
        // Returns list of rule IDs that need CDP
    }
}
```

### 7.2 `rgaa-core` Extension

```rust
/// Mapping from axe-core rule ID to RGAA criterion
pub struct RuleMapping {
    pub axe_rule_id: String,
    pub rgaa_criterion: CriterionId,
    pub is_static: bool,
    pub html_linter_rule_type: Option<RuleType>,
}
```

### 7.3 New Dependencies

```toml
[dependencies]
html-linter = "0.1"
scraper = "0.23"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
clap = { version = "4", features = ["derive"] }
glob = "0.3"
anyhow = "1"
thiserror = "2"
tracing = "0.1"

# Runtime engine (optional, feature-gated)
rgaa-obscura = { path = "../rgaa-obscura", optional = true }
reqwest = { version = "0.12", optional = true }

[features]
default = ["static"]
runtime = ["rgaa-obscura", "reqwest"]
```

---

## 8. Implementation Plan

### Phase 1: Static Engine (Weeks 1-2)
- [ ] Create `rgaa-linter` crate
- [ ] Implement axe-core → html-linter rule translator
- [ ] Port 30 most common static rules
- [ ] CLI with file input + pretty output
- [ ] Unit tests for each rule

### Phase 2: Config & Exclusions (Week 3)
- [ ] `.rgaa-lint.yml` config loading
- [ ] File glob patterns (include/exclude)
- [ ] Template extraction (JSX, Vue, Svelte)
- [ ] Severity overrides

### Phase 3: GitHub Action (Week 4)
- [ ] `action.yml` composite action
- [ ] GitHub annotations output format
- [ ] Step summary markdown builder
- [ ] PR-only changed file detection
- [ ] Test on real repos

### Phase 4: Runtime Engine (Weeks 5-6)
- [ ] Feature-gated `runtime` module
- [ ] CDP browser spin-up via rgaa-obscura
- [ ] axe-core injection + evaluation
- [ ] Source mapping for runtime violations
- [ ] Merge static + runtime results

### Phase 5: Polish (Week 7)
- [ ] SARIF output format (for GitHub code scanning)
- [ ] Remaining static rules (reach ~70)
- [ ] Performance optimization (parallel file processing)
- [ ] Documentation + examples

---

## 9. Testing Strategy

### Unit Tests
- Each rule tested with passing + failing HTML snippets
- Rule translator tested against known axe-core rule definitions
- Config parsing tested with various `.rgaa-lint.yml` formats

### Integration Tests
- Full CLI run against fixture HTML files
- GitHub Actions annotation format validation
- Template extraction (JSX, Vue, Svelte) tested separately

### Reference Tests
- Run against axe-core's own test cases where applicable
- Compare static engine results against full axe-core run
- Measure coverage: what % of axe violations are caught statically

---

## 10. Open Questions

1. **Rule coverage target**: Start with 30 rules and grow, or aim for all 70+ static rules from day one?
2. **Template support scope**: JSX/Vue/Svelte from day one, or HTML-only MVP?
3. **Auto-fix**: Should Phase 1 include `--fix` for simple cases (adding `alt=""`, adding `lang="fr"`)?
4. **SARIF output**: Needed for GitHub code scanning integration, or nice-to-have?
5. **Branding**: Public name — `rgaa-lint`, `axe-lint-rs`, something else?
