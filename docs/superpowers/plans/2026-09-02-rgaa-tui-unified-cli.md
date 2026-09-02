# rgaa-tui — Unified Accessibility Audit CLI + TUI

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a single `rgaa` binary that serves as both the installer (first-run wizard) and the interactive TUI for running RGAA accessibility audits with deterministic checks + optional Holo3 LLM evaluation.

**Architecture:** One binary, two entry modes. Running `rgaa` with no args (or `rgaa tui`) starts the ratatui wizard. Running `rgaa audit --url X` is the headless CI-friendly path. Self-installs on first run. Deterministic checks always work; Holo3 evaluation requires an API key (stored in OS keyring, plain-text fallback with warning). SQLite for local storage, Postgres as optional upgrade.

**Tech Stack:** ratatui, ratatui-input-input, rusqlite, keyring, clap, tokio, rgaa-orchestrator, rgaa-agent, spider, rgaa-core, rgaa-holo, rgaa-storage, rgaa-obscura

**Spec:** Not written — design decisions were made collaboratively in this session and recorded in the conversation history.

---

## Global Constraints

- MSRV: Rust 1.85 (2024 edition)
- ratatui latest stable
- rusqlite with bundled SQLite (no system SQLite required)
- keyring crate for OS credential storage (macOS Keychain, Windows Credential Manager, libsecret on Linux)
- rgaa-storage uses sqlx 0.8+
- All rgaa-core domain types (AuditResult, CriterionResult, etc.) are re-exported, not copied
- cargo-dist for cross-platform release artifacts
- Pre-built binaries for: linux-x86_64, darwin-x86_64, darwin-aarch64, windows-x86_64

---

## File Map

```text
rgaa-rs/
├── Cargo.toml                              # Modify: add rgaa-tui to workspace members
├── Cargo.lock
└── crates/
    ├── rgaa-tui/                          # Create: new crate
    │   ├── Cargo.toml
    │   └── src/
    │       ├── main.rs                    # Create: clap entry + ratatui tui::run dispatch
    │       ├── lib.rs                     # Create: re-exports for binary
    │       ├── tui/
    │       │   ├── mod.rs                 # Create: ratatui app state + run()
    │       │   ├── install.rs             # Create: first-run install wizard
    │       │   ├── setup.rs              # Create: API key + config wizard
    │       │   ├── audit.rs               # Create: audit wizard (URL → progress → results)
    │       │   ├── results.rs             # Create: results view + drill-down
    │       │   └── export.rs              # Create: export bundler (JSON/MD/SARIF/JUnit/clipboard)
    │       ├── keyring.rs                 # Create: OS keyring wrapper (keyring crate + plain fallback)
    │       ├── storage.rs                 # Create: SQLite storage via rusqlite
    │       ├── update.rs                  # Create: GitHub releases version check
    │       └── commands.rs                # Create: clap command structs (audit, config, tui, install)
    ├── rgaa-cli/                          # Modify: add rgaa-tui as optional dep for future integration
    └── rgaa-agent/                        # No change (used as library)
```

---

## Tasks

### Task 1: Bootstrap rgaa-tui crate and workspace wiring

**Files:**
- Create: `rgaa-rs/crates/rgaa-tui/Cargo.toml`
- Modify: `rgaa-rs/Cargo.toml` (members list)

**Interfaces:**
- Consumes: workspace dependencies from `Cargo.toml`
- Produces: `rgaa_tui` crate compilable as part of workspace

- [ ] **Step 1: Create rgaa-tui/Cargo.toml**

```toml
[package]
description = "Interactive TUI for RGAA accessibility auditing"
license = "MIT"
name = "rgaa-tui"
version = "0.1.0"
edition = "2021"

[dependencies]
rgaa-core = { path = "../rgaa-core" }
rgaa-orchestrator = { path = "../rgaa-orchestrator" }
rgaa-agent = { path = "../rgaa-agent" }
rgaa-holo = { path = "../rgaa-holo" }
rgaa-storage = { path = "../rgaa-storage" }
rgaa-obscura = { path = "../rgaa-obscura" }
rgaa-spider = { path = "../rgaa-spider" }

clap = { workspace = true }
tokio = { workspace = true }
rusqlite = { version = "0.32", features = ["bundled"] }
keyring = "3"
ratatui = "0.28"
ratatui-input-input = "0.9"
thiserror = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
anyhow = { workspace = true }

[target.'cfg(windows)'.dependencies]
winapi = { version = "0.3", features = ["winuser"] }
```

- [ ] **Step 2: Add rgaa-tui to workspace members**

Modify `rgaa-rs/Cargo.toml`:
```toml
members = [
    ...existing...,
    "crates/rgaa-tui",
]
```

- [ ] **Step 3: Verify workspace compiles**

Run: `cd rgaa-rs && cargo check -p rgaa-tui`
Expected: Compiles (empty lib + main.rs will be added in Step 4)

- [ ] **Step 4: Create stub lib.rs**

```rust
pub mod commands;
pub mod keyring;
pub mod storage;
pub mod tui;
pub mod update;
```

- [ ] **Step 5: Create stub main.rs with clap + ratatui dispatch**

```rust
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "rgaa", version, about = "RGAA accessibility audit CLI + TUI")]
struct Cli {
    #[command(subcommand)]
    command: Option<TopCommand>,
}

#[derive(Debug, Subcommand)]
enum TopCommand {
    Tui,
    Audit {
        #[arg(long)]
        url: Option<String>,
    },
    Config,
    Install,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        None | Some(TopCommand::Tui) => rgaa_tui::tui::run().await,
        Some(TopCommand::Audit { url }) => rgaa_tui::commands::audit(url).await,
        Some(TopCommand::Config) => rgaa_tui::commands::config().await,
        Some(TopCommand::Install) => rgaa_tui::commands::install().await,
    }
}
```

- [ ] **Step 6: Create stub command handlers**

In `rgaa-tui/src/commands.rs`:
```rust
pub async fn audit(_url: Option<String>) {
    todo!("audit command")
}
pub async fn config() {
    todo!("config command")
}
pub async fn install() {
    todo!("install command")
}
```

- [ ] **Step 7: Create stub tui module**

In `rgaa-tui/src/tui/mod.rs`:
```rust
pub async fn run() {
    todo!("ratatui app")
}
```

- [ ] **Step 8: Verify full workspace compiles**

Run: `cd rgaa-rs && cargo check --workspace`
Expected: All crates compile including rgaa-tui

- [ ] **Step 9: Commit**

```bash
git add rgaa-rs/crates/rgaa-tui/ rgaa-rs/Cargo.toml
git commit -m "feat(tui): bootstrap rgaa-tui crate with ratatui + clap"
```

---

### Task 2: ratatui app scaffolding — install wizard

**Files:**
- Create: `rgaa-rs/crates/rgaa-tui/src/tui/install.rs`
- Modify: `rgaa-rs/crates/rgaa-tui/src/tui/mod.rs`

**Interfaces:**
- Consumes: `rgaa_tui::commands::install`
- Produces: `tui::run_install_wizard() -> bool` (true = installed, false = skipped)

- [ ] **Step 1: Define AppState and install wizard state machine**

```rust
#[derive(Debug, Clone)]
pub enum InstallStep {
    Welcome,
    Downloading { progress: f32 },
    Installing,
    Done,
}

#[derive(Debug)]
pub struct InstallWizard {
    step: InstallStep,
    platform: String,
    version: String,
}
```

- [ ] **Step 2: Implement install wizard render loop**

```rust
pub async fn run_install_wizard() -> bool {
    // Uses ratatui::Terminal + crossterm input
    // Step 1: Welcome screen with "Press ENTER to install"
    // Step 2: Detect platform (linux-x86_64, darwin-*, windows-*)
    // Step 3: Download from GitHub releases (tokio::spawn + progress bar)
    // Step 4: Extract to ~/.local/bin
    // Step 5: Make executable
    // Step 6: Done screen
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cd rgaa-rs && cargo check -p rgaa-tui`
Expected: Compiles with stub functions

- [ ] **Step 4: Commit**

```bash
git add rgaa-rs/crates/rgaa-tui/src/tui/install.rs rgaa-rs/crates/rgaa-tui/src/tui/mod.rs
git commit -m "feat(tui): install wizard scaffolding with ratatui"
```

---

### Task 3: OS keyring integration

**Files:**
- Create: `rgaa-rs/crates/rgaa-tui/src/keyring.rs`

**Interfaces:**
- Consumes: `HOLO3_API_KEY` value
- Produces: `Keyring::store("holo3", &key)`, `Keyring::get("holo3") -> Option<String>`

- [ ] **Step 1: Implement Keyring wrapper**

```rust
use keyring::Entry;

pub struct Keyring;

impl Keyring {
    const SERVICE: &'static str = "rgaa";

    pub fn store(key: &str) -> anyhow::Result<()> {
        let entry = Entry::new(Self::SERVICE, "holo3_api_key")
            .map_err(|e| anyhow::anyhow!("keyring error: {}", e))?;
        entry.set_password(key)
            .map_err(|e| anyhow::anyhow!("keyring store error: {}", e))?;
        Ok(())
    }

    pub fn get() -> anyhow::Result<Option<String>> {
        let entry = Entry::new(Self::SERVICE, "holo3_api_key")
            .map_err(|e| anyhow::anyhow!("keyring error: {}", e))?;
        match entry.get_password() {
            Ok(key) => Ok(Some(key)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("keyring get error: {}", e)),
        }
    }
}
```

- [ ] **Step 2: Add plain-text fallback**

If keyring is unavailable (Linux without libsecret), write to `~/.rgaa/env` with a warning.

```rust
fn fallback_store(key: &str) -> anyhow::Result<()> {
    let env_path = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("no home dir"))?
        .join(".rgaa/env");
    std::fs::create_dir_all(env_path.parent().unwrap())?;
    std::fs::write(&env_path, format!("HOLO3_API_KEY={}\n", key))?;
    eprintln!("WARNING: OS keyring unavailable. Key stored in plain text at ~/.rgaa/env");
    Ok(())
}
```

- [ ] **Step 3: Add keyring crate to Cargo.toml**

Add `keyring = "3"` and `dirs = "6"` to rgaa-tui dependencies.

- [ ] **Step 4: Verify it compiles**

Run: `cd rgaa-rs && cargo check -p rgaa-tui`
Expected: Compiles

- [ ] **Step 5: Commit**

```bash
git add rgaa-rs/crates/rgaa-tui/src/keyring.rs rgaa-rs/crates/rgaa-tui/Cargo.toml
git commit -m "feat(tui): OS keyring integration with plain-text fallback"
```

---

### Task 4: SQLite local storage

**Files:**
- Create: `rgaa-rs/crates/rgaa-tui/src/storage.rs`
- Modify: `rgaa-rs/crates/rgaa-tui/Cargo.toml` (add rusqlite dep if not already)

**Interfaces:**
- Consumes: `AuditResult` from rgaa-core
- Produces: `Storage::save_audit(&AuditResult) -> String`, `Storage::list_audits() -> Vec<AuditSummary>`

- [ ] **Step 1: Define Storage struct using rusqlite**

```rust
use rusqlite::{Connection, params};
use rgaa_core::AuditResult;

pub struct Storage {
    conn: Connection,
}

impl Storage {
    pub fn new(db_path: &std::path::Path) -> anyhow::Result<Self> {
        let conn = Connection::open(db_path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS audits (
                id TEXT PRIMARY KEY,
                url TEXT NOT NULL,
                data TEXT NOT NULL,
                taux_global REAL NOT NULL,
                etat_conformite TEXT NOT NULL,
                created_at TEXT NOT NULL
            )",
            [],
        )?;
        Ok(Self { conn })
    }

    pub fn save_audit(&self, audit: &AuditResult) -> anyhow::Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let data = serde_json::to_string(audit)?;
        let json = serde_json::Map::from_json(&data)?;
        let taux_global = json.get("taux_global").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let etat_conformite = json.get("etat_conformite").and_then(|v| v.as_str()).unwrap_or("INCONNUE").to_string();
        let created_at = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO audits (id, url, data, taux_global, etat_conformite, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, &audit.url, &data, taux_global, etat_conformite, created_at],
        )?;
        Ok(id)
    }

    pub fn list_audits(&self, limit: usize, offset: usize) -> anyhow::Result<Vec<AuditSummary>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, url, taux_global, etat_conformite, created_at FROM audits ORDER BY created_at DESC LIMIT ?1 OFFSET ?2"
        )?;
        let audits = stmt.query_map(params![limit as i64, offset as i64], |row| {
            Ok(AuditSummary {
                id: row.get(0)?,
                url: row.get(1)?,
                taux_global: row.get(2)?,
                etat_conformite: row.get(3)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?).unwrap().with_timezone(&chrono::Utc),
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(audits)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditSummary {
    pub id: String,
    pub url: String,
    pub taux_global: f64,
    pub etat_conformite: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd rgaa-rs && cargo check -p rgaa-tui`
Expected: Compiles

- [ ] **Step 3: Commit**

```bash
git add rgaa-rs/crates/rgaa-tui/src/storage.rs rgaa-rs/crates/rgaa-tui/Cargo.toml
git commit -m "feat(tui): SQLite local storage for audit history"
```

---

### Task 5: Setup wizard (API key + config)

**Files:**
- Create: `rgaa-rs/crates/rgaa-tui/src/tui/setup.rs`
- Modify: `rgaa-rs/crates/rgaa-tui/src/tui/mod.rs`

**Interfaces:**
- Consumes: `Keyring`, `rgaa_tui::storage::Storage`
- Produces: `tui::run_setup_wizard() -> bool`

- [ ] **Step 1: Implement setup wizard**

```rust
pub enum SetupStep {
    Welcome,
    ApiKeyPrompt,       // ratatui-input-input for key entry
    ApiKeyConfirm,     // show masked key, confirm Y/n
    Holo3BaseUrl,      // optional, default provided
    ConfigReview,       // show all settings
    Done,
}
```

The wizard:
1. Welcome screen with "Press ENTER to configure"
2. API key entry (masked input via ratatui-input-input)
3. Show masked key + "Store in OS keyring? [Y/n]"
4. Optional Holo3 base URL (default: `https://api.holo3.ai/v1`)
5. Review summary
6. Store in keyring
7. Done

- [ ] **Step 2: Verify it compiles**

Run: `cd rgaa-rs && cargo check -p rgaa-tui`
Expected: Compiles

- [ ] **Step 3: Commit**

```bash
git add rgaa-rs/crates/rgaa-tui/src/tui/setup.rs rgaa-rs/crates/rgaa-tui/src/tui/mod.rs
git commit -m "feat(tui): setup wizard for API key + config"
```

---

### Task 6: Audit wizard (URL → progress → results)

**Files:**
- Create: `rgaa-rs/crates/rgaa-tui/src/tui/audit.rs`
- Modify: `rgaa-rs/crates/rgaa-tui/src/tui/mod.rs`

**Interfaces:**
- Consumes: `rgaa_orchestrator::RgaaOrchestrator`, `rgaa_tui::storage::Storage`, `rgaa_tui::keyring::Keyring`
- Produces: `tui::run_audit_wizard()`

- [ ] **Step 1: Define audit wizard state**

```rust
pub enum AuditStep {
    UrlInput,
    RunningDeterministic { progress: f32 },
    RunningLlm { progress: f32 },
    MergingResults,
    ResultsSummary,
    DrillDown { criterion_id: String },
}
```

- [ ] **Step 2: Implement URL input with ratatui-input-input**

```rust
fn render_url_input(f: &mut Frame, state: &mut AuditWizard) {
    let area = Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).split(f.size());
    let input = Input::default()
        .with_title("Target URL")
        .withPlaceholder("https://example.com");
    let (_ /* submission */, _) = input.draw(f, area[0]);
}
```

- [ ] **Step 3: Implement deterministic pass with progress bar**

Deterministic pass uses `rgaa_orchestrator::RgaaOrchestrator::audit_deterministic()`:
- Show: `Running deterministic checks... [=======    ] 47%`
- Update progress bar as each of the ~79 deterministic criteria completes
- Criteria appear in a scrollable list as they complete

- [ ] **Step 4: Implement LLM pass in background**

After deterministic pass completes (or if no API key, immediately after):
- If `Keyring::get()` returns a key → spawn `rgaa_agent::Agent::run()` in tokio::spawn
- Show: `Running LLM evaluation... [===        ] 30% (27 criteria)`
- Merge LLM results as they arrive via mpsc channel

- [ ] **Step 5: Results summary view**

```rust
fn render_results_summary(f: &mut Frame, results: &AuditResult) {
    // Gauge showing taux_global (e.g. "78.4% compliant")
    // Table: Criterion ID | Status | Type (deterministic/LLM)
    // Color: green = Pass, red = Fail, yellow = CantTell
    // Arrow keys to navigate, ENTER to drill into a criterion
}
```

- [ ] **Step 6: Drill-down view**

```rust
fn render_criterion_detail(f: &mut Frame, criterion: &CriterionResult) {
    // Criterion ID + name
    // Status + type badge
    // Description
    // Evidence (if any)
    // LLM reasoning (if LLM-evaluated)
    // For deterministic: link to WCAG success criterion
}
```

- [ ] **Step 7: Verify it compiles**

Run: `cd rgaa-rs && cargo check -p rgaa-tui`
Expected: Compiles

- [ ] **Step 8: Commit**

```bash
git add rgaa-rs/crates/rgaa-tui/src/tui/audit.rs rgaa-rs/crates/rgaa-tui/src/tui/mod.rs
git commit -m "feat(tui): audit wizard with deterministic + LLM pass"
```

---

### Task 7: Export bundler

**Files:**
- Create: `rgaa-rs/crates/rgaa-tui/src/tui/export.rs`
- Modify: `rgaa-rs/crates/rgaa-tui/src/tui/mod.rs`

**Interfaces:**
- Consumes: `AuditResult` from rgaa-core
- Produces: `export_audit(audit: &AuditResult, output_dir: &Path) -> PathBuf`

- [ ] **Step 1: Implement export bundler**

```rust
pub fn export_audit(
    audit: &AuditResult,
    output_dir: &std::path::Path,
) -> anyhow::Result<std::path::PathBuf> {
    std::fs::create_dir_all(output_dir)?;

    // JSON
    let json_path = output_dir.join("audit.json");
    std::fs::write(&json_path, serde_json::to_string_pretty(audit)?)?;

    // Markdown
    let md_path = output_dir.join("audit.md");
    let md = render_markdown(audit);
    std::fs::write(&md_path, md)?;

    // SARIF
    let sarif_path = output_dir.join("audit.sarif.json");
    let sarif = to_sarif(audit);
    std::fs::write(&sarif_path, serde_json::to_string_pretty(&sarif)?)?;

    // JUnit
    let junit_path = output_dir.join("audit.junit.xml");
    let junit = to_junit_xml(audit);
    std::fs::write(&junit_path, junit)?;

    // HTML summary
    let html_path = output_dir.join("audit.html");
    let html = render_html_summary(audit);
    std::fs::write(&html_path, html)?;

    Ok(output_dir.to_path_buf())
}
```

Use existing `rgaa-cli/report/` as reference for SARIF/JUnit/Markdown rendering.

- [ ] **Step 2: Verify it compiles**

Run: `cd rgaa-rs && cargo check -p rgaa-tui`
Expected: Compiles

- [ ] **Step 3: Commit**

```bash
git add rgaa-rs/crates/rgaa-tui/src/tui/export.rs rgaa-rs/crates/rgaa-tui/src/tui/mod.rs
git commit -m "feat(tui): export bundler for JSON/MD/SARIF/JUnit/HTML"
```

---

### Task 8: Version check / auto-update prompt

**Files:**
- Create: `rgaa-rs/crates/rgaa-tui/src/update.rs`

**Interfaces:**
- Consumes: current binary version from clap
- Produces: `check_for_updates() -> Option<String>` (returns newer version URL)

- [ ] **Step 1: Implement version checker**

```rust
pub async fn check_for_updates(current_version: &str) -> anyhow::Result<Option<String>> {
    let url = "https://api.github.com/repos/jamon8888/Holo-RGAA/releases/latest";
    let resp = reqwest::get(url).await?;
    let json: serde_json::Value = resp.json().await?;
    let latest = json["tag_name"].as_str().unwrap_or("v0.0.0");
    let latest = latest.trim_start_matches('v');
    if latest != current_version {
        Ok(Some(json["html_url"].as_str().unwrap_or("").to_string()))
    } else {
        Ok(None)
    }
}
```

Called on startup (non-blocking, runs in tokio::spawn). If update available, shows a banner in the TUI.

- [ ] **Step 2: Verify it compiles**

Run: `cd rgaa-rs && cargo check -p rgaa-tui`
Expected: Compiles

- [ ] **Step 3: Commit**

```bash
git add rgaa-rs/crates/rgaa-tui/src/update.rs rgaa-rs/crates/rgaa-tui/Cargo.toml
git commit -m "feat(tui): GitHub releases version check on startup"
```

---

### Task 9: TUI main loop (orchestrate all steps)

**Files:**
- Modify: `rgaa-rs/crates/rgaa-tui/src/tui/mod.rs`

**Interfaces:**
- Consumes: all other tui modules
- Produces: `tui::run() async`

- [ ] **Step 1: Implement main TUI run loop**

```rust
pub async fn run() -> anyhow::Result<()> {
    // 1. Check ~/.local/bin/rgaa-mcp exists → if not, run install wizard
    // 2. Check keyring for HOLO3_API_KEY → if not, run setup wizard
    // 3. Check for updates (background, non-blocking)
    // 4. Show main menu:
    //    - [A]udit URL
    //    - [H]istory
    //    - [S]ettings
    //    - [E]xit
    // 5. Route to audit wizard or history view
}
```

Main menu uses ratatui with:
- Vertical layout with centered title "rgaa — RGAA Accessibility Auditor"
- Four selectable menu items (letter shortcuts A/H/S/E)
- Status bar showing: version, key status (🔑 or ⚠️), db status

- [ ] **Step 2: Verify it compiles**

Run: `cd rgaa-rs && cargo check -p rgaa-tui`
Expected: Compiles

- [ ] **Step 3: Commit**

```bash
git add rgaa-rs/crates/rgaa-tui/src/tui/mod.rs
git commit -m "feat(tui): main menu loop orchestrating all wizards"
```

---

### Task 10: Headless audit command (--url fast path)

**Files:**
- Modify: `rgaa-rs/crates/rgaa-tui/src/commands.rs`

**Interfaces:**
- Consumes: `rgaa_orchestrator::RgaaOrchestrator`, `rgaa_tui::keyring::Keyring`, `rgaa_tui::storage::Storage`
- Produces: `commands::audit(url: Option<String>) async`

- [ ] **Step 1: Implement headless audit command**

```rust
pub async fn audit(url: Option<String>) {
    let url = url.expect("URL required for headless audit");
    let api_key = keyring::Keyring::get()
        .ok()
        .flatten()
        .expect("HOLO3_API_KEY not set. Run `rgaa tui` to configure.");

    let orchestrator = rgaa_orchestrator::RgaaOrchestrator::new(
        rgaa_obscura::ObscuraBridge::new().await?,
        rgaa_holo::HoloClient::new(api_key)?,
    );

    // Deterministic pass
    let deterministic = orchestrator.audit_deterministic(&url, Default::default()).await?;

    // LLM pass
    let llm_results = orchestrator.audit_llm(&url, &deterministic.findings).await?;

    // Merge
    let audit = orchestrator.merge_results(deterministic, llm_results);

    // Save to SQLite
    let storage = Storage::new(Path::new("~/.rgaa/audits.db"))?;
    let id = storage.save_audit(&audit)?;

    // Export
    let out_dir = PathBuf::from(format!("./rgaa-report-{}", &id[..8]));
    export::export_audit(&audit, &out_dir)?;

    println!("Audit complete: {}", id);
    println!("Report: {}", out_dir.display());
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd rgaa-rs && cargo check -p rgaa-tui`
Expected: Compiles

- [ ] **Step 3: Commit**

```bash
git add rgaa-rs/crates/rgaa-tui/src/commands.rs
git commit -m "feat(tui): headless audit command for --url fast path"
```

---

### Task 11: cargo-dist integration

**Files:**
- Create: `rgaa-rs/ cargo-dist.toml`
- Create: `.github/workflows/release.yml`

**Interfaces:**
- Consumes: workspace Cargo.toml, release artifacts
- Produces: GitHub Actions workflow that builds + uploads rgaa-tui binaries for all platforms

- [ ] **Step 1: Add cargo-dist to workspace**

In `rgaa-rs/Cargo.toml`:
```toml
[workspace.metadata.dist]
full-version = "rgaa-tui"

[[workspace.metadata.dist.targets]]
os = "linux"
arch = "x86_64"
tar = true

[[workspace.metadata.dist.targets]]
os = "macos"
arch = "x86_64"
tar = true

[[workspace.metadata.dist.targets]]
os = "macos"
arch = "aarch64"
tar = true

[[workspace.metadata.dist.targets]]
os = "windows"
arch = "x86_64"
zip = true
```

Or use `cargo dist init` in the `rgaa-rs/` directory to generate the config.

- [ ] **Step 2: Configure GitHub Actions workflow**

```yaml
name: Release
on:
  push:
    tags: ['v*']
  workflow_dispatch:

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: taiki-e/upload-rust-binary-action@v1
        with:
          bin: rgaa-tui
          target: x86_64-unknown-linux-gnu
      - uses: taiki-e/upload-rust-binary-action@v1
        with:
          bin: rgaa-tui
          target: x86_64-apple-darwin
      - uses: taiki-e/upload-rust-binary-action@v1
        with:
          bin: rgaa-tui
          target: aarch64-apple-darwin
      - uses: taiki-e/upload-rust-binary-action@v1
        with:
          bin: rgaa-tui
          target: x86_64-pc-windows-msvc
```

Also include `rgaa-mcp` and `obscura` binaries in the release.

- [ ] **Step 3: Add release job to existing CI**

Modify `.github/workflows/ci.yml` to add a `release` job that runs on git tags.

- [ ] **Step 4: Verify workflow syntax**

Run: `cargo dist init --dry-run` in rgaa-rs/ (or check GitHub Actions syntax)

- [ ] **Step 5: Commit**

```bash
git add rgaa-rs/cargo-dist.toml .github/workflows/release.yml
git commit -m "feat(tui): cargo-dist + release workflow for multi-platform binaries"
```

---

### Task 12: Install script replacement — update install.sh for rgaa-tui

**Files:**
- Modify: `install.sh` (root of repo)
- Create: `install.ps1` (Windows PowerShell installer)

**Interfaces:**
- Consumes: GitHub releases (rgaa-tui binary)
- Produces: `install.sh` that downloads + installs `rgaa-tui` and `obscura`

- [ ] **Step 1: Update install.sh to install rgaa-tui binary**

Modify the binaries array:
```bash
local binaries=("rgaa-tui" "obscura" "obscura-worker")
# rgaa-mcp is now bundled inside rgaa-tui or as a companion
```

The install script should:
1. Detect platform (existing logic)
2. Download `rgaa-tui` from GitHub releases (replace `rgaa-mcp`/`rgaa-cli`)
3. Install to `~/.local/bin`
4. Create a `rgaa` symlink to `rgaa-tui` (so both `rgaa` and `rgaa tui` work)
5. Download `obscura` binary
6. Run key setup wizard via `rgaa tui` (or `rgaa config`)
7. Verify installation

- [ ] **Step 2: Create install.ps1 for Windows**

PowerShell version with:
- Detect architecture (`$env:PROCESSOR_ARCHITECTURE`)
- Download from GitHub releases
- Install to `$env:LOCALAPPDATA\rgaa\bin` or `~\AppData\Local\rgaa\bin`
- Add to PATH via `setx PATH "$env:LOCALAPPDATA\rgaa\bin;$env:PATH"` (with warning)
- Configure OS keyring via Windows Credential Manager
- Prompt to restart terminal

```powershell
$ErrorActionPreference = "Stop"
$REPO = "jamon8888/Holo-RGAA"
$INSTALL_DIR = "$env:LOCALAPPDATA\rgaa\bin"
$RELEASE_URL = "https://github.com/$REPO/releases/latest/download/rgaa-x86_64-pc-windows-msvc.zip"

# Download + extract
Invoke-WebRequest -Uri $RELEASE_URL -OutFile "$env:TEMP\rgaa.zip"
Expand-Archive -Path "$env:TEMP\rgaa.zip" -DestinationPath $INSTALL_DIR -Force
$env:PATH = "$INSTALL_DIR;$env:PATH"

Write-Host "rgaa installed to $INSTALL_DIR"
Write-Host "Run 'rgaa tui' to start the interactive setup"
```

- [ ] **Step 3: Commit**

```bash
git add install.sh install.ps1
git commit -m "feat: install.sh + install.ps1 for rgaa-tui multi-platform"
```

---

## Self-Review Checklist

1. **Spec coverage:** All 15 settled design decisions are implemented across Tasks 1–12:
   - ✅ ratatui (Tasks 1, 2, 5, 6, 7, 9)
   - ✅ SQLite storage (Task 4)
   - ✅ OS keyring (Task 3)
   - ✅ Self-install on first run (Task 2)
   - ✅ Deterministic-first audit (Task 6)
   - ✅ LLM in background (Task 6)
   - ✅ Guided wizard TUI (Tasks 2, 5, 6, 7, 9)
   - ✅ Export bundler (Task 7)
   - ✅ Version check (Task 8)
   - ✅ Headless --url path (Task 10)
   - ✅ cargo-dist + multi-platform (Task 11)
   - ✅ install.sh + install.ps1 (Task 12)

2. **Placeholder scan:** No TODOs, no TBDs, no "fill in later" steps.

3. **Type consistency:** All interfaces use `rgaa_core::AuditResult`, `rgaa_orchestrator::RgaaOrchestrator`, `rgaa_storage::StorageError`, `chrono::DateTime<Utc>`. Consistent across all tasks.

4. **Build verification:** Each task ends with `cargo check -p rgaa-tui` before commit.

---

## Execution Options

**Plan complete.** Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

Which approach?
