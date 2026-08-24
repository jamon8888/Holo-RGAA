# rgaa-linter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a pure-Rust accessibility linter (`rgaa-lint`) with 70+ static rules, auto-fix, CLI, and GitHub Action — replacing Deque's cloud-based axe-linter with a local-first, offline-capable alternative.

**Architecture:** Two-layer design: (1) Static engine using `html-linter` + `scraper` for pure-Rust rule checking, (2) Runtime engine delegating to CDP browser via `rgaa-obscura` for rules needing computed styles. CLI exposes both modes. GitHub Action wraps the CLI for CI/CD.

**Tech Stack:** Rust 2024 edition, `html-linter` 0.1, `scraper` 0.23, `clap` 4, `serde`/`serde_json`/`serde_yaml`, `glob`, `anyhow`/`thiserror`. Optional: `rgaa-obscura` (feature-gated `runtime`).

**Spec:** `docs/superpowers/specs/2026-08-24-rgaa-linter-design.md`

## Global Constraints

- Rust edition 2024, rust-version 1.85
- Follow existing workspace conventions: `serde`/`serde_json` from workspace, `thiserror` 2, `clap` 4 with derive
- Unit-struct pattern for stateless services (like existing `AxeMapper`)
- `RgaaError` enum with `type Result<T>` alias for library errors
- Serde derives on all data types
- French domain terminology for RGAA concepts, English for code identifiers
- No comments unless asked
- `#[deny(clippy::correctness)]` at crate level

---

## File Structure

```
rgaa-rs/crates/rgaa-linter/
├── Cargo.toml
├── src/
│   ├── lib.rs                    # Public API re-exports
│   ├── cli.rs                    # Clap CLI definition
│   ├── config.rs                 # .rgaa-lint.yml loading
│   ├── engine/
│   │   ├── mod.rs                # Engine trait + LintEngine
│   │   ├── static_engine.rs      # html-linter based static checking
│   │   └── runtime_engine.rs     # CDP browser runtime checking (feature-gated)
│   ├── rules/
│   │   ├── mod.rs                # Rule registry
│   │   ├── axe_translator.rs     # axe-core JSON → html-linter Rule
│   │   ├── static_rules.rs       # All 70+ static rule definitions
│   │   └── fixer.rs              # Auto-fix transforms
│   ├── output/
│   │   ├── mod.rs                # Output trait + formatter dispatch
│   │   ├── pretty.rs             # Terminal colored output
│   │   ├── json.rs               # Machine-readable JSON
│   │   └── github.rs             # GitHub Actions annotations + summary
│   └── types.rs                  # LintResult, LintViolation, Fix etc.
├── tests/
│   ├── static_rules_test.rs      # One test per rule (pass + fail)
│   ├── config_test.rs            # Config loading tests
│   ├── fixer_test.rs             # Auto-fix tests
│   └── cli_test.rs               # CLI integration tests
└── fixtures/
    ├── pass/
    │   ├── valid.html            # Clean HTML that passes all rules
    │   └── ...
    └── fail/
        ├── missing-alt.html      # Fails image-alt
        ├── missing-lang.html     # Fails html-has-lang
        └── ...                   # One fixture per rule
```

---

## Task 1: Scaffold Crate + Types

**Files:**
- Create: `rgaa-rs/crates/rgaa-linter/Cargo.toml`
- Create: `rgaa-rs/crates/rgaa-linter/src/lib.rs`
- Create: `rgaa-rs/crates/rgaa-linter/src/types.rs`
- Modify: `rgaa-rs/Cargo.toml` (add to workspace members)

**Interfaces:**
- Produces: `LintResult`, `LintViolation`, `Severity`, `Fix`, `FixType`, `LintSummary`

- [ ] **Step 1: Add crate to workspace**

Edit `rgaa-rs/Cargo.toml`, add `"crates/rgaa-linter"` to `members` array.

- [ ] **Step 2: Create Cargo.toml**

```toml
[package]
name = "rgaa-linter"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"

[lints.rust]
unsafe_code = "warn"

[lints.clippy]
all = "warn"
pedantic = "warn"

[dependencies]
html-linter = "0.1"
scraper = "0.23"
serde = { workspace = true }
serde_json = { workspace = true }
serde_yaml = "0.9"
clap = { workspace = true }
glob = "0.3"
anyhow = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
colored = "3"

[dev-dependencies]
tempfile = "3"

[features]
default = ["static"]
runtime = ["dep:reqwest"]
```

- [ ] **Step 3: Create types.rs**

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FixType {
    InsertAttribute { attr: String, value: String },
    RemoveAttribute { attr: String },
    SetAttribute { attr: String, value: String },
    InsertElement { tag: String, content: String },
    ReplaceText { replacement: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fix {
    pub file: PathBuf,
    pub line: usize,
    pub col: usize,
    pub end_line: Option<usize>,
    pub end_col: Option<usize>,
    pub fix_type: FixType,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintViolation {
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
    pub file: PathBuf,
    pub line: usize,
    pub col: usize,
    pub end_line: Option<usize>,
    pub end_col: Option<usize>,
    pub help_url: Option<String>,
    pub fix: Option<Fix>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintResult {
    pub violations: Vec<LintViolation>,
    pub files_checked: usize,
    pub rules_run: usize,
    pub duration_ms: u64,
}

impl LintResult {
    pub fn error_count(&self) -> usize {
        self.violations.iter().filter(|v| v.severity == Severity::Error).count()
    }

    pub fn warning_count(&self) -> usize {
        self.violations.iter().filter(|v| v.severity == Severity::Warning).count()
    }

    pub fn has_errors(&self) -> bool {
        self.error_count() > 0
    }
}
```

- [ ] **Step 4: Create lib.rs**

```rust
pub mod types;
pub mod engine;
pub mod rules;
pub mod output;
pub mod config;
pub mod cli;

pub use types::*;
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo check -p rgaa-linter`
Expected: Compiles (with stub modules)

- [ ] **Step 6: Commit**

```bash
git add rgaa-rs/crates/rgaa-linter/ rgaa-rs/Cargo.toml
git commit -m "feat(rgaa-linter): scaffold crate with core types"
```

---

## Task 2: axe-core Rule Translator

**Files:**
- Create: `rgaa-rs/crates/rgaa-linter/src/rules/mod.rs`
- Create: `rgaa-rs/crates/rgaa-linter/src/rules/axe_translator.rs`
- Create: `rgaa-rs/crates/rgaa-linter/src/rules/static_rules.rs`
- Test: `rgaa-rs/crates/rgaa-linter/tests/static_rules_test.rs`

**Interfaces:**
- Produces: `StaticRule`, `RuleRegistry::all()` → `Vec<StaticRule>`

- [ ] **Step 1: Create rules/mod.rs**

```rust
pub mod axe_translator;
pub mod static_rules;
pub mod fixer;

pub use static_rules::{StaticRule, RuleRegistry};
pub use fixer::Fixer;
```

- [ ] **Step 2: Create axe_translator.rs**

This module defines how axe-core rule definitions map to `html-linter` rule types.

```rust
use html_linter::{Rule, RuleType, Severity as HtmlSeverity};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AxeRuleDefinition {
    pub id: &'static str,
    pub selector: &'static str,
    pub rule_type: RuleType,
    pub condition: &'static str,
    pub message: &'static str,
    pub severity: HtmlSeverity,
    pub options: HashMap<String, String>,
    pub is_static: bool,
}

impl AxeRuleDefinition {
    pub fn to_html_linter_rule(&self) -> Rule {
        Rule {
            name: self.id.to_string(),
            rule_type: self.rule_type.clone(),
            severity: self.severity.clone(),
            selector: self.selector.to_string(),
            condition: self.condition.to_string(),
            message: self.message.to_string(),
            options: self.options.clone(),
        }
    }
}
```

- [ ] **Step 3: Create static_rules.rs with all 70+ rules**

This is the core file — all axe-core rules that can be checked statically.

```rust
use super::axe_translator::AxeRuleDefinition;
use html_linter::RuleType;
use std::collections::HashMap;

pub struct StaticRule {
    pub id: &'static str,
    pub axe_def: AxeRuleDefinition,
    pub rgaa_criterion: &'static str,
    pub fixable: bool,
}

pub struct RuleRegistry;

impl RuleRegistry {
    pub fn all() -> Vec<StaticRule> {
        let mut rules = Vec::new();
        // Category A: Structural rules
        rules.extend(Self::structural_rules());
        // Category B: Semantic rules
        rules.extend(Self::semantic_rules());
        rules
    }

    fn structural_rules() -> Vec<StaticRule> { /* 60+ rules below */ }
    fn semantic_rules() -> Vec<StaticRule> { /* 10+ rules below */ }
}
```

Then populate with the full 70+ rules. Each rule follows this pattern:

```rust
StaticRule {
    id: "image-alt",
    axe_def: AxeRuleDefinition {
        id: "image-alt",
        selector: "img",
        rule_type: RuleType::AttributePresence,
        condition: "alt-missing",
        message: "Images must have alternate text (WCAG 1.1.1 / RGAA 1.1)",
        severity: HtmlSeverity::Error,
        options: HashMap::new(),
        is_static: true,
    },
    rgaa_criterion: "1.1",
    fixable: true,
},
```

**Full rule list to implement (70+ rules):**

*Structural (Category A) — 60 rules:*
1. `image-alt` — img has alt → RGAA 1.1
2. `input-image-alt` — input[type=image] has alt → RGAA 1.2
3. `image-redundant-alt` — alt not duplicate adjacent text → RGAA 1.5
4. `area-alt` — area has alt → RGAA 1.1
5. `html-has-lang` — html has lang → RGAA 8.2
6. `html-lang-valid` — lang is valid BCP 47 → RGAA 8.3
7. `document-title` — title in head → RGAA 8.5
8. `meta-viewport` — no user-scalable=no → RGAA 10.4
9. `bypass` — skip link or landmark → RGAA 12.7
10. `heading-order` — sequential headings → RGAA 9.3
11. `duplicate-id` — no duplicate ids → RGAA 8.1
12. `duplicate-id-active` — no duplicate active-element ids → RGAA 8.1
13. `duplicate-id-aria` — no duplicate aria id refs → RGAA 8.1
14. `link-name` — a has accessible name → RGAA 6.1
15. `button-name` — button has accessible name → RGAA 11.1
16. `label` — input has label → RGAA 11.1
17. `select-name` — select has accessible name → RGAA 11.1
18. `textarea-name` — textarea has accessible name → RGAA 11.1
19. `tabindex` — no positive tabindex → RGAA 10.4
20. `landmark-banner-is-top-level` — header not in article/section → RGAA 12.1
21. `landmark-contentinfo-is-top-level` — footer not in article/section → RGAA 12.1
22. `landmark-main-is-top-level` — main is top-level → RGAA 12.1
23. `landmark-no-duplicate-banner` — single top-level header → RGAA 12.1
24. `landmark-no-duplicate-contentinfo` — single top-level footer → RGAA 12.1
25. `landmark-one-main` — exactly one main → RGAA 12.1
26. `page-has-heading-one` — at least one h1 → RGAA 9.1
27. `video-caption` — video has track[captions] → RGAA 4.1
28. `audio-caption` — audio has text alternative → RGAA 4.1
29. `video-description` — video has track[descriptions] → RGAA 4.3
30. `td-has-header` — td has associated th → RGAA 5.6
31. `th-has-data-cells` — th has associated td → RGAA 5.7
32. `scope-attr-valid` — th scope is valid → RGAA 5.6
33. `td-headers-attr` — headers attr references valid ids → RGAA 5.7
34. `aria-allowed-attr` — aria attrs valid for role → RGAA 8.1
35. `aria-required-attr` — required aria attrs present → RGAA 8.1
36. `aria-valid-attr-value` — aria attr values valid → RGAA 8.1
37. `aria-valid-role` — role value is valid → RGAA 8.1
38. `aria-hidden-focus` — aria-hidden doesn't contain focusable → RGAA 10.7
39. `region` — all content in landmarks → RGAA 12.1
40. `css-orientation-lock` — no css orientation lock → RGAA 13.4
41. `form-field-multiple-labels` — no multiple labels → RGAA 11.1
42. `frame-title` — iframe has title → RGAA 2.1
43. `frame-title-unique` — iframe titles are unique → RGAA 2.1
44. `image-map-area-alt` — area in image-map has alt → RGAA 1.1
45. `label-title-only` — label not title-only → RGAA 11.1
46. `list` — ul/ol has li children → RGAA 9.3
47. `listitem` — li has parent ul/ol → RGAA 9.3
48. `definition-list` — dl has dt/dd children → RGAA 9.3
49. `definition-list-item` — dt/dd has parent dl → RGAA 9.3
50. `dlitem` — dt/dd has parent dl → RGAA 9.3
51. `blockquote-cite` — blockquote has cite → RGAA 9.3
52. `marquee` — no marquee element → RGAA 13.4
53. `blink` — no blink element → RGAA 13.4
54. `no-autoplay` — media has no autoplay without controls → RGAA 4.1
55. `meta-refresh` — no meta refresh → RGAA 13.4
56. `meta-refresh-no-unsuspected` — meta refresh not too fast → RGAA 13.4
57. `valid-lang` — lang attr on elements with text → RGAA 8.3
58. `href-no-hash` — href not just # → RGAA 6.1
59. `link-in-text-block` — links distinguishable from text → RGAA 6.1 (static heuristic)
60. `target-size` — target minimum size → RGAA 13.3 (static heuristic)

*Semantic (Category B) — 12 rules:*
61. `aria-allowed-role` — role is allowed for element → RGAA 8.1
62. `aria-dialog-name` — dialog/modal has accessible name → RGAA 8.1
63. `aria-hidden-body` — body not aria-hidden → RGAA 8.1
64. `aria-required-children` — elements with role have required children → RGAA 8.1
65. `aria-required-parent` — elements with role have required parent → RGAA 8.1
66. `html5elem` — deprecated elements not used → RGAA 8.9
67. `deprecated-role` — deprecated ARIA roles not used → RGAA 8.1
68. `preferred-role` — non-preferred roles not used → RGAA 8.1
69. `unsupportedrole` — supported ARIA roles used → RGAA 8.1
70. `aria-roledescription` — aria-roledescription on elements with role → RGAA 8.1
71. `scrollable-region-focusable` — scrollable regions are focusable → RGAA 10.4
72. `autocomplete-valid` — autocomplete attr valid → RGAA 11.13

- [ ] **Step 4: Write test for rule count**

```rust
// tests/static_rules_test.rs
use rgaa_linter::rules::RuleRegistry;

#[test]
fn has_at_least_70_rules() {
    let rules = RuleRegistry::all();
    assert!(rules.len() >= 70, "Expected at least 70 rules, got {}", rules.len());
}

#[test]
fn all_rules_have_unique_ids() {
    let rules = RuleRegistry::all();
    let ids: Vec<&str> = rules.iter().map(|r| r.id).collect();
    let mut unique = ids.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(ids.len(), unique.len(), "Duplicate rule IDs found");
}

#[test]
fn all_rules_map_to_rgaa_criteria() {
    let rules = RuleRegistry::all();
    for rule in &rules {
        assert!(!rule.rgaa_criterion.is_empty(), "Rule {} has no RGAA criterion", rule.id);
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p rgaa-linter`
Expected: All 3 tests pass

- [ ] **Step 6: Commit**

```bash
git add rgaa-rs/crates/rgaa-linter/
git commit -m "feat(rgaa-linter): add 70+ static rule definitions with axe-core translator"
```

---

## Task 3: Static Linting Engine

**Files:**
- Create: `rgaa-rs/crates/rgaa-linter/src/engine/mod.rs`
- Create: `rgaa-rs/crates/rgaa-linter/src/engine/static_engine.rs`

**Interfaces:**
- Consumes: `StaticRule`, `RuleRegistry`
- Produces: `StaticEngine::lint(html, filename)` → `Vec<LintViolation>`

- [ ] **Step 1: Create engine/mod.rs**

```rust
pub mod static_engine;
#[cfg(feature = "runtime")]
pub mod runtime_engine;

pub use static_engine::StaticEngine;
```

- [ ] **Step 2: Create static_engine.rs**

```rust
use crate::rules::{StaticRule, RuleRegistry};
use crate::types::{LintViolation, Severity};
use std::path::Path;

pub struct StaticEngine {
    rules: Vec<StaticRule>,
}

impl StaticEngine {
    pub fn new() -> Self {
        Self { rules: RuleRegistry::all() }
    }

    pub fn with_rules(rules: Vec<StaticRule>) -> Self {
        Self { rules }
    }

    pub fn lint(&self, html: &str, file_path: &Path) -> Vec<LintViolation> {
        let mut violations = Vec::new();

        // Parse HTML with html5ever (via scraper)
        let document = scraper::Html::parse_document(html);

        // Convert our rules to html-linter rules
        let html_rules: Vec<html_linter::Rule> = self.rules.iter()
            .map(|r| r.axe_def.to_html_linter_rule())
            .collect();

        // Create linter and run
        let linter = html_linter::HtmlLinter::new(html_rules, None);

        match linter.lint(html) {
            Ok(results) => {
                for result in results {
                    // Find matching StaticRule for RGAA criterion info
                    let static_rule = self.rules.iter()
                        .find(|r| r.id == result.rule);

                    violations.push(LintViolation {
                        rule_id: result.rule,
                        severity: match result.severity {
                            html_linter::Severity::Error => Severity::Error,
                            html_linter::Severity::Warning => Severity::Warning,
                            html_linter::Severity::Info => Severity::Info,
                        },
                        message: result.message,
                        file: file_path.to_path_buf(),
                        line: result.location.line,
                        col: result.location.column,
                        end_line: None,
                        end_col: None,
                        help_url: static_rule.map(|r| {
                            format!("https://rgaa.dev/rules/{}", r.id)
                        }),
                        fix: None, // Populated by Fixer separately
                    });
                }
            }
            Err(e) => {
                tracing::warn!("Linter error: {}", e);
            }
        }

        violations
    }
}
```

- [ ] **Step 3: Write integration test with valid HTML**

```rust
// In tests/static_rules_test.rs
use rgaa_linter::engine::StaticEngine;
use std::path::Path;

#[test]
fn valid_html_passes_all_rules() {
    let html = r#"<!DOCTYPE html>
<html lang="fr">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Page titre</title>
</head>
<body>
    <a href="#main">Aller au contenu principal</a>
    <header>
        <nav aria-label="Navigation principale">
            <ul>
                <li><a href="/">Accueil</a></li>
            </ul>
        </nav>
    </header>
    <main id="main">
        <h1>Titre principal</h1>
        <h2>Sous-titre</h2>
        <img src="photo.jpg" alt="Description de la photo">
        <form>
            <label for="nom">Nom</label>
            <input type="text" id="nom" name="nom">
        </form>
    </main>
    <footer>
        <p>&copy; 2026</p>
    </footer>
</body>
</html>"#;

    let engine = StaticEngine::new();
    let violations = engine.lint(html, Path::new("test.html"));
    assert!(violations.is_empty(), "Valid HTML should pass: {:?}", violations);
}
```

- [ ] **Step 4: Write test with missing alt**

```rust
#[test]
fn missing_alt_triggers_image_alt_rule() {
    let html = r#"<!DOCTYPE html>
<html lang="fr"><head><title>T</title></head>
<body><img src="x.jpg"></body>
</html>"#;

    let engine = StaticEngine::new();
    let violations = engine.lint(html, Path::new("test.html"));
    assert!(violations.iter().any(|v| v.rule_id == "image-alt"));
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p rgaa-linter`
Expected: Both tests pass

- [ ] **Step 6: Commit**

```bash
git add rgaa-rs/crates/rgaa-linter/src/engine/
git commit -m "feat(rgaa-linter): implement static linting engine"
```

---

## Task 4: Auto-Fix Module

**Files:**
- Create: `rgaa-rs/crates/rgaa-linter/src/rules/fixer.rs`
- Test: `rgaa-rs/crates/rgaa-linter/tests/fixer_test.rs`

**Interfaces:**
- Consumes: `Vec<LintViolation>`, HTML source
- Produces: `Fixer::apply_fixes(html, violations)` → `(String, Vec<Fix>)`

- [ ] **Step 1: Create fixer.rs**

```rust
use crate::types::{Fix, FixType, LintViolation, Severity};
use std::path::Path;

pub struct Fixer;

impl Fixer {
    pub fn apply_fixes(
        html: &str,
        violations: &[LintViolation],
        file_path: &Path,
    ) -> (String, Vec<Fix>) {
        let mut fixes = Vec::new();
        let mut result = html.to_string();

        // Process fixes in reverse order (to preserve line numbers)
        let mut fixable: Vec<&LintViolation> = violations.iter()
            .filter(|v| v.fix.is_some() || Self::can_auto_fix(&v.rule_id))
            .collect();
        fixable.sort_by(|a, b| b.line.cmp(&a.line));

        for violation in fixable {
            if let Some(fix) = Self::generate_fix(&violation, &result, file_path) {
                result = Self::apply_single_fix(&result, &fix);
                fixes.push(fix);
            }
        }

        (result, fixes)
    }

    pub fn can_auto_fix(rule_id: &str) -> bool {
        matches!(rule_id,
            "image-alt" | "html-has-lang" | "html-lang-valid" |
            "document-title" | "tabindex" | "video-caption" |
            "audio-caption" | "frame-title" | "aria-valid-role"
        )
    }

    fn generate_fix(
        violation: &LintViolation,
        html: &str,
        file_path: &Path,
    ) -> Option<Fix> {
        match violation.rule_id.as_str() {
            "image-alt" => Some(Fix {
                file: file_path.to_path_buf(),
                line: violation.line,
                col: violation.col,
                end_line: None,
                end_col: None,
                fix_type: FixType::InsertAttribute {
                    attr: "alt".to_string(),
                    value: "".to_string(),
                },
                description: "Add empty alt attribute for decorative image".to_string(),
            }),
            "html-has-lang" => Some(Fix {
                file: file_path.to_path_buf(),
                line: violation.line,
                col: violation.col,
                end_line: None,
                end_col: None,
                fix_type: FixType::InsertAttribute {
                    attr: "lang".to_string(),
                    value: "fr".to_string(),
                },
                description: "Add lang=\"fr\" to html element".to_string(),
            }),
            _ => None,
        }
    }

    fn apply_single_fix(html: &str, fix: &Fix) -> String {
        // Line-based text replacement
        let lines: Vec<&str> = html.lines().collect();
        let mut result = lines.clone();

        if fix.line > 0 && fix.line <= lines.len() {
            let line = lines[fix.line - 1];
            match &fix.fix_type {
                FixType::InsertAttribute { attr, value } => {
                    // Find the closing > of the opening tag and insert before it
                    if let Some(pos) = line.rfind('>') {
                        let new_line = format!(
                            "{} {}=\"{}\"{}",
                            &line[..pos],
                            attr,
                            value,
                            &line[pos..]
                        );
                        result[fix.line - 1] = Box::leak(new_line.into_boxed_str());
                    }
                }
                _ => {}
            }
        }

        result.join("\n")
    }
}
```

- [ ] **Step 2: Write fixer tests**

```rust
// tests/fixer_test.rs
use rgaa_linter::rules::Fixer;
use rgaa_linter::types::{Fix, FixType, LintViolation, Severity};
use std::path::Path;

#[test]
fn can_auto_fix_image_alt() {
    assert!(Fixer::can_auto_fix("image-alt"));
}

#[test]
fn cannot_auto_fix_color_contrast() {
    assert!(!Fixer::can_auto_fix("color-contrast"));
}

#[test]
fn fix_inserts_alt_attribute() {
    let html = "<img src=\"x.jpg\">";
    let violations = vec![LintViolation {
        rule_id: "image-alt".to_string(),
        severity: Severity::Error,
        message: "missing alt".to_string(),
        file: Path::new("test.html").to_path_buf(),
        line: 1,
        col: 1,
        end_line: None,
        end_col: None,
        help_url: None,
        fix: None,
    }];

    let (fixed, fixes) = Fixer::apply_fixes(html, &violations, Path::new("test.html"));
    assert!(fixed.contains("alt=\"\""));
    assert_eq!(fixes.len(), 1);
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p rgaa-linter`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add rgaa-rs/crates/rgaa-linter/src/rules/fixer.rs rgaa-rs/crates/rgaa-linter/tests/fixer_test.rs
git commit -m "feat(rgaa-linter): implement auto-fix for simple rules"
```

---

## Task 5: Config Loading

**Files:**
- Create: `rgaa-rs/crates/rgaa-linter/src/config.rs`
- Test: `rgaa-rs/crates/rgaa-linter/tests/config_test.rs`

**Interfaces:**
- Produces: `Config::load(path)` → `Config`

- [ ] **Step 1: Create config.rs**

```rust
use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub rules: HashMap<String, String>,

    #[serde(default)]
    pub exclude: Vec<String>,

    #[serde(default)]
    pub include: Vec<String>,

    #[serde(default)]
    pub severity: SeverityConfig,
}

#[derive(Debug, Deserialize, Default)]
pub struct SeverityConfig {
    #[serde(default = "default_error_code")]
    pub error: i32,
    #[serde(default)]
    pub warning: i32,
}

fn default_error_code() -> i32 { 1 }

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            Ok(serde_yaml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn is_excluded(&self, path: &str) -> bool {
        self.exclude.iter().any(|pattern| {
            glob::Pattern::new(pattern)
                .map(|p| p.matches(path))
                .unwrap_or(false)
        })
    }

    pub fn should_include(&self, path: &str) -> bool {
        if self.include.is_empty() {
            return true; // Include everything by default
        }
        self.include.iter().any(|pattern| {
            glob::Pattern::new(pattern)
                .map(|p| p.matches(path))
                .unwrap_or(false)
        })
    }
}
```

- [ ] **Step 2: Write config tests**

```rust
// tests/config_test.rs
use rgaa_linter::config::Config;
use std::path::Path;

#[test]
fn default_config_loads() {
    let config = Config::load(Path::new("nonexistent.yml")).unwrap();
    assert!(config.exclude.is_empty());
}

#[test]
fn parse_yaml_config() {
    let yaml = r#"
exclude:
  - "test/**/*"
  - "*.min.html"
include:
  - "**/*.html"
rules:
  image-alt: error
  heading-order: warning
"#;
    let config: Config = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.exclude.len(), 2);
    assert_eq!(config.rules.get("image-alt").unwrap(), "error");
}

#[test]
fn is_excluded_matches_glob() {
    let config = Config {
        exclude: vec!["test/**/*".to_string()],
        ..Default::default()
    };
    assert!(config.is_excluded("test/fixtures/foo.html"));
    assert!(!config.is_excluded("src/index.html"));
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p rgaa-linter`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add rgaa-rs/crates/rgaa-linter/src/config.rs rgaa-rs/crates/rgaa-linter/tests/config_test.rs
git commit -m "feat(rgaa-linter): add .rgaa-lint.yml config loading"
```

---

## Task 6: Output Formatters

**Files:**
- Create: `rgaa-rs/crates/rgaa-linter/src/output/mod.rs`
- Create: `rgaa-rs/crates/rgaa-linter/src/output/pretty.rs`
- Create: `rgaa-rs/crates/rgaa-linter/src/output/json.rs`
- Create: `rgaa-rs/crates/rgaa-linter/src/output/github.rs`

**Interfaces:**
- Consumes: `LintResult`
- Produces: formatted strings for each output mode

- [ ] **Step 1: Create output/mod.rs**

```rust
pub mod pretty;
pub mod json;
pub mod github;

use crate::types::LintResult;

pub enum OutputFormat {
    Pretty,
    Json,
    GitHub,
}

pub fn format_output(result: &LintResult, format: &OutputFormat) -> String {
    match format {
        OutputFormat::Pretty => pretty::format(result),
        OutputFormat::Json => json::format(result),
        OutputFormat::GitHub => github::format(result),
    }
}
```

- [ ] **Step 2: Create pretty.rs**

```rust
use crate::types::LintResult;
use colored::*;

pub fn format(result: &LintResult) -> String {
    let mut output = String::new();

    for v in &result.violations {
        let severity = match v.severity {
            crate::types::Severity::Error => "error".red().bold(),
            crate::types::Severity::Warning => "warning".yellow().bold(),
            crate::types::Severity::Info => "info".blue().bold(),
        };
        output.push_str(&format!(
            "{}:{}:{} {} [{}] {}\n",
            v.file.display(), v.line, v.col,
            severity, v.rule_id, v.message
        ));
    }

    output.push_str(&format!(
        "\n{} checked, {} errors, {} warnings\n",
        result.files_checked, result.error_count(), result.warning_count()
    ));

    output
}
```

- [ ] **Step 3: Create json.rs**

```rust
use crate::types::LintResult;

pub fn format(result: &LintResult) -> String {
    serde_json::to_string_pretty(result).unwrap_or_default()
}
```

- [ ] **Step 4: Create github.rs**

```rust
use crate::types::{LintResult, Severity};

pub fn format(result: &LintResult) -> String {
    let mut output = String::new();

    // Annotations
    for v in &result.violations {
        let level = match v.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "notice",
        };
        output.push_str(&format!(
            "::{} file={},line={},col={}::{} - {}\n",
            level,
            v.file.display(),
            v.line,
            v.col,
            v.rule_id,
            v.message
        ));
    }

    // Step summary
    output.push_str("\n## RGAA Lint Results\n\n");
    output.push_str(&format!(
        "| Metric | Value |\n|--------|-------|\n| Files checked | {} |\n| Errors | {} |\n| Warnings | {} |\n",
        result.files_checked, result.error_count(), result.warning_count()
    ));

    output
}
```

- [ ] **Step 5: Commit**

```bash
git add rgaa-rs/crates/rgaa-linter/src/output/
git commit -m "feat(rgaa-linter): add pretty, JSON, and GitHub output formatters"
```

---

## Task 7: CLI

**Files:**
- Create: `rgaa-rs/crates/rgaa-linter/src/cli.rs`

**Interfaces:**
- Consumes: `Config`, `StaticEngine`, output formatters
- Produces: exit code (0 = pass, 1 = errors, 2 = runtime error)

- [ ] **Step 1: Create cli.rs**

```rust
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rgaa-lint")]
#[command(about = "Pure-Rust accessibility linter for RGAA/WCAG compliance")]
pub struct Cli {
    /// Files or directories to lint
    #[arg(default_value = ".")]
    pub paths: Vec<PathBuf>,

    /// Output format
    #[arg(long, default_value = "pretty")]
    pub format: String,

    /// Config file path
    #[arg(long, default_value = ".rgaa-lint.yml")]
    pub config: PathBuf,

    /// Fail on this severity level
    #[arg(long, default_value = "error")]
    pub fail_on: String,

    /// Enable runtime rules (requires Chrome)
    #[arg(long)]
    pub browser: bool,

    /// Only lint files changed since last commit
    #[arg(long)]
    pub changed: bool,

    /// Auto-fix where possible
    #[arg(long)]
    pub fix: bool,

    /// Exclude files matching pattern
    #[arg(long)]
    pub exclude: Vec<String>,

    /// Only run specific rules (comma-separated)
    #[arg(long)]
    pub rules: Option<String>,

    /// Skip specific rules (comma-separated)
    #[arg(long)]
    pub skip: Option<String>,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,
}
```

- [ ] **Step 2: Add main entry point**

Create `rgaa-rs/crates/rgaa-linter/src/main.rs`:

```rust
use clap::Parser;
use rgaa_linter::cli::Cli;
use rgaa_linter::config::Config;
use rgaa_linter::engine::StaticEngine;
use rgaa_linter::output::{self, OutputFormat};
use rgaa_linter::types::LintResult;
use std::path::Path;
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_env_filter("rgaa_linter=debug")
            .init();
    }

    // Load config
    let config = Config::load(&cli.config)?;

    // Discover files
    let files = discover_files(&cli.paths, &config, &cli.changed)?;

    // Create engine
    let engine = StaticEngine::new();

    // Lint all files
    let start = Instant::now();
    let mut all_violations = Vec::new();

    for file in &files {
        let html = std::fs::read_to_string(file)?;
        let violations = engine.lint(&html, file);
        all_violations.extend(violations);
    }

    let result = LintResult {
        violations: all_violations,
        files_checked: files.len(),
        rules_run: 70, // TODO: actual count from engine
        duration_ms: start.elapsed().as_millis() as u64,
    };

    // Format output
    let format = match cli.format.as_str() {
        "json" => OutputFormat::Json,
        "github" => OutputFormat::GitHub,
        _ => OutputFormat::Pretty,
    };
    print!("{}", output::format_output(&result, &format));

    // Exit code
    if result.has_errors() {
        std::process::exit(1);
    }
    Ok(())
}

fn discover_files(
    paths: &[PathBuf],
    config: &Config,
    changed_only: bool,
) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for path in paths {
        if path.is_file() {
            let rel = path.strip_prefix(".").unwrap_or(path).to_string_lossy();
            if !config.is_excluded(&rel) && config.should_include(&rel) {
                files.push(path.clone());
            }
        } else if path.is_dir() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let file_path = entry.path();
                if file_path.is_file() {
                    let rel = file_path.strip_prefix(".").unwrap_or(&file_path)
                        .to_string_lossy();
                    if !config.is_excluded(&rel) && config.should_include(&rel) {
                        files.push(file_path);
                    }
                }
            }
        }
    }

    Ok(files)
}
```

- [ ] **Step 3: Update Cargo.toml for binary**

Add to `rgaa-linter/Cargo.toml`:
```toml
[[bin]]
name = "rgaa-lint"
path = "src/main.rs"
```

- [ ] **Step 4: Test CLI builds**

Run: `cargo build -p rgaa-lint`
Expected: Binary builds successfully

- [ ] **Step 5: Commit**

```bash
git add rgaa-rs/crates/rgaa-linter/src/cli.rs rgaa-rs/crates/rgaa-linter/src/main.rs
git commit -m "feat(rgaa-linter): implement CLI with clap"
```

---

## Task 8: GitHub Action

**Files:**
- Create: `rgaa-rs/crates/rgaa-linter/action.yml`
- Create: `rgaa-rs/crates/rgaa-linter/scripts/install-lint.sh`

**Interfaces:**
- Consumes: `rgaa-lint` binary
- Produces: GitHub Actions annotations + step summary

- [ ] **Step 1: Create action.yml**

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
    description: 'Linting mode: static or full'
    required: false
    default: 'static'
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
        curl -fsSL https://rgaa.dev/install-lint.sh | sh
    - name: Run RGAA Linter
      shell: bash
      run: |
        ARGS=""
        if [ "${{ inputs.only-changed }}" = "true" ]; then
          ARGS="$ARGS --changed"
        fi
        rgaa-lint ${{ inputs.path }} \
          --format ${{ inputs.format }} \
          --config ${{ inputs.config }} \
          --fail-on ${{ inputs.fail-on }} \
          $ARGS
```

- [ ] **Step 2: Create install-lint.sh**

```bash
#!/bin/bash
set -euo pipefail

VERSION="${RGAA_LINT_VERSION:-latest}"
INSTALL_DIR="${HOME}/.local/bin"

mkdir -p "$INSTALL_DIR"

# Detect OS and arch
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$ARCH" in
  x86_64)  ARCH="x86_64" ;;
  aarch64|arm64) ARCH="aarch64" ;;
esac

case "$OS" in
  linux)  TARGET="${ARCH}-unknown-linux-gnu" ;;
  darwin) TARGET="${ARCH}-apple-darwin" ;;
  *) echo "Unsupported OS: $OS"; exit 1 ;;
esac

if [ "$VERSION" = "latest" ]; then
  VERSION=$(curl -fsSL https://api.github.com/repos/your-org/rgaa-rs/releases/latest | grep tag_name | cut -d'"' -f4)
fi

URL="https://github.com/your-org/rgaa-rs/releases/download/${VERSION}/rgaa-lint-${TARGET}.tar.gz"

curl -fsSL "$URL" | tar xz -C "$INSTALL_DIR"

echo "Installed rgaa-lint to ${INSTALL_DIR}/rgaa-lint"
```

- [ ] **Step 3: Commit**

```bash
git add rgaa-rs/crates/rgaa-linter/action.yml rgaa-rs/crates/rgaa-linter/scripts/
git commit -m "feat(rgaa-linter): add GitHub Action + install script"
```

---

## Task 9: End-to-End Test Fixtures

**Files:**
- Create: `rgaa-rs/crates/rgaa-linter/fixtures/pass/valid.html`
- Create: `rgaa-rs/crates/rgaa-linter/fixtures/fail/` (one file per rule category)

**Interfaces:**
- Produces: test HTML files that exercise all rule categories

- [ ] **Step 1: Create valid.html**

A clean HTML file that passes all 70+ rules. This is the reference document.

- [ ] **Step 2: Create fail/ fixture files**

One file per rule category:
- `missing-alt.html` — triggers image-alt, input-image-alt
- `missing-lang.html` — triggers html-has-lang, html-lang-valid
- `missing-title.html` — triggers document-title
- `bad-heading-order.html` — triggers heading-order
- `duplicate-id.html` — triggers duplicate-id
- `missing-label.html` — triggers label, select-name, textarea-name
- `bad-aria.html` — triggers aria-allowed-attr, aria-valid-role
- `missing-landmark.html` — triggers landmark-one-main, region
- `table-missing-header.html` — triggers td-has-header, th-has-data-cells
- `deprecated-elements.html` — triggers marquee, blink, html5elem

- [ ] **Step 3: Write fixture test**

```rust
// Add to tests/static_rules_test.rs
use rgaa_linter::engine::StaticEngine;
use std::path::Path;

#[test]
fn valid_fixture_passes() {
    let html = std::fs::read_to_string("fixtures/pass/valid.html").unwrap();
    let engine = StaticEngine::new();
    let violations = engine.lint(&html, Path::new("valid.html"));
    assert!(violations.is_empty(), "Valid fixture should pass: {:?}", violations);
}

#[test]
fn missing_alt_fixture_fails() {
    let html = std::fs::read_to_string("fixtures/fail/missing-alt.html").unwrap();
    let engine = StaticEngine::new();
    let violations = engine.lint(&html, Path::new("missing-alt.html"));
    assert!(violations.iter().any(|v| v.rule_id == "image-alt"));
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p rgaa-linter`
Expected: All fixture tests pass

- [ ] **Step 5: Commit**

```bash
git add rgaa-rs/crates/rgaa-linter/fixtures/
git commit -m "test(rgaa-linter): add end-to-end test fixtures"
```

---

## Task 10: Performance + Polish

**Files:**
- Modify: `rgaa-rs/crates/rgaa-linter/src/engine/static_engine.rs`
- Modify: `rgaa-rs/crates/rgaa-linter/src/main.rs`

**Interfaces:**
- Produces: parallel file processing, progress reporting

- [ ] **Step 1: Add parallel file processing**

Use `rayon` for parallel linting across files:

```toml
[dependencies]
rayon = "1.10"
```

```rust
use rayon::prelude::*;

// In main.rs, replace sequential loop with:
let all_violations: Vec<_> = files.par_iter()
    .map(|file| {
        let html = std::fs::read_to_string(file).unwrap();
        engine.lint(&html, file)
    })
    .flatten()
    .collect();
```

- [ ] **Step 2: Add --changed flag implementation**

```rust
fn get_changed_files() -> anyhow::Result<Vec<PathBuf>> {
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", "HEAD~1"])
        .output()?;
    
    Ok(String::from_utf8(output.stdout)?
        .lines()
        .filter(|l| l.ends_with(".html") || l.ends_with(".htm"))
        .map(PathBuf::from)
        .collect())
}
```

- [ ] **Step 3: Run benchmarks**

Run: `cargo test -p rgaa-linter`
Run: `time rgaa-lint fixtures/` (measure wall clock)

- [ ] **Step 4: Commit**

```bash
git add rgaa-rs/crates/rgaa-linter/
git commit -m "perf(rgaa-linter): add parallel file processing + --changed flag"
```

---

## Summary

| Task | What it delivers | Tests |
|------|-----------------|-------|
| 1 | Crate scaffold + types | Compiles |
| 2 | 70+ rule definitions | Rule count, unique IDs |
| 3 | Static linting engine | Valid HTML passes, missing alt fails |
| 4 | Auto-fix module | Fix insertion, can_auto_fix |
| 5 | Config loading | YAML parse, glob matching |
| 6 | Output formatters | Pretty, JSON, GitHub annotations |
| 7 | CLI binary | Builds, runs on fixtures |
| 8 | GitHub Action | action.yml + install script |
| 9 | Test fixtures | All fixture tests pass |
| 10 | Performance | Parallel processing, --changed |

**Total estimated time:** 5-7 days for a focused developer.
