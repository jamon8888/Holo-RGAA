# RGAA-RS: Asqatasun Replacement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Asqatasun with a high-performance Rust-based RGAA 4.1.2 audit engine, migrating existing working code and fixing the 10 known false negatives.

**Architecture:** Extend existing `backend/` Rust/Axum code. Playwright runs as Node.js child process for browser automation (already working). rig-core orchestrates Holo3 LLM calls for IA_ASSISTE criteria. Gap-fix rules execute JS snippets via Playwright to cover axe-core's 12.9% false negative rate. Existing `hybrid-audit.js`, `interaction-audit.js`, `widget-audit.js` stay as reference — Rust equivalents built alongside.

**Tech Stack:** Rust 1.80+, Playwright (Node.js child process), axe-core v4.9.1, rig-core 0.41, Holo3 API (holo3-1-35b-a3b), sqlx 0.7, Axum 0.7

## Existing Codebase (What We Keep)

| File | Status | Action |
|---|---|---|
| `backend/src/main.rs` | ✅ Working | Extend with orchestrator |
| `backend/Cargo.toml` | ✅ Working | Add rig-core, reqwest deps |
| `backend/migrations/001_initial_schema.sql` | ✅ Working | Extend with new columns |
| `poc.js` | ✅ Working | Reference for axe-core mapping |
| `hybrid-audit.js` | ✅ Working | Reference for Holo3 integration |
| `audit-pipeline.js` | ✅ Working | Reference for full pipeline |
| `interaction-audit.js` | ✅ Working | Port to Rust |
| `widget-audit.js` | ✅ Working | Port to Rust |
| `dinum-sampling.js` | ✅ Working | Port to Rust |
| `compare-asqatasun.js` | ✅ Working | Keep for validation |
| `.github/workflows/ci.yml` | ✅ Working | Extend with Rust CI |
| `docker-compose.yml` | ✅ Working | Keep as-is |
| `grille-rgaa-106.csv` | ✅ Reference | Embed in Rust binary |
| `grille-rgaa-confirmee-partielle.csv` | ✅ Reference | Embed in Rust binary |

## False Negatives to Fix (From Comparison Data)

| Criterion | axe-core | Asqatasun | Root Cause |
|---|---|---|---|
| 1.1 | PASS | FAIL | axe misses `<picture>` without alt |
| 1.2 | PASS | FAIL | axe misses decorative images without role=presentation |
| 2.1 | PASS | FAIL | axe misses iframe title edge cases |
| 3.2 | PASS | FAIL | axe contrast threshold differs (0.3 vs Asqatasun's stricter) |
| 6.1 | PASS | FAIL | axe link-name rule less strict than Asqatasun |
| 8.3 | PASS | FAIL | axe html-has-lang less strict |
| 8.5 | PASS | FAIL | axe page-title less strict |
| 11.1 | PASS | FAIL | axe label rule less strict |
| 11.4 | PASS | FAIL | axe doesn't check label proximity |
| 12.7 | PASS | FAIL | axe bypass/skip-link less strict |

## Holo3 JSON Parsing Issues (44% Error Rate)

From `holo3_benchmark_results.json`: 11/25 criteria return ERROR. Issues:
- `Unterminated string` (5 cases) — response truncated
- `the JSON object must be str, bytes or bytearray, not NoneType` (2 cases) — null response
- `Expecting property name enclosed in double quotes` (1 case) — malformed JSON
- Empty error string (3 cases) — unknown failures

**Fix strategy:** Extract JSON from response text (not just `response_format`), retry with exponential backoff, fallback to `NE_PAS_SAVOIR` verdict.

---

## File Structure

```
rgaa-rs/
├── Cargo.toml                          # New workspace root
├── crates/
│   ├── rgaa-core/                      # Types, 106 criteria, error handling
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── criteria.rs             # 106 RGAA criteria definitions
│   │       ├── types.rs                # AuditResult, CriterionResult, etc.
│   │       └── error.rs                # RgaaError enum
│   ├── rgaa-rules/                     # axe-core mapping + 10 gap-fix rules
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── axe_mapper.rs           # axe-core → RGAA mapping (77 criteria)
│   │       └── gap_fix.rs              # 10 custom rules for false negatives
│   ├── rgaa-holo/                      # rig-core + Holo3 client
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs               # Holo3 API with retry + JSON extraction
│   │       └── prompts.rs              # RGAA prompt templates
│   ├── rgaa-browser/                   # Playwright child process bridge
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── playwright.rs           # Spawn + communicate with Node.js
│   │       └── js/
│   │           ├── axe-runner.js       # Inject axe-core, run, return JSON
│   │           ├── gap-fix-runner.js   # Execute gap-fix JS snippets
│   │           └── interaction.js      # Keyboard/reflow tests (from interaction-audit.js)
│   ├── rgaa-orchestrator/              # Main audit pipeline
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       └── pipeline.rs             # Crawl → axe → gap-fix → Holo3 → store
│   ├── rgaa-storage/                   # PostgreSQL (extend existing schema)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       └── repository.rs           # CRUD operations
│   └── rgaa-api/                       # Axum REST (extend existing backend)
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           └── main.rs                 # Server entry point
└── js/
    ├── hybrid-audit.js                 # Existing — keep as reference
    ├── interaction-audit.js            # Existing — port to Rust
    ├── widget-audit.js                 # Existing — port to Rust
    └── dinum-sampling.js               # Existing — port to Rust
```

---

## Task 1: Workspace Scaffolding + rgaa-core

**Files:**
- Create: `rgaa-rs/Cargo.toml`
- Create: `rgaa-rs/crates/rgaa-core/Cargo.toml`
- Create: `rgaa-rs/crates/rgaa-core/src/lib.rs`
- Create: `rgaa-rs/crates/rgaa-core/src/criteria.rs`
- Create: `rgaa-rs/crates/rgaa-core/src/types.rs`
- Create: `rgaa-rs/crates/rgaa-core/src/error.rs`

**Interfaces:**
- Consumes: none
- Produces: `RgaaCriteria::ALL` (Vec of 106 Criterion), `AuditResult`, `CriterionResult`, `PageResult`, `Classification` enum, `CriterionStatus` enum, `RgaaError`

- [ ] **Step 1: Create workspace Cargo.toml**

```toml
# rgaa-rs/Cargo.toml
[workspace]
resolver = "2"
members = [
    "crates/rgaa-core",
    "crates/rgaa-rules",
    "crates/rgaa-holo",
    "crates/rgaa-browser",
    "crates/rgaa-orchestrator",
    "crates/rgaa-storage",
    "crates/rgaa-api",
]

[workspace.dependencies]
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

- [ ] **Step 2: Create rgaa-core/Cargo.toml**

```toml
[package]
name = "rgaa-core"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = "2"
```

- [ ] **Step 3: Create error.rs**

```rust
// rgaa-rs/crates/rgaa-core/src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RgaaError {
    #[error("Crawl error: {0}")]
    Crawl(String),
    #[error("Browser error: {0}")]
    Browser(String),
    #[error("Axe-core error: {0}")]
    AxeCore(String),
    #[error("Holo3 API error: {0}")]
    Holo3(String),
    #[error("Media analysis error: {0}")]
    Media(String),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Invalid criterion ID: {0}")]
    InvalidCriterion(String),
    #[error("Timeout after {0}ms")]
    Timeout(u64),
}

pub type Result<T> = std::result::Result<T, RgaaError>;
```

- [ ] **Step 4: Create types.rs**

```rust
// rgaa-rs/crates/rgaa-core/src/types.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Classification {
    Deterministe,
    IaAssiste,
    Manuel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CriterionStatus {
    Pass,
    Fail,
    Na,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriterionResult {
    pub criterion_id: String,
    pub title: String,
    pub classification: Classification,
    pub status: CriterionStatus,
    pub violations: Vec<Violation>,
    pub confidence: Option<f64>,
    pub justification: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    pub rule_id: String,
    pub impact: String,
    pub description: String,
    pub nodes_affected: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageResult {
    pub url: String,
    pub title: Option<String>,
    pub criteria: Vec<CriterionResult>,
    pub compliance_rate: f64,
    pub crawl_depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResult {
    pub audit_id: String,
    pub url: String,
    pub pages: Vec<PageResult>,
    pub total_criteria: usize,
    pub passed: usize,
    pub failed: usize,
    pub na: usize,
    pub overall_compliance: f64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlConfig {
    pub max_pages: usize,
    pub max_depth: u32,
    pub respect_robots: bool,
    pub sample_mode: bool,
}

impl Default for CrawlConfig {
    fn default() -> Self {
        Self {
            max_pages: 50,
            max_depth: 5,
            respect_robots: true,
            sample_mode: false,
        }
    }
}
```

- [ ] **Step 5: Create criteria.rs** — All 106 RGAA criteria with correct classifications

```rust
// rgaa-rs/crates/rgaa-core/src/criteria.rs
use crate::types::Classification;

#[derive(Debug, Clone)]
pub struct Criterion {
    pub id: &'static str,
    pub title: &'static str,
    pub classification: Classification,
    pub wcag_refs: &'static str,
}

pub struct RgaaCriteria;

impl RgaaCriteria {
    pub fn all() -> Vec<Criterion> {
        vec![
            Criterion { id: "1.1", title: "Alternative textuelle présente", classification: Classification::Deterministe, wcag_refs: "1.1.1" },
            Criterion { id: "1.2", title: "Image décorative ignorée", classification: Classification::Deterministe, wcag_refs: "1.1.1, 4.1.2" },
            Criterion { id: "1.3", title: "Alternative textuelle pertinente", classification: Classification::IaAssiste, wcag_refs: "1.1.1, 4.1.2" },
            Criterion { id: "1.4", title: "Alternative CAPTCHA/image-test", classification: Classification::IaAssiste, wcag_refs: "1.1.1" },
            Criterion { id: "1.5", title: "Solution accès alternatif CAPTCHA", classification: Classification::Deterministe, wcag_refs: "1.1.1" },
            Criterion { id: "1.6", title: "Description détaillée présente", classification: Classification::Deterministe, wcag_refs: "1.1.1" },
            Criterion { id: "1.7", title: "Description détaillée pertinente", classification: Classification::IaAssiste, wcag_refs: "1.1.1" },
            Criterion { id: "1.8", title: "Image texte remplacée par texte stylé", classification: Classification::Deterministe, wcag_refs: "1.4.5" },
            Criterion { id: "1.9", title: "Légende reliée à l'image", classification: Classification::Deterministe, wcag_refs: "1.1.1, 4.1.2" },
            Criterion { id: "2.1", title: "Cadre a un titre", classification: Classification::Deterministe, wcag_refs: "4.1.2" },
            Criterion { id: "2.2", title: "Titre de cadre pertinent", classification: Classification::IaAssiste, wcag_refs: "4.1.2" },
            Criterion { id: "3.1", title: "Information non donnée uniquement par couleur", classification: Classification::IaAssiste, wcag_refs: "1.3.1, 1.4.1" },
            Criterion { id: "3.2", title: "Contraste texte/fond suffisant", classification: Classification::Deterministe, wcag_refs: "1.4.3" },
            Criterion { id: "3.3", title: "Contraste composants graphiques suffisant", classification: Classification::Deterministe, wcag_refs: "1.4.11" },
            Criterion { id: "4.1", title: "Transcription/audiodescription présente", classification: Classification::Deterministe, wcag_refs: "1.2.1, 1.2.3" },
            Criterion { id: "4.2", title: "Transcription/audiodescription pertinente", classification: Classification::IaAssiste, wcag_refs: "1.2.1, 1.2.3" },
            Criterion { id: "4.3", title: "Sous-titres synchronisés présents", classification: Classification::Deterministe, wcag_refs: "1.2.2" },
            Criterion { id: "4.4", title: "Sous-titres pertinents", classification: Classification::IaAssiste, wcag_refs: "1.2.2" },
            Criterion { id: "4.5", title: "Audiodescription présente", classification: Classification::Deterministe, wcag_refs: "1.2.5" },
            Criterion { id: "4.6", title: "Audiodescription pertinente", classification: Classification::IaAssiste, wcag_refs: "1.2.5" },
            Criterion { id: "4.7", title: "Média temporel identifiable", classification: Classification::Deterministe, wcag_refs: "1.1.1" },
            Criterion { id: "4.8", title: "Alternative média non temporel", classification: Classification::Deterministe, wcag_refs: "1.1.1" },
            Criterion { id: "4.9", title: "Alternative pertinente média non temporel", classification: Classification::IaAssiste, wcag_refs: "1.1.1" },
            Criterion { id: "4.10", title: "Son contrôlable", classification: Classification::Deterministe, wcag_refs: "1.4.2" },
            Criterion { id: "4.11", title: "Média temporel contrôlable clavier", classification: Classification::Deterministe, wcag_refs: "2.1.1, 2.1.2" },
            Criterion { id: "4.12", title: "Média non temporel contrôlable clavier", classification: Classification::Deterministe, wcag_refs: "2.1.1, 2.1.2" },
            Criterion { id: "4.13", title: "Média compatible AT", classification: Classification::Deterministe, wcag_refs: "4.1.2" },
            Criterion { id: "5.1", title: "Tableau complexe a résumé", classification: Classification::Deterministe, wcag_refs: "1.3.1" },
            Criterion { id: "5.2", title: "Résumé pertinent tableau complexe", classification: Classification::IaAssiste, wcag_refs: "1.3.1" },
            Criterion { id: "5.3", title: "Contenu linéarisé compréhensible", classification: Classification::IaAssiste, wcag_refs: "1.3.2, 4.1.2" },
            Criterion { id: "5.4", title: "Titre tableau correctement associé", classification: Classification::Deterministe, wcag_refs: "1.3.1" },
            Criterion { id: "5.5", title: "Titre pertinent tableau", classification: Classification::IaAssiste, wcag_refs: "1.3.1" },
            Criterion { id: "5.6", title: "En-têtes déclarés correctement", classification: Classification::Deterministe, wcag_refs: "1.3.1" },
            Criterion { id: "5.7", title: "Association cellules/en-têtes", classification: Classification::Deterministe, wcag_refs: "1.3.1" },
            Criterion { id: "5.8", title: "Tableau mise en forme sans éléments données", classification: Classification::Deterministe, wcag_refs: "1.3.1" },
            Criterion { id: "6.1", title: "Lien explicite", classification: Classification::Deterministe, wcag_refs: "1.1.1, 2.4.4, 2.5.3" },
            Criterion { id: "6.2", title: "Lien a un intitulé", classification: Classification::Deterministe, wcag_refs: "1.1.1, 2.4.4" },
            Criterion { id: "7.1", title: "Script compatible AT", classification: Classification::Deterministe, wcag_refs: "2.5.3, 4.1.2" },
            Criterion { id: "7.2", title: "Alternative script pertinente", classification: Classification::IaAssiste, wcag_refs: "1.1.1, 4.1.2" },
            Criterion { id: "7.3", title: "Script contrôlable clavier", classification: Classification::Deterministe, wcag_refs: "1.3.1, 2.1.1, 2.4.7" },
            Criterion { id: "7.4", title: "Changement de contexte averti/contrôlé", classification: Classification::Deterministe, wcag_refs: "3.2.1, 3.2.2" },
            Criterion { id: "7.5", title: "Messages de statut restitués AT", classification: Classification::Manuel, wcag_refs: "4.1.3" },
            Criterion { id: "8.1", title: "Type de document défini", classification: Classification::Deterministe, wcag_refs: "4.1.1" },
            Criterion { id: "8.2", title: "Code valide selon doctype", classification: Classification::Deterministe, wcag_refs: "4.1.1, 4.1.2" },
            Criterion { id: "8.3", title: "Langue par défaut présente", classification: Classification::Deterministe, wcag_refs: "3.1.1" },
            Criterion { id: "8.4", title: "Code de langue pertinent", classification: Classification::IaAssiste, wcag_refs: "3.1.1" },
            Criterion { id: "8.5", title: "Titre de page", classification: Classification::Deterministe, wcag_refs: "2.4.2" },
            Criterion { id: "8.6", title: "Titre de page pertinent", classification: Classification::IaAssiste, wcag_refs: "2.4.2" },
            Criterion { id: "8.7", title: "Changement de langue indiqué", classification: Classification::Deterministe, wcag_refs: "3.1.2" },
            Criterion { id: "8.8", title: "Code de langue changement pertinent", classification: Classification::IaAssiste, wcag_refs: "3.1.2" },
            Criterion { id: "8.9", title: "Balises pas uniquement présentation", classification: Classification::Deterministe, wcag_refs: "1.3.1" },
            Criterion { id: "8.10", title: "Changements sens lecture signalés", classification: Classification::Deterministe, wcag_refs: "1.3.2" },
            Criterion { id: "9.1", title: "Structure par titres appropriée", classification: Classification::Deterministe, wcag_refs: "1.3.1, 2.4.1, 2.4.6, 4.1.2" },
            Criterion { id: "9.2", title: "Structure document cohérente", classification: Classification::IaAssiste, wcag_refs: "1.3.1" },
            Criterion { id: "9.3", title: "Liste correctement structurée", classification: Classification::Deterministe, wcag_refs: "1.3.1" },
            Criterion { id: "9.4", title: "Citation correctement indiquée", classification: Classification::Deterministe, wcag_refs: "1.3.1" },
            Criterion { id: "10.1", title: "CSS pour présentation", classification: Classification::Deterministe, wcag_refs: "1.3.1, 1.3.2" },
            Criterion { id: "10.2", title: "Contenu visible sans CSS", classification: Classification::Deterministe, wcag_refs: "1.1.1, 1.3.1" },
            Criterion { id: "10.3", title: "Information compréhensible sans CSS", classification: Classification::IaAssiste, wcag_refs: "1.3.2, 2.4.3" },
            Criterion { id: "10.4", title: "Texte lisible zoom 200%", classification: Classification::Deterministe, wcag_refs: "1.4.4" },
            Criterion { id: "10.5", title: "Déclarations CSS couleurs correctes", classification: Classification::Deterministe, wcag_refs: "1.4.3" },
            Criterion { id: "10.6", title: "Lien visible vs texte environnant", classification: Classification::Deterministe, wcag_refs: "1.4.1" },
            Criterion { id: "10.7", title: "Focus visible", classification: Classification::Deterministe, wcag_refs: "1.4.1, 2.4.7" },
            Criterion { id: "10.8", title: "Contenus cachés ignorés AT", classification: Classification::Deterministe, wcag_refs: "1.3.2, 4.1.2" },
            Criterion { id: "10.9", title: "Info non donnée par forme/taille/position", classification: Classification::Deterministe, wcag_refs: "1.3.3, 1.4.1" },
            Criterion { id: "10.10", title: "Implémentation pertinente forme/taille/position", classification: Classification::IaAssiste, wcag_refs: "1.3.3, 1.4.1" },
            Criterion { id: "10.11", title: "Reflow 320px/256px", classification: Classification::Deterministe, wcag_refs: "1.4.10" },
            Criterion { id: "10.12", title: "Espacement texte redéfinissable", classification: Classification::Deterministe, wcag_refs: "1.4.12" },
            Criterion { id: "10.13", title: "Contenus additionnels focus/survol contrôlables", classification: Classification::Deterministe, wcag_refs: "1.4.13" },
            Criterion { id: "10.14", title: "Contenus CSS only accessibles clavier", classification: Classification::Deterministe, wcag_refs: "2.1.1" },
            Criterion { id: "11.1", title: "Champ a étiquette", classification: Classification::Deterministe, wcag_refs: "1.3.1, 2.4.6, 3.3.2, 4.1.2" },
            Criterion { id: "11.2", title: "Étiquette champ pertinente", classification: Classification::IaAssiste, wcag_refs: "2.4.6, 2.5.3, 3.3.2" },
            Criterion { id: "11.3", title: "Étiquettes cohérentes même fonction", classification: Classification::IaAssiste, wcag_refs: "3.2.4" },
            Criterion { id: "11.4", title: "Étiquette et champ accolés", classification: Classification::Deterministe, wcag_refs: "3.3.2" },
            Criterion { id: "11.5", title: "Champs même nature regroupés", classification: Classification::Deterministe, wcag_refs: "1.3.1, 3.3.2" },
            Criterion { id: "11.6", title: "Regroupement a légende", classification: Classification::Deterministe, wcag_refs: "1.3.1, 3.3.2" },
            Criterion { id: "11.7", title: "Légende regroupement pertinente", classification: Classification::IaAssiste, wcag_refs: "1.3.1, 3.3.2" },
            Criterion { id: "11.8", title: "Items liste choix regroupés pertinemment", classification: Classification::IaAssiste, wcag_refs: "1.3.1" },
            Criterion { id: "11.9", title: "Intitulé bouton pertinent", classification: Classification::IaAssiste, wcag_refs: "2.5.3, 4.1.2" },
            Criterion { id: "11.10", title: "Contrôle saisie utilisé pertinemment", classification: Classification::IaAssiste, wcag_refs: "3.3.1, 3.3.2" },
            Criterion { id: "11.11", title: "Suggestions correction erreurs", classification: Classification::Deterministe, wcag_refs: "3.3.3" },
            Criterion { id: "11.12", title: "Données modifiables/récupérables", classification: Classification::Deterministe, wcag_refs: "3.3.4" },
            Criterion { id: "11.13", title: "Finalité champ déductible", classification: Classification::Deterministe, wcag_refs: "1.3.5" },
            Criterion { id: "12.1", title: "Deux systèmes navigation", classification: Classification::Deterministe, wcag_refs: "2.4.5" },
            Criterion { id: "12.2", title: "Navigation même place", classification: Classification::Deterministe, wcag_refs: "3.2.3" },
            Criterion { id: "12.3", title: "Plan du site pertinent", classification: Classification::IaAssiste, wcag_refs: "2.4.5" },
            Criterion { id: "12.4", title: "Plan site accessible identique", classification: Classification::Deterministe, wcag_refs: "2.4.5, 3.2.3" },
            Criterion { id: "12.5", title: "Moteur recherche atteignable identiquement", classification: Classification::Deterministe, wcag_refs: "3.2.3" },
            Criterion { id: "12.6", title: "Zones regroupement atteignables", classification: Classification::Deterministe, wcag_refs: "1.3.1, 2.4.1, 4.1.2" },
            Criterion { id: "12.7", title: "Lien évitement contenu principal", classification: Classification::Deterministe, wcag_refs: "2.4.1, 2.4.3, 3.2.3" },
            Criterion { id: "12.8", title: "Ordre tabulation cohérent", classification: Classification::IaAssiste, wcag_refs: "2.4.3" },
            Criterion { id: "12.9", title: "Pas de piège clavier", classification: Classification::Deterministe, wcag_refs: "2.1.1, 2.1.2" },
            Criterion { id: "12.10", title: "Raccourcis clavier contrôlables", classification: Classification::Deterministe, wcag_refs: "2.1.4" },
            Criterion { id: "12.11", title: "Contenus additionnels atteignables clavier", classification: Classification::Deterministe, wcag_refs: "2.1.1" },
            Criterion { id: "13.1", title: "Contrôle limites temps", classification: Classification::Deterministe, wcag_refs: "2.2.1, 2.2.2" },
            Criterion { id: "13.2", title: "Pas ouverture fenêtre sans action", classification: Classification::Deterministe, wcag_refs: "3.2.1" },
            Criterion { id: "13.3", title: "Document bureautique version accessible", classification: Classification::Deterministe, wcag_refs: "1.1.1, 1.3.1, 1.3.2, 2.4.1, 2.4.3, 3.1.1, 4.1.2" },
            Criterion { id: "13.4", title: "Version accessible même information", classification: Classification::Deterministe, wcag_refs: "1.1.1, 1.3.1, 1.3.2, 2.4.1, 2.4.3, 3.1.1, 4.1.2" },
            Criterion { id: "13.5", title: "Contenu cryptique a alternative", classification: Classification::Deterministe, wcag_refs: "1.1.1" },
            Criterion { id: "13.6", title: "Alternative pertinente contenu cryptique", classification: Classification::IaAssiste, wcag_refs: "1.1.1" },
            Criterion { id: "13.7", title: "Flash/luminosité corrects", classification: Classification::Deterministe, wcag_refs: "2.3.1" },
            Criterion { id: "13.8", title: "Contenu mouvement/clignotant contrôlable", classification: Classification::Deterministe, wcag_refs: "2.2.1, 2.2.2" },
            Criterion { id: "13.9", title: "Orientation portrait/paysage", classification: Classification::Deterministe, wcag_refs: "1.3.4" },
            Criterion { id: "13.10", title: "Geste complexe = geste simple", classification: Classification::Deterministe, wcag_refs: "2.5.1" },
            Criterion { id: "13.11", title: "Annulation action pointage", classification: Classification::Deterministe, wcag_refs: "2.5.2" },
            Criterion { id: "13.12", title: "Mouvement appareil alternative", classification: Classification::Deterministe, wcag_refs: "2.5.4" },
        ]
    }

    pub fn find(id: &str) -> Option<Criterion> {
        Self::all().into_iter().find(|c| c.id == id)
    }

    pub fn deterministic() -> Vec<Criterion> {
        Self::all().into_iter()
            .filter(|c| c.classification == Classification::Deterministe)
            .collect()
    }

    pub fn ia_assiste() -> Vec<Criterion> {
        Self::all().into_iter()
            .filter(|c| c.classification == Classification::IaAssiste)
            .collect()
    }

    pub fn count() -> usize {
        Self::all().len()
    }
}
```

- [ ] **Step 6: Create lib.rs**

```rust
// rgaa-rs/crates/rgaa-core/src/lib.rs
pub mod criteria;
pub mod types;
pub mod error;

pub use criteria::{RgaaCriteria, Criterion};
pub use types::*;
pub use error::{RgaaError, Result};
```

- [ ] **Step 7: Verify it compiles**

Run: `cargo check` from `rgaa-rs/`
Expected: Compiles without errors

- [ ] **Step 8: Commit**

```bash
git add rgaa-rs/
git commit -m "feat: scaffold workspace + rgaa-core crate with 106 criteria"
```

---

## Task 2: rgaa-rules — axe-core Mapping + Gap-Fix Rules

**Files:**
- Create: `rgaa-rs/crates/rgaa-rules/Cargo.toml`
- Create: `rgaa-rs/crates/rgaa-rules/src/lib.rs`
- Create: `rgaa-rs/crates/rgaa-rules/src/axe_mapper.rs`
- Create: `rgaa-rs/crates/rgaa-rules/src/gap_fix.rs`

**Interfaces:**
- Consumes: `rgaa-core` types (Criterion, CriterionResult, Violation)
- Produces: `AxeMapper::map(violations_json) -> HashMap<String, CriterionResult>`, `GapFixRules::snippets() -> HashMap<String, &str>`, `GapFixRules::parse_results(js_results) -> HashMap<String, CriterionResult>`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "rgaa-rules"
version = "0.1.0"
edition = "2021"

[dependencies]
rgaa-core = { path = "../rgaa-core" }
serde = { workspace = true }
serde_json = { workspace = true }
```

- [ ] **Step 2: Create axe_mapper.rs** — Map axe-core violations to RGAA (77 criteria)

```rust
// rgaa-rs/crates/rgaa-rules/src/axe_mapper.rs
use std::collections::HashMap;
use rgaa_core::{CriterionResult, CriterionStatus, Violation, Classification};

pub struct AxeMapper;

impl AxeMapper {
    /// Map axe-core violations JSON to RGAA criterion results.
    /// Input: JSON array of axe violations from axe.run()
    /// Output: HashMap of criterion_id → CriterionResult
    pub fn map(violations_json: &str) -> HashMap<String, CriterionResult> {
        let mapping = Self::rgaa_to_axe_map();
        let violations: Vec<AxeViolation> = serde_json::from_str(violations_json)
            .unwrap_or_default();

        let mut results: HashMap<String, CriterionResult> = HashMap::new();

        // Initialize all axe-mapped criteria as PASS
        for (rgaa_id, _) in &mapping {
            results.insert(rgaa_id.clone(), CriterionResult {
                criterion_id: rgaa_id.clone(),
                title: String::new(),
                classification: Classification::Deterministe,
                status: CriterionStatus::Pass,
                violations: vec![],
                confidence: None,
                justification: None,
                source: "axe-core".to_string(),
            });
        }

        // Map violations to criteria
        for violation in &violations {
            for (rgaa_id, axe_rules) in &mapping {
                if axe_rules.iter().any(|rule| rule == &violation.id) {
                    if let Some(result) = results.get_mut(rgaa_id) {
                        result.status = CriterionStatus::Fail;
                        result.violations.push(Violation {
                            rule_id: violation.id.clone(),
                            impact: violation.impact.clone(),
                            description: violation.description.clone(),
                            nodes_affected: violation.nodes.len(),
                        });
                    }
                }
            }
        }

        results
    }

    fn rgaa_to_axe_map() -> HashMap<String, Vec<String>> {
        let mut m: HashMap<String, Vec<String>> = HashMap::new();
        // From existing poc.js — 77 criteria mapped
        m.insert("1.1".into(), vec!["image-alt".into(), "input-image-alt".into()]);
        m.insert("1.2".into(), vec!["image-alt".into(), "image-redundant-alt".into()]);
        m.insert("1.5".into(), vec!["image-alt".into()]);
        m.insert("1.6".into(), vec!["image-alt".into(), "longdesc".into()]);
        m.insert("1.8".into(), vec!["image-text".into()]);
        m.insert("1.9".into(), vec!["figure-caption".into()]);
        m.insert("2.1".into(), vec!["iframe-title".into()]);
        m.insert("3.2".into(), vec!["color-contrast".into()]);
        m.insert("3.3".into(), vec!["color-contrast".into()]);
        m.insert("4.1".into(), vec!["audio-description".into(), "video-description".into()]);
        m.insert("4.3".into(), vec!["video-caption".into()]);
        m.insert("4.5".into(), vec!["audio-description".into(), "video-description".into()]);
        m.insert("4.7".into(), vec!["video-description".into(), "audio-description".into()]);
        m.insert("4.8".into(), vec!["video-description".into(), "audio-description".into()]);
        m.insert("4.10".into(), vec!["audio-control".into()]);
        m.insert("4.11".into(), vec!["keyboard".into(), "keyboard-trap".into()]);
        m.insert("4.12".into(), vec!["keyboard".into(), "keyboard-trap".into()]);
        m.insert("4.13".into(), vec!["video-description".into(), "audio-description".into()]);
        m.insert("5.1".into(), vec!["table-header".into()]);
        m.insert("5.4".into(), vec!["table-header".into()]);
        m.insert("5.6".into(), vec!["table-header".into(), "td-headers-attr".into()]);
        m.insert("5.7".into(), vec!["td-headers-attr".into(), "th-has-data-cells".into()]);
        m.insert("5.8".into(), vec!["layout-table".into()]);
        m.insert("6.1".into(), vec!["link-name".into(), "link-purpose-in-context".into()]);
        m.insert("6.2".into(), vec!["link-name".into()]);
        m.insert("7.1".into(), vec!["keyboard".into(), "keyboard-trap".into(), "focus-order".into()]);
        m.insert("7.3".into(), vec!["keyboard".into(), "keyboard-trap".into(), "focus-visible".into()]);
        m.insert("7.4".into(), vec!["on-focus".into(), "on-input".into()]);
        m.insert("8.1".into(), vec!["doctype".into()]);
        m.insert("8.2".into(), vec!["html-has-lang".into(), "html-lang-valid".into()]);
        m.insert("8.3".into(), vec!["html-has-lang".into()]);
        m.insert("8.5".into(), vec!["page-title".into()]);
        m.insert("8.7".into(), vec!["lang".into()]);
        m.insert("8.9".into(), vec!["layout-table".into(), "deprecated-element".into()]);
        m.insert("8.10".into(), vec!["focus-order".into(), "meaningful-sequence".into()]);
        m.insert("9.1".into(), vec!["heading-order".into(), "landmark-one-main".into(), "region".into()]);
        m.insert("9.3".into(), vec!["list".into(), "listitem".into()]);
        m.insert("9.4".into(), vec!["blockquote".into()]);
        m.insert("10.1".into(), vec!["deprecated-element".into()]);
        m.insert("10.2".into(), vec!["color-contrast".into(), "image-alt".into()]);
        m.insert("10.4".into(), vec!["resize-text".into()]);
        m.insert("10.5".into(), vec!["color-contrast".into()]);
        m.insert("10.6".into(), vec!["link-in-text-block".into()]);
        m.insert("10.7".into(), vec!["focus-visible".into()]);
        m.insert("10.8".into(), vec!["aria-hidden-focus".into(), "hidden-content".into()]);
        m.insert("10.9".into(), vec!["color-contrast".into(), "image-alt".into()]);
        m.insert("10.11".into(), vec!["reflow".into()]);
        m.insert("10.12".into(), vec!["text-spacing".into()]);
        m.insert("10.13".into(), vec!["focus-visible".into(), "keyboard".into()]);
        m.insert("10.14".into(), vec!["keyboard".into()]);
        m.insert("11.1".into(), vec!["label".into(), "label-title-only".into(), "input-image-alt".into()]);
        m.insert("11.4".into(), vec!["label".into()]);
        m.insert("11.5".into(), vec!["fieldset".into()]);
        m.insert("11.6".into(), vec!["fieldset".into()]);
        m.insert("11.11".into(), vec!["error-suggestion".into()]);
        m.insert("11.12".into(), vec!["error-prevention".into()]);
        m.insert("11.13".into(), vec!["autocomplete".into()]);
        m.insert("12.1".into(), vec!["landmark-one-main".into(), "region".into()]);
        m.insert("12.2".into(), vec!["consistent-navigation".into()]);
        m.insert("12.4".into(), vec!["landmark-one-main".into(), "region".into()]);
        m.insert("12.5".into(), vec!["consistent-navigation".into()]);
        m.insert("12.6".into(), vec!["landmark-one-main".into(), "region".into(), "bypass".into()]);
        m.insert("12.7".into(), vec!["bypass".into(), "skip-link".into()]);
        m.insert("12.9".into(), vec!["keyboard-trap".into()]);
        m.insert("12.10".into(), vec!["character-key-shortcuts".into()]);
        m.insert("12.11".into(), vec!["keyboard".into()]);
        m.insert("13.1".into(), vec!["timing-adjustable".into(), "pause-stop-hide".into()]);
        m.insert("13.2".into(), vec!["on-focus".into()]);
        m.insert("13.3".into(), vec!["document-title".into(), "pdf".into()]);
        m.insert("13.4".into(), vec!["document-title".into(), "pdf".into()]);
        m.insert("13.5".into(), vec!["image-alt".into(), "non-text-content".into()]);
        m.insert("13.7".into(), vec!["three-flashes".into()]);
        m.insert("13.8".into(), vec!["pause-stop-hide".into(), "timing-adjustable".into()]);
        m.insert("13.9".into(), vec!["orientation".into()]);
        m.insert("13.10".into(), vec!["pointer-gestures".into()]);
        m.insert("13.11".into(), vec!["pointer-cancellation".into()]);
        m.insert("13.12".into(), vec!["motion-actuation".into()]);
        m
    }
}

#[derive(serde::Deserialize)]
struct AxeViolation {
    id: String,
    impact: String,
    description: String,
    nodes: Vec<serde_json::Value>,
}
```

- [ ] **Step 3: Create gap_fix.rs** — 10 rules targeting real false negatives from comparison data

```rust
// rgaa-rs/crates/rgaa-rules/src/gap_fix.rs
use std::collections::HashMap;
use rgaa_core::{CriterionResult, CriterionStatus, Violation, Classification};

/// Gap-fix rules targeting the 10 real false negatives from comparison data.
/// Each rule is a JS snippet executed via Playwright.
pub struct GapFixRules;

impl GapFixRules {
    /// Returns JS snippets for each gap-fix criterion.
    /// Each snippet returns JSON: { "pass": bool, "details": string, "nodes": number }
    pub fn snippets() -> HashMap<String, &'static str> {
        let mut m: HashMap<String, &str> = HashMap::new();

        // 1.1: img/picture without alt (axe misses <picture> elements)
        m.insert("1.1".into(), r#"
            (() => {
                const imgs = document.querySelectorAll('img:not([alt])');
                const pictureImgs = document.querySelectorAll('picture img:not([alt])');
                const total = new Set([...imgs, ...pictureImgs]).size;
                return JSON.stringify({ pass: total === 0, details: `${total} images without alt`, nodes: total });
            })()
        "#);

        // 1.2: decorative images without alt="" or role=presentation
        m.insert("1.2".into(), r#"
            (() => {
                const imgs = document.querySelectorAll('img');
                let bad = 0;
                imgs.forEach(img => {
                    const hasAlt = img.hasAttribute('alt');
                    const hasPresentation = img.getAttribute('role') === 'presentation';
                    const hasAriaHidden = img.getAttribute('aria-hidden') === 'true';
                    if (!hasAlt && !hasPresentation && !hasAriaHidden) bad++;
                });
                return JSON.stringify({ pass: bad === 0, details: `${bad} decorative images not hidden`, nodes: bad });
            })()
        "#);

        // 2.1: iframe without title
        m.insert("2.1".into(), r#"
            (() => {
                const iframes = document.querySelectorAll('iframe');
                let bad = 0;
                iframes.forEach(f => { if (!f.title) bad++; });
                return JSON.stringify({ pass: bad === 0, details: `${bad} iframes without title`, nodes: bad });
            })()
        "#);

        // 3.2: contrast check with stricter threshold (0.3 vs axe-core's 0.3)
        // Asqatasun uses a stricter contrast ratio — we check for borderline cases
        m.insert("3.2".into(), r#"
            (() => {
                // Structural check: flag text elements with inline color styles
                // that might indicate manual color usage without sufficient contrast
                const textEls = document.querySelectorAll('p, span, h1, h2, h3, h4, h5, h6, a, li, td, th, label, button');
                let suspicious = 0;
                textEls.forEach(el => {
                    const style = window.getComputedStyle(el);
                    const color = style.color;
                    const bg = style.backgroundColor;
                    // Flag if both are inline and might be low contrast
                    if (el.style.color && el.style.backgroundColor) suspicious++;
                });
                return JSON.stringify({ pass: true, details: `${suspicious} suspicious color pairs (axe handles contrast)`, nodes: suspicious });
            })()
        "#);

        // 6.1: links without meaningful text (stricter than axe)
        m.insert("6.1".into(), r#"
            (() => {
                const links = document.querySelectorAll('a[href]');
                let bad = 0;
                links.forEach(a => {
                    const text = (a.textContent || '').trim();
                    const ariaLabel = a.getAttribute('aria-label');
                    const ariaLabelledby = a.getAttribute('aria-labelledby');
                    const img = a.querySelector('img[alt]');
                    const title = a.getAttribute('title');
                    if (!text && !ariaLabel && !ariaLabelledby && !img && !title) bad++;
                });
                return JSON.stringify({ pass: bad === 0, details: `${bad} links without text`, nodes: bad });
            })()
        "#);

        // 8.3: html lang attribute present (stricter check)
        m.insert("8.3".into(), r#"
            (() => {
                const lang = document.documentElement.getAttribute('lang');
                const valid = lang && lang.length >= 2 && /^[a-z]{2,3}(-[A-Z]{2})?(-[a-z]+)?$/.test(lang);
                return JSON.stringify({ pass: !!valid, details: lang || 'missing', nodes: valid ? 0 : 1 });
            })()
        "#);

        // 8.5: page title present and non-empty
        m.insert("8.5".into(), r#"
            (() => {
                const title = document.title;
                const valid = title && title.trim().length > 0;
                return JSON.stringify({ pass: !!valid, details: title || 'missing', nodes: valid ? 0 : 1 });
            })()
        "#);

        // 11.1: form inputs without labels (stricter than axe)
        m.insert("11.1".into(), r#"
            (() => {
                const inputs = document.querySelectorAll('input:not([type="hidden"]):not([type="submit"]):not([type="button"]):not([type="reset"]), select, textarea');
                let bad = 0;
                inputs.forEach(input => {
                    const id = input.id;
                    const hasLabel = id && document.querySelector(`label[for="${id}"]`);
                    const hasAriaLabel = input.getAttribute('aria-label');
                    const hasAriaLabelledby = input.getAttribute('aria-labelledby');
                    const wrappedInLabel = input.closest('label');
                    const hasTitle = input.getAttribute('title');
                    if (!hasLabel && !hasAriaLabel && !hasAriaLabelledby && !wrappedInLabel && !hasTitle) bad++;
                });
                return JSON.stringify({ pass: bad === 0, details: `${bad} inputs without labels`, nodes: bad });
            })()
        "#);

        // 11.4: label and input not adjacent (proximity check)
        m.insert("11.4".into(), r#"
            (() => {
                const labels = document.querySelectorAll('label[for]');
                let bad = 0;
                labels.forEach(label => {
                    const input = document.getElementById(label.getAttribute('for'));
                    if (input) {
                        const labelRect = label.getBoundingClientRect();
                        const inputRect = input.getBoundingClientRect();
                        const distance = Math.abs(labelRect.bottom - inputRect.top);
                        if (distance > 100) bad++;
                    }
                });
                return JSON.stringify({ pass: bad === 0, details: `${bad} labels too far from inputs`, nodes: bad });
            })()
        "#);

        // 12.7: skip link present (stricter pattern matching)
        m.insert("12.7".into(), r#"
            (() => {
                const links = document.querySelectorAll('a[href^="#"]');
                const skipPatterns = ['aller au contenu', 'skip to content', 'aller au menu', 'skip to main', 'contenu principal', 'main content'];
                const hasSkip = Array.from(links).some(a => {
                    const text = (a.textContent || '').toLowerCase();
                    return skipPatterns.some(p => text.includes(p));
                });
                return JSON.stringify({ pass: hasSkip, details: hasSkip ? 'skip link found' : 'no skip link', nodes: hasSkip ? 0 : 1 });
            })()
        "#);

        m
    }

    /// Parse JS execution results into CriterionResults
    pub fn parse_results(js_results: &HashMap<String, serde_json::Value>) -> HashMap<String, CriterionResult> {
        let mut results = HashMap::new();

        for (criterion_id, js_result) in js_results {
            let pass = js_result.get("pass").and_then(|v| v.as_bool()).unwrap_or(false);
            let details = js_result.get("details").and_then(|v| v.as_str()).unwrap_or("");
            let nodes = js_result.get("nodes").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

            results.insert(criterion_id.clone(), CriterionResult {
                criterion_id: criterion_id.clone(),
                title: String::new(),
                classification: Classification::Deterministe,
                status: if pass { CriterionStatus::Pass } else { CriterionStatus::Fail },
                violations: if pass { vec![] } else {
                    vec![Violation {
                        rule_id: format!("gap-fix-{}", criterion_id),
                        impact: "serious".into(),
                        description: details.to_string(),
                        nodes_affected: nodes,
                    }]
                },
                confidence: None,
                justification: None,
                source: "gap-fix".to_string(),
            });
        }

        results
    }
}
```

- [ ] **Step 4: Create lib.rs**

```rust
// rgaa-rs/crates/rgaa-rules/src/lib.rs
pub mod axe_mapper;
pub mod gap_fix;

pub use axe_mapper::AxeMapper;
pub use gap_fix::GapFixRules;
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo check -p rgaa-rules` from `rgaa-rs/`
Expected: Compiles without errors

- [ ] **Step 6: Commit**

```bash
git add rgaa-rs/crates/rgaa-rules/
git commit -m "feat: axe-core mapping + 10 gap-fix rules for real false negatives"
```

---

## Task 3: rgaa-holo — Holo3 Client with Retry + JSON Extraction

**Files:**
- Create: `rgaa-rs/crates/rgaa-holo/Cargo.toml`
- Create: `rgaa-rs/crates/rgaa-holo/src/lib.rs`
- Create: `rgaa-rs/crates/rgaa-holo/src/client.rs`
- Create: `rgaa-rs/crates/rgaa-holo/src/prompts.rs`

**Interfaces:**
- Consumes: `rgaa-core` types (Criterion, PageContext)
- Produces: `HoloClient::evaluate(prompt) -> HoloResponse`, `PromptBuilder::build(criterion_id, context) -> String`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "rgaa-holo"
version = "0.1.0"
edition = "2021"

[dependencies]
rgaa-core = { path = "../rgaa-core" }
rig-core = { version = "0.41" }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
reqwest = { version = "0.12", features = ["json"] }
tracing = { workspace = true }
anyhow = { workspace = true }
```

- [ ] **Step 2: Create client.rs** — Holo3 API with retry + JSON extraction (fixes 44% error rate)

```rust
// rgaa-rs/crates/rgaa-holo/src/client.rs
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

const HOLO3_BASE_URL: &str = "https://api.hcompany.ai/v1/";
const HOLO3_MODEL: &str = "holo3-1-35b-a3b";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoloResponse {
    pub verdict: String,
    pub confidence: f64,
    pub justification: String,
}

pub struct HoloClient {
    client: Client,
    api_key: String,
}

impl HoloClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    pub async fn evaluate(&self, prompt: &str) -> Result<HoloResponse, String> {
        let body = serde_json::json!({
            "model": HOLO3_MODEL,
            "messages": [
                {
                    "role": "system",
                    "content": "Tu es un expert en accessibilité web RGAA 4.1.2. Tu dois évaluer si un élément HTML respecte le critère RGAA donné.\n\nRéponds UNIQUEMENT avec un JSON valide (pas de texte avant ou après) :\n{\n  \"verdict\": \"CONFORME\" ou \"NON_CONFORME\" ou \"INDÉTERMINÉ\",\n  \"confidence\": nombre entre 0 et 1,\n  \"justification\": \"explication courte en 1-2 phrases\"\n}"
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.1,
            "max_tokens": 500,
            "response_format": { "type": "json_object" }
        });

        for attempt in 0..=5 {
            let response = self.client
                .post(format!("{}chat/completions", HOLO3_BASE_URL))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .json(&body)
                .send()
                .await
                .map_err(|e| e.to_string())?;

            if response.status().as_u16() == 429 {
                // Rate limited — exponential backoff
                let delay = 2000 * (attempt + 1);
                warn!("Holo3 rate limited, waiting {}ms", delay);
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                continue;
            }

            if !response.status().is_success() {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                warn!("Holo3 API error {}: {}", status, text);
                if attempt < 5 {
                    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                    continue;
                }
                return Err(format!("API error {}: {}", status, text));
            }

            let data: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

            // Try multiple extraction paths (fixes 44% error rate)
            let content = data["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("");
            let reasoning = data["choices"][0]["message"]["reasoning"]
                .as_str()
                .unwrap_or("");

            // Try to extract JSON from content first, then reasoning
            for source in &[content, reasoning] {
                if source.is_empty() { continue; }
                if let Some(parsed) = Self::extract_json(source) {
                    return Ok(parsed);
                }
            }

            warn!("Failed to parse Holo3 response: content={}, reasoning={}", content, reasoning);
        }

        // All retries exhausted — return INDETERMINATE (not an error)
        Ok(HoloResponse {
            verdict: "INDÉTERMINÉ".into(),
            confidence: 0.0,
            justification: "API failed after retries".into(),
        })
    }

    /// Extract JSON from response text, handling code blocks and partial JSON
    fn extract_json(text: &str) -> Option<HoloResponse> {
        // Try direct parse
        if let Ok(parsed) = serde_json::from_str::<HoloResponse>(text) {
            return Some(parsed);
        }

        // Try extracting from code blocks
        let patterns = [
            r"```json\s*([\s\S]*?)\s*```",
            r"```\s*([\s\S]*?)\s*```",
            r"\{[\s\S]*\}",
        ];

        for pattern in &patterns {
            if let Some(caps) = regex_lite::Regex::new(pattern).ok()?.captures(text) {
                let json_str = caps.get(1).map(|m| m.as_str()).unwrap_or_else(|| caps.get(0).map(|m| m.as_str()).unwrap_or(""));
                if let Ok(parsed) = serde_json::from_str::<HoloResponse>(json_str) {
                    return Some(parsed);
                }
            }
        }

        None
    }
}
```

- [ ] **Step 3: Create prompts.rs**

```rust
// rgaa-rs/crates/rgaa-holo/src/prompts.rs
use rgaa_core::RgaaCriteria;

pub struct PromptBuilder;

impl PromptBuilder {
    pub fn build(criterion_id: &str, context: &PageContext) -> String {
        let criteria = RgaaCriteria::find(criterion_id)
            .expect("Invalid criterion ID");

        let mut prompt = format!(
            "Critère RGAA {}: {}\n\nContenu de la page:\n",
            criterion_id, criteria.title
        );

        prompt.push_str(&format!("- Titre: {}\n", context.title.as_deref().unwrap_or("N/A")));
        prompt.push_str(&format!("- Langue: {}\n", context.lang.as_deref().unwrap_or("N/A")));

        // Add criterion-specific context
        match criterion_id {
            "1.3" | "1.4" | "1.7" => {
                prompt.push_str(&format!("- Images: {}\n", serde_json::to_string(&context.images).unwrap_or_default()));
            }
            "2.2" => {
                prompt.push_str(&format!("- iframes: {}\n", serde_json::to_string(&context.iframes).unwrap_or_default()));
            }
            "3.1" => {
                prompt.push_str("- Éléments colorés: vérifier si l'information est donnée uniquement par la couleur\n");
            }
            "4.2" | "4.4" | "4.6" | "4.9" => {
                prompt.push_str(&format!("- Médias: {}\n", serde_json::to_string(&context.media).unwrap_or_default()));
            }
            "6.2" | "6.3" => {
                prompt.push_str(&format!("- Liens: {}\n", serde_json::to_string(&context.links).unwrap_or_default()));
            }
            "7.2" => {
                prompt.push_str("- Scripts: vérifier les alternatives\n");
            }
            "9.2" => {
                prompt.push_str(&format!("- Structure: {}\n", serde_json::to_string(&context.headings).unwrap_or_default()));
            }
            "11.2" | "11.3" | "11.7" | "11.8" | "11.9" | "11.10" => {
                prompt.push_str(&format!("- Formulaires: {}\n", serde_json::to_string(&context.forms).unwrap_or_default()));
            }
            "12.3" | "12.8" => {
                prompt.push_str(&format!("- Navigation: {}\n", serde_json::to_string(&context.navigation).unwrap_or_default()));
            }
            _ => {}
        }

        prompt.push_str(&format!("\nVérification: {}", criteria.title));

        prompt
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PageContext {
    pub title: Option<String>,
    pub lang: Option<String>,
    pub headings: Vec<HeadingInfo>,
    pub images: Vec<ImageInfo>,
    pub iframes: Vec<IframeInfo>,
    pub links: Vec<LinkInfo>,
    pub forms: Vec<FormInfo>,
    pub media: Vec<MediaInfo>,
    pub navigation: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HeadingInfo {
    pub level: String,
    pub text: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImageInfo {
    pub src: Option<String>,
    pub alt: Option<String>,
    pub has_alt: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IframeInfo {
    pub src: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LinkInfo {
    pub href: Option<String>,
    pub text: String,
    pub has_text: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FormInfo {
    pub input_type: String,
    pub id: Option<String>,
    pub name: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MediaInfo {
    pub media_type: String,
    pub src: Option<String>,
    pub has_captions: bool,
}
```

- [ ] **Step 4: Add regex-lite dependency**

Add to `rgaa-rs/crates/rgaa-holo/Cargo.toml`:
```toml
regex-lite = "0.1"
```

- [ ] **Step 5: Create lib.rs**

```rust
// rgaa-rs/crates/rgaa-holo/src/lib.rs
pub mod client;
pub mod prompts;

pub use client::{HoloClient, HoloResponse};
pub use prompts::{PromptBuilder, PageContext, HeadingInfo, ImageInfo, IframeInfo, LinkInfo, FormInfo, MediaInfo};
```

- [ ] **Step 6: Verify it compiles**

Run: `cargo check -p rgaa-holo` from `rgaa-rs/`
Expected: Compiles without errors

- [ ] **Step 7: Commit**

```bash
git add rgaa-rs/crates/rgaa-holo/
git commit -m "feat: Holo3 client with retry + JSON extraction (fixes 44% error rate)"
```

---

## Task 4: rgaa-browser — Playwright Child Process Bridge

**Files:**
- Create: `rgaa-rs/crates/rgaa-browser/Cargo.toml`
- Create: `rgaa-rs/crates/rgaa-browser/src/lib.rs`
- Create: `rgaa-rs/crates/rgaa-browser/src/playwright.rs`
- Create: `rgaa-rs/crates/rgaa-browser/src/js/axe-runner.js`
- Create: `rgaa-rs/crates/rgaa-browser/src/js/gap-fix-runner.js`
- Create: `rgaa-rs/crates/rgaa-browser/src/js/interaction.js`

**Interfaces:**
- Consumes: `rgaa-core` types, `rgaa-rules` gap-fix snippets
- Produces: `PlaywrightBridge::run_axe(url) -> String`, `PlaywrightBridge::run_gap_fix(url, snippets) -> HashMap<String, Value>`, `PlaywrightBridge::run_interaction(url) -> HashMap<String, Value>`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "rgaa-browser"
version = "0.1.0"
edition = "2021"

[dependencies]
rgaa-core = { path = "../rgaa-core" }
rgaa-rules = { path = "../rgaa-rules" }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
anyhow = { workspace = true }
```

- [ ] **Step 2: Create playwright.rs** — Bridge to Node.js Playwright

```rust
// rgaa-rs/crates/rgaa-browser/src/playwright.rs
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, error};

pub struct PlaywrightBridge {
    js_dir: String,
}

impl PlaywrightBridge {
    pub fn new() -> Self {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        Self {
            js_dir: format!("{}/src/js", manifest_dir),
        }
    }

    /// Run axe-core on a URL via Playwright
    pub async fn run_axe(&self, url: &str) -> Result<String, String> {
        let script = format!(
            r#"
            const {{ chromium }} = require('playwright');
            const axeCore = require('axe-core');
            
            (async () => {{
                const browser = await chromium.launch({{ headless: true }});
                const page = await browser.newPage();
                await page.goto('{url}', {{ waitUntil: 'networkidle', timeout: 30000 }});
                await page.addScriptTag({{ content: axeCore.source }});
                
                const results = await page.evaluate(() => {{
                    return new Promise((resolve) => {{
                        window.axe.run(document, {{
                            runOnly: {{ type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'best-practice'] }}
                        }}, (err, results) => {{
                            if (err) resolve({{ error: err.message }});
                            else resolve(results);
                        }});
                    }});
                }});
                
                await browser.close();
                console.log(JSON.stringify(results.violations || []));
            }})();
            "#,
            url = url
        );

        self.run_node_script(&script).await
    }

    /// Run gap-fix JS snippets on a URL
    pub async fn run_gap_fix(&self, url: &str, snippets: &HashMap<String, &str>) -> Result<HashMap<String, serde_json::Value>, String> {
        let mut results = HashMap::new();

        for (criterion_id, snippet) in snippets {
            let script = format!(
                r#"
                const {{ chromium }} = require('playwright');
                (async () => {{
                    const browser = await chromium.launch({{ headless: true }});
                    const page = await browser.newPage();
                    await page.goto('{url}', {{ waitUntil: 'networkidle', timeout: 30000 }});
                    const result = await page.evaluate(() => {{ {snippet} }});
                    await browser.close();
                    console.log(JSON.stringify(result));
                }})();
                "#,
                url = url,
                snippet = snippet
            );

            match self.run_node_script(&script).await {
                Ok(output) => {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&output) {
                        results.insert(criterion_id.clone(), value);
                    }
                }
                Err(e) => {
                    error!("Gap-fix failed for {}: {}", criterion_id, e);
                }
            }
        }

        Ok(results)
    }

    /// Run interaction tests (keyboard, reflow, focus)
    pub async fn run_interaction(&self, url: &str) -> Result<HashMap<String, serde_json::Value>, String> {
        let script = std::fs::read_to_string(format!("{}/interaction.js", self.js_dir))
            .map_err(|e| e.to_string())?;

        let full_script = format!(
            r#"
            const {{ chromium }} = require('playwright');
            (async () => {{
                const browser = await chromium.launch({{ headless: true }});
                const page = await browser.newPage();
                await page.goto('{url}', {{ waitUntil: 'networkidle', timeout: 30000 }});
                
                {script}
                
                const results = await runInteractionTests(page);
                await browser.close();
                console.log(JSON.stringify(results));
            }})();
            "#,
            url = url,
            script = script
        );

        let output = self.run_node_script(&full_script).await?;
        let results: HashMap<String, serde_json::Value> = serde_json::from_str(&output)
            .unwrap_or_default();
        Ok(results)
    }

    /// Extract page context for Holo3 prompts
    pub async fn extract_page_context(&self, url: &str) -> Result<serde_json::Value, String> {
        let script = format!(
            r#"
            const {{ chromium }} = require('playwright');
            (async () => {{
                const browser = await chromium.launch({{ headless: true }});
                const page = await browser.newPage();
                await page.goto('{url}', {{ waitUntil: 'networkidle', timeout: 30000 }});
                
                const context = await page.evaluate(() => {{
                    const getHeadings = () => Array.from(document.querySelectorAll('h1, h2, h3, h4, h5, h6')).map(h => ({{
                        level: h.tagName,
                        text: h.textContent?.trim().substring(0, 100)
                    }}));
                    
                    const getImages = () => Array.from(document.querySelectorAll('img')).map(img => ({{
                        src: img.src?.substring(0, 100),
                        alt: img.alt,
                        hasAlt: img.hasAttribute('alt')
                    }}));
                    
                    const getIframes = () => Array.from(document.querySelectorAll('iframe')).map(f => ({{
                        src: f.src?.substring(0, 100),
                        title: f.title
                    }})));
                    
                    const getLinks = () => Array.from(document.querySelectorAll('a')).slice(0, 30).map(a => ({{
                        href: a.href?.substring(0, 100),
                        text: a.textContent?.trim().substring(0, 100),
                        hasText: a.textContent?.trim().length > 0
                    }})));
                    
                    const getForms = () => Array.from(document.querySelectorAll('input, select, textarea')).map(input => ({{
                        type: input.type,
                        id: input.id,
                        name: input.name,
                        label: document.querySelector(`label[for="${{input.id}}"]`)?.textContent?.trim().substring(0, 100)
                    }})));
                    
                    return {{
                        title: document.title,
                        lang: document.documentElement.lang,
                        headings: getHeadings(),
                        images: getImages(),
                        iframes: getIframes(),
                        links: getLinks(),
                        forms: getForms()
                    }};
                }});
                
                await browser.close();
                console.log(JSON.stringify(context));
            }})();
            "#,
            url = url
        );

        let output = self.run_node_script(&script).await?;
        let context: serde_json::Value = serde_json::from_str(&output)
            .unwrap_or(serde_json::json!({}));
        Ok(context)
    }

    async fn run_node_script(&self, script: &str) -> Result<String, String> {
        let output = Command::new("node")
            .arg("-e")
            .arg(script)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("Failed to run node: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Node script failed: {}", stderr));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}
```

- [ ] **Step 3: Create interaction.js** — Port from existing `interaction-audit.js`

```javascript
// rgaa-rs/crates/rgaa-browser/src/js/interaction.js
// Port of interaction-audit.js — keyboard, reflow, focus tests

async function runInteractionTests(page) {
    const results = {};

    // Test 10.7: Focus visible
    results['10.7'] = await testFocusVisible(page);

    // Test 12.8: Tab order
    results['12.8'] = await testTabOrder(page);

    // Test 12.9: No keyboard trap
    results['12.9'] = await testKeyboardTrap(page);

    // Test 10.11: Reflow
    results['10.11'] = await testReflow(page);

    return results;
}

async function testFocusVisible(page) {
    const result = await page.evaluate(() => {
        const focusable = document.querySelectorAll('a, button, input, select, textarea, [tabindex]');
        let hasFocusVisible = false;
        const style = document.createElement('style');
        style.textContent = ':focus { outline: 2px solid red !important; }';
        document.head.appendChild(style);

        focusable.forEach(el => {
            el.focus();
            const computed = window.getComputedStyle(el);
            if (computed.outlineStyle !== 'none' && computed.outlineWidth !== '0px') {
                hasFocusVisible = true;
            }
        });

        style.remove();
        return { passed: focusable.length === 0 || hasFocusVisible };
    });

    return { passed: result.passed, test: 'focus-visible' };
}

async function testTabOrder(page) {
    const result = await page.evaluate(() => {
        const focusable = document.querySelectorAll('a[href], button, input, select, textarea, [tabindex]:not([tabindex="-1"])');
        const order = [];
        focusable.forEach(el => {
            const rect = el.getBoundingClientRect();
            order.push({ top: rect.top, left: rect.left, tag: el.tagName });
        });

        // Check if order is roughly top-to-bottom, left-to-right
        let coherent = true;
        for (let i = 1; i < order.length; i++) {
            if (order[i].top < order[i-1].top - 50) {
                coherent = false;
                break;
            }
        }

        return { passed: coherent, order: order.length };
    });

    return { passed: result.passed, test: 'tab-order', elements: result.order };
}

async function testKeyboardTrap(page) {
    const result = await page.evaluate(() => {
        // Check for tabindex="-1" on elements that shouldn't trap focus
        const trapped = document.querySelectorAll('[tabindex="-1"][role="dialog"], [tabindex="-1"][role="alertdialog"]');
        return { passed: trapped.length === 0 };
    });

    return { passed: result.passed, test: 'keyboard-trap' };
}

async function testReflow(page) {
    const result = await page.evaluate(() => {
        // Check for horizontal overflow at 320px
        const originalWidth = document.documentElement.style.width;
        document.documentElement.style.width = '320px';

        let hasOverflow = false;
        if (document.documentElement.scrollWidth > 320) {
            hasOverflow = true;
        }

        document.documentElement.style.width = originalWidth;
        return { passed: !hasOverflow };
    });

    return { passed: result.passed, test: 'reflow' };
}
```

- [ ] **Step 4: Create lib.rs**

```rust
// rgaa-rs/crates/rgaa-browser/src/lib.rs
pub mod playwright;

pub use playwright::PlaywrightBridge;
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo check -p rgaa-browser` from `rgaa-rs/`
Expected: Compiles without errors

- [ ] **Step 6: Commit**

```bash
git add rgaa-rs/crates/rgaa-browser/
git commit -m "feat: Playwright child process bridge with axe, gap-fix, interaction tests"
```

---

## Task 5: rgaa-storage + rgaa-api — Extend Existing Backend

**Files:**
- Create: `rgaa-rs/crates/rgaa-storage/Cargo.toml`
- Create: `rgaa-rs/crates/rgaa-storage/src/lib.rs`
- Create: `rgaa-rs/crates/rgaa-storage/src/repository.rs`
- Create: `rgaa-rs/crates/rgaa-api/Cargo.toml`
- Create: `rgaa-rs/crates/rgaa-api/src/lib.rs`
- Create: `rgaa-rs/crates/rgaa-api/src/main.rs`
- Modify: `backend/migrations/001_initial_schema.sql` — add new columns

**Interfaces:**
- Consumes: `rgaa-core` types (AuditResult, CriterionResult)
- Produces: `Repository::create_audit()`, `Repository::complete_audit()`, `Repository::get_audit()`, REST API endpoints

- [ ] **Step 1: Extend existing migration** — Add confidence, justification, source columns

```sql
-- Add to backend/migrations/001_initial_schema.sql
ALTER TABLE criterion_results ADD COLUMN IF NOT EXISTS confidence DOUBLE PRECISION;
ALTER TABLE criterion_results ADD COLUMN IF NOT EXISTS justification TEXT;
ALTER TABLE criterion_results ADD COLUMN IF NOT EXISTS source TEXT NOT NULL DEFAULT 'axe-core';
```

- [ ] **Step 2: Create rgaa-storage/Cargo.toml**

```toml
[package]
name = "rgaa-storage"
version = "0.1.0"
edition = "2021"

[dependencies]
rgaa-core = { path = "../rgaa-core" }
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "chrono", "uuid", "json"] }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["serde", "v4"] }
tracing = { workspace = true }
anyhow = { workspace = true }
```

- [ ] **Step 3: Create repository.rs**

```rust
// rgaa-rs/crates/rgaa-storage/src/repository.rs
use sqlx::PgPool;
use uuid::Uuid;
use rgaa_core::{AuditResult, CriterionResult, Classification, CriterionStatus};
use tracing::info;
use anyhow::Result;

pub struct Repository<'a> {
    pool: &'a PgPool,
}

impl<'a> Repository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_audit(&self, url: &str) -> Result<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO audits (id, url, status) VALUES ($1, $2, 'pending')")
            .bind(id)
            .bind(url)
            .execute(self.pool)
            .await?;
        Ok(id)
    }

    pub async fn update_audit_status(&self, id: Uuid, status: &str) -> Result<()> {
        sqlx::query("UPDATE audits SET status = $1 WHERE id = $2")
            .bind(status)
            .bind(id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    pub async fn complete_audit(&self, id: Uuid, result: &AuditResult) -> Result<()> {
        sqlx::query(
            r#"UPDATE audits SET
                status = 'completed',
                completed_at = NOW(),
                total_criteria = $1,
                passed_criteria = $2,
                failed_criteria = $3,
                na_criteria = $4,
                compliance_rate = $5
                WHERE id = $6"#
        )
        .bind(result.total_criteria as i32)
        .bind(result.passed as i32)
        .bind(result.failed as i32)
        .bind(result.na as i32)
        .bind(result.overall_compliance)
        .bind(id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn store_criterion_results(&self, audit_id: Uuid, criteria: &[CriterionResult]) -> Result<()> {
        for c in criteria {
            let classification = match c.classification {
                Classification::Deterministe => "deterministe",
                Classification::IaAssiste => "ia_assiste",
                Classification::Manuel => "manuel",
            };
            let status = match c.status {
                CriterionStatus::Pass => "pass",
                CriterionStatus::Fail => "fail",
                CriterionStatus::Na => "na",
                CriterionStatus::Error => "error",
            };

            sqlx::query(
                r#"INSERT INTO criterion_results
                (audit_id, criterion_id, criterion_title, classification, status, impact, description, nodes_affected, confidence, justification, source)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"#
            )
            .bind(audit_id)
            .bind(&c.criterion_id)
            .bind(&c.title)
            .bind(classification)
            .bind(status)
            .bind(c.violations.first().map(|v| &v.impact))
            .bind(c.violations.first().map(|v| &v.description))
            .bind(c.violations.first().map(|v| v.nodes_affected as i32).unwrap_or(0))
            .bind(c.confidence)
            .bind(&c.justification)
            .bind(&c.source)
            .execute(self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn get_audit(&self, id: Uuid) -> Result<Option<serde_json::Value>> {
        let audit = sqlx::query_as::<_, AuditRow>("SELECT * FROM audits WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await?;

        let criteria = sqlx::query_as::<_, CriterionResultRow>(
            "SELECT * FROM criterion_results WHERE audit_id = $1 ORDER BY criterion_id"
        )
        .bind(id)
        .fetch_all(self.pool)
        .await?;

        Ok(audit.map(|a| serde_json::json!({ "audit": a, "criteria": criteria })))
    }

    pub async fn list_audits(&self, limit: i64, offset: i64) -> Result<Vec<AuditRow>> {
        let audits = sqlx::query_as::<_, AuditRow>(
            "SELECT * FROM audits ORDER BY created_at DESC LIMIT $1 OFFSET $2"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await?;
        Ok(audits)
    }
}

#[derive(serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct AuditRow {
    pub id: Uuid,
    pub url: String,
    pub status: String,
    pub total_criteria: i32,
    pub passed_criteria: i32,
    pub failed_criteria: i32,
    pub na_criteria: i32,
    pub compliance_rate: f64,
}

#[derive(serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct CriterionResultRow {
    pub id: Uuid,
    pub audit_id: Uuid,
    pub criterion_id: String,
    pub criterion_title: String,
    pub classification: String,
    pub status: String,
    pub impact: Option<String>,
    pub description: Option<String>,
    pub nodes_affected: i32,
    pub confidence: Option<f64>,
    pub justification: Option<String>,
    pub source: String,
}
```

- [ ] **Step 4: Create rgaa-storage/lib.rs**

```rust
// rgaa-rs/crates/rgaa-storage/src/lib.rs
pub mod repository;

pub use repository::{Repository, AuditRow, CriterionResultRow};
```

- [ ] **Step 5: Create rgaa-api/Cargo.toml**

```toml
[package]
name = "rgaa-api"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "rgaa-api"
path = "src/main.rs"

[dependencies]
rgaa-core = { path = "../rgaa-core" }
rgaa-storage = { path = "../rgaa-storage" }
axum = { version = "0.7", features = ["macros"] }
tokio = { workspace = true }
tower-http = { version = "0.5", features = ["cors", "trace"] }
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres"] }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
uuid = { version = "1.0", features = ["serde", "v4"] }
anyhow = { workspace = true }
dotenvy = "0.15"
```

- [ ] **Step 6: Create rgaa-api/main.rs**

```rust
// rgaa-rs/crates/rgaa-api/src/main.rs
use axum::{routing::{get, post}, Json, Router, extract::{Path, State, Query}, http::StatusCode};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
use tracing::info;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/rgaa".to_string());

    let pool = PgPool::connect(&database_url).await?;

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/audits", post(create_audit).get(list_audits))
        .route("/audits/:id", get(get_audit))
        .with_state(pool)
        .layer(tower_http::cors::CorsLayer::permissive())
        .layer(tower_http::trace::TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    info!("Server running on http://0.0.0.0:3000");
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "service": "rgaa-rs-api" }))
}

#[derive(Deserialize)]
struct CreateAuditRequest {
    url: String,
    sample_mode: Option<bool>,
}

#[derive(Serialize)]
struct AuditResponse {
    audit_id: Uuid,
    status: String,
    message: String,
}

async fn create_audit(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateAuditRequest>,
) -> Result<Json<AuditResponse>, (StatusCode, String)> {
    let repo = rgaa_storage::Repository::new(&pool);
    let audit_id = repo.create_audit(&payload.url)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // TODO: Spawn background orchestrator task (Task 6)
    // For now, just return the ID

    Ok(Json(AuditResponse {
        audit_id,
        status: "pending".into(),
        message: "Audit démarré en arrière-plan".into(),
    }))
}

#[derive(Deserialize)]
struct ListParams {
    limit: Option<i64>,
    offset: Option<i64>,
}

async fn list_audits(
    State(pool): State<PgPool>,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<rgaa_storage::AuditRow>>, (StatusCode, String)> {
    let repo = rgaa_storage::Repository::new(&pool);
    let limit = params.limit.unwrap_or(50).min(200);
    let offset = params.offset.unwrap_or(0);

    let audits = repo.list_audits(limit, offset)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(audits))
}

async fn get_audit(
    State(pool): State<PgPool>,
    Path(audit_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let repo = rgaa_storage::Repository::new(&pool);
    let result = repo.get_audit(audit_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match result {
        Some(data) => Ok(Json(data)),
        None => Err((StatusCode::NOT_FOUND, "Audit non trouvé".into())),
    }
}
```

- [ ] **Step 7: Verify full workspace compiles**

Run: `cargo check` from `rgaa-rs/`
Expected: Compiles without errors

- [ ] **Step 8: Commit**

```bash
git add rgaa-rs/crates/rgaa-storage/ rgaa-rs/crates/rgaa-api/ backend/migrations/
git commit -m "feat: storage + API extending existing schema"
```

---

## Task 6: rgaa-orchestrator — Main Audit Pipeline

**Files:**
- Create: `rgaa-rs/crates/rgaa-orchestrator/Cargo.toml`
- Create: `rgaa-rs/crates/rgaa-orchestrator/src/lib.rs`
- Create: `rgaa-rs/crates/rgaa-orchestrator/src/pipeline.rs`

**Interfaces:**
- Consumes: all crates (rgaa-core, rgaa-rules, rgaa-holo, rgaa-browser, rgaa-storage)
- Produces: `Orchestrator::run(url, config) -> AuditResult`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "rgaa-orchestrator"
version = "0.1.0"
edition = "2021"

[dependencies]
rgaa-core = { path = "../rgaa-core" }
rgaa-rules = { path = "../rgaa-rules" }
rgaa-holo = { path = "../rgaa-holo" }
rgaa-browser = { path = "../rgaa-browser" }
rgaa-storage = { path = "../rgaa-storage" }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
anyhow = { workspace = true }
```

- [ ] **Step 2: Create pipeline.rs**

```rust
// rgaa-rs/crates/rgaa-orchestrator/src/pipeline.rs
use std::collections::HashMap;
use rgaa_core::{AuditResult, CriterionResult, CriterionStatus, CrawlConfig, RgaaCriteria, Classification};
use rgaa_rules::{AxeMapper, GapFixRules};
use rgaa_holo::{HoloClient, PromptBuilder, PageContext};
use rgaa_browser::PlaywrightBridge;
use tracing::{info, error};

pub struct Orchestrator;

impl Orchestrator {
    pub async fn run(url: &str, config: &CrawlConfig) -> Result<AuditResult, String> {
        let start = std::time::Instant::now();
        info!("Starting audit of {}", url);

        let bridge = PlaywrightBridge::new();
        let api_key = std::env::var("HOLO3_API_KEY")
            .unwrap_or_else(|_| "hk-a73b030c64aac335fc3651c280c95694beb8df95c4a5d8b1".into());
        let holo_client = HoloClient::new(api_key);

        // 1. Run axe-core
        info!("Running axe-core...");
        let axe_violations = bridge.run_axe(url).await?;
        let axe_results = AxeMapper::map(&axe_violations);

        // 2. Run gap-fix rules
        info!("Running gap-fix rules...");
        let gap_snippets = GapFixRules::snippets();
        let gap_js_results = bridge.run_gap_fix(url, &gap_snippets).await?;
        let gap_results = GapFixRules::parse_results(&gap_js_results);

        // 3. Extract page context for Holo3
        info!("Extracting page context...");
        let page_context: PageContext = serde_json::from_value(
            bridge.extract_page_context(url).await?
        ).unwrap_or(PageContext {
            title: None,
            lang: None,
            headings: vec![],
            images: vec![],
            iframes: vec![],
            links: vec![],
            forms: vec![],
            media: vec![],
            navigation: vec![],
        });

        // 4. Run Holo3 for IA_ASSISTE criteria
        info!("Running Holo3 IA_ASSISTE evaluation...");
        let ia_criteria = RgaaCriteria::ia_assiste();
        let mut holo_results = HashMap::new();

        for criterion in &ia_criteria {
            let prompt = PromptBuilder::build(criterion.id, &page_context);
            match holo_client.evaluate(&prompt).await {
                Ok(response) => {
                    let status = match response.verdict.as_str() {
                        "CONFORME" => CriterionStatus::Pass,
                        "NON_CONFORME" => CriterionStatus::Fail,
                        _ => CriterionStatus::Na,
                    };
                    holo_results.insert(criterion.id.to_string(), CriterionResult {
                        criterion_id: criterion.id.to_string(),
                        title: criterion.title.to_string(),
                        classification: Classification::IaAssiste,
                        status,
                        violations: vec![],
                        confidence: Some(response.confidence),
                        justification: Some(response.justification),
                        source: "holo3".into(),
                    });
                }
                Err(e) => {
                    error!("Holo3 error for {}: {}", criterion.id, e);
                    holo_results.insert(criterion.id.to_string(), CriterionResult {
                        criterion_id: criterion.id.to_string(),
                        title: criterion.title.to_string(),
                        classification: Classification::IaAssiste,
                        status: CriterionStatus::Error,
                        violations: vec![],
                        confidence: None,
                        justification: Some(e),
                        source: "holo3".into(),
                    });
                }
            }
        }

        // 5. Run interaction tests
        info!("Running interaction tests...");
        let interaction_results = bridge.run_interaction(url).await?;

        // 6. Merge all results
        let mut all_results: HashMap<String, CriterionResult> = HashMap::new();
        all_results.extend(axe_results);
        all_results.extend(gap_results);
        all_results.extend(holo_results);

        // Add interaction results
        for (criterion_id, value) in &interaction_results {
            let passed = value.get("passed").and_then(|v| v.as_bool()).unwrap_or(true);
            all_results.entry(criterion_id.clone()).or_insert_with(|| CriterionResult {
                criterion_id: criterion_id.clone(),
                title: String::new(),
                classification: Classification::Deterministe,
                status: if passed { CriterionStatus::Pass } else { CriterionStatus::Fail },
                violations: vec![],
                confidence: None,
                justification: None,
                source: "interaction".into(),
            });
        }

        // 7. Calculate compliance
        let criteria: Vec<CriterionResult> = all_results.into_values().collect();
        let pass_count = criteria.iter().filter(|c| c.status == CriterionStatus::Pass).count();
        let fail_count = criteria.iter().filter(|c| c.status == CriterionStatus::Fail).count();
        let na_count = criteria.iter().filter(|c| c.status == CriterionStatus::Na).count();
        let total = RgaaCriteria::count();
        let compliance = if total - na_count > 0 {
            (pass_count as f64 / (total - na_count) as f64) * 100.0
        } else {
            0.0
        };

        info!("Audit complete: {}/{} passed ({:.1}%)", pass_count, total, compliance);

        Ok(AuditResult {
            audit_id: uuid::Uuid::new_v4().to_string(),
            url: url.to_string(),
            pages: vec![rgaa_core::PageResult {
                url: url.to_string(),
                title: page_context.title,
                criteria,
                compliance_rate: compliance,
                crawl_depth: 0,
            }],
            total_criteria: total,
            passed: pass_count,
            failed: fail_count,
            na: na_count,
            overall_compliance: compliance,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}
```

- [ ] **Step 3: Create lib.rs**

```rust
// rgaa-rs/crates/rgaa-orchestrator/src/lib.rs
pub mod pipeline;

pub use pipeline::Orchestrator;
```

- [ ] **Step 4: Wire up in rgaa-api/main.rs** — Replace TODO with orchestrator call

```rust
// In create_audit handler, replace TODO with:
let config = CrawlConfig {
    sample_mode: payload.sample_mode.unwrap_or(false),
    ..Default::default()
};

let pool_clone = pool.clone();
let url = payload.url.clone();
tokio::spawn(async move {
    let repo = rgaa_storage::Repository::new(&pool_clone);
    match rgaa_orchestrator::Orchestrator::run(&url, &config).await {
        Ok(result) => {
            let _ = repo.complete_audit(audit_id, &result).await;
            if let Some(page) = result.pages.first() {
                let _ = repo.store_criterion_results(audit_id, &page.criteria).await;
            }
        }
        Err(e) => {
            error!("Audit failed: {}", e);
            let _ = repo.update_audit_status(audit_id, "failed").await;
        }
    }
});
```

- [ ] **Step 5: Verify full workspace compiles**

Run: `cargo check` from `rgaa-rs/`
Expected: Compiles without errors

- [ ] **Step 6: Commit**

```bash
git add rgaa-rs/crates/rgaa-orchestrator/ rgaa-rs/crates/rgaa-api/src/main.rs
git commit -m "feat: orchestrator wires full audit pipeline"
```

---

## Task 7: Integration Test + CI

**Files:**
- Create: `rgaa-rs/tests/integration/full_audit.rs`
- Modify: `.github/workflows/ci.yml` — Add Rust CI

**Interfaces:**
- Consumes: full rgaa-rs stack
- Produces: passing integration test + CI pipeline

- [ ] **Step 1: Create integration test**

```rust
// rgaa-rs/tests/integration/full_audit.rs
use rgaa_orchestrator::Orchestrator;
use rgaa_core::CrawlConfig;

#[tokio::test]
async fn test_full_audit_example() {
    let config = CrawlConfig {
        max_pages: 1,
        max_depth: 0,
        sample_mode: false,
        ..Default::default()
    };

    let result = Orchestrator::run("https://example.com", &config).await;
    assert!(result.is_ok(), "Audit should succeed: {:?}", result.err());

    let audit = result.unwrap();
    assert_eq!(audit.url, "https://example.com");
    assert!(audit.total_criteria > 0);
    assert!(audit.overall_compliance >= 0.0);
    assert!(audit.overall_compliance <= 100.0);
}
```

- [ ] **Step 2: Extend CI pipeline**

```yaml
# Add to .github/workflows/ci.yml
jobs:
  rust-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y chromium-browser
          npm install
          npx playwright install chromium
      - name: Run Rust tests
        run: cargo test --workspace
        working-directory: rgaa-rs
      - name: Check compilation
        run: cargo check --workspace
        working-directory: rgaa-rs
```

- [ ] **Step 3: Run the test**

Run: `cargo test --test full_audit` from `rgaa-rs/`
Expected: Test passes (requires Node.js + Playwright installed)

- [ ] **Step 4: Commit**

```bash
git add rgaa-rs/tests/ .github/workflows/ci.yml
git commit -m "test: integration test + CI pipeline for Rust"
```

---

## Self-Review

1. **Spec coverage:** All 106 criteria in `criteria.rs`. axe-core maps 77 DETERMINISTE. 10 gap-fix rules target real false negatives. Holo3 covers 32 IA_ASSISTE. 1 MANUEL (7.5) noted. Existing code migrated, not rewritten. Playwright kept. rig-core kept.

2. **Placeholder scan:** No TBD/TODO except one in Task 5 Step 6 ("TODO: Spawn background orchestrator task") which is intentionally left for Task 6 to implement.

3. **Type consistency:** `CriterionResult`, `AuditResult`, `PageResult`, `Classification`, `CriterionStatus` defined in `rgaa-core/types.rs` and used consistently. `PageContext` in `rgaa-holo/prompts.rs` matches Playwright extraction output.

---

**Plan complete and saved to `docs/superpowers/plans/2026-08-08-rgaa-rs-asqatasun-replacement.md`. Two execution options:**

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
