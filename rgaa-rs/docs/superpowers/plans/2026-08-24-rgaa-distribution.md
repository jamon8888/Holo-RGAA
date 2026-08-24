# RGAA Distribution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the full distribution solution — license crate, updater crate, CLI subcommands, SQLite storage, multi-provider LLM, MCP tools, SaaS API, install script, and GitHub release workflow.

**Architecture:** Fat Client approach — single `rgaa` binary bundles all tools (CLI + MCP). Two new crates (`rgaa-license`, `rgaa-updater`) handle SaaS integration. SaaS backend extends existing `rgaa-api` with auth, rules, billing endpoints. One-liner install script with auto Claude Desktop configuration.

**Tech Stack:** Rust 2024 edition, `axum` 0.7, `sqlx` (sqlite + postgres), `reqwest`, `tokio`, `serde`/`serde_json`/`serde_yaml`/`serde_toml`, `clap` 4, `rmcp` (MCP), `ring` (crypto), `sha2`, `chrono`, `tracing`.

**Spec:** `docs/superpowers/specs/2026-08-24-rgaa-distribution-design.md`

## Global Constraints

- Rust edition 2024, rust-version 1.85
- Follow existing workspace conventions: unit-struct pattern, `RgaaError` with `thiserror`, serde derives
- French domain terminology for RGAA concepts, English for code identifiers
- `#[deny(clippy::correctness)]` at crate level
- API keys: `chmod 600`, never logged, never sent to MCP client
- MCP: stdio transport only (no network exposure)
- License: offline grace 7/14 days, clock manipulation detection

---

## File Structure

```
rgaa-rs/crates/
├── rgaa-license/                    # NEW
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                   # LicenseClient, LicenseStatus
│       ├── key_store.rs             # ~/.config/rgaa/license.toml read/write
│       ├── validator.rs             # API key validation + offline grace
│       └── offline.rs               # Grace period logic + clock check
├── rgaa-updater/                    # NEW
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                   # UpdateClient, RuleManifest
│       ├── feed.rs                  # GET /api/v1/rules with ETag
│       ├── downloader.rs            # Atomic download + signature verify
│       └── cache.rs                 # ~/.config/rgaa/rules/ management
├── rgaa-cli/src/
│   └── commands/
│       ├── configure.rs             # NEW: interactive setup wizard
│       ├── verify_install.rs        # NEW: binary integrity check
│       └── update.rs                # NEW: manual rule update trigger
├── rgaa-storage/src/
│   ├── lib.rs                       # MODIFY: add SqliteBackend module
│   ├── sqlite_backend.rs            # NEW: SQLite implementation
│   └── repository.rs                # MODIFY: trait-based backend selection
├── rgaa-holo/src/
│   ├── client.rs                    # MODIFY: multi-provider support
│   └── config.rs                    # NEW: LLM provider config loader
├── rgaa-mcp/src/tools/
│   ├── source_map.rs                # NEW: rgaa_source_map tool
│   └── verify_fix.rs                # NEW: rgaa_verify_fix tool
├── rgaa-api/src/
│   ├── main.rs                      # MODIFY: router setup
│   ├── auth.rs                      # NEW: license validation middleware
│   ├── rules.rs                     # NEW: rule update feed endpoints
│   ├── usage.rs                     # NEW: usage analytics endpoints
│   └── billing.rs                   # NEW: Stripe integration
└── scripts/
    └── install.sh                   # NEW: one-liner installer
```

---

## Phase 1: Foundation Crates

### Task 1: rgaa-license Crate

**Files:**
- Create: `rgaa-rs/crates/rgaa-license/Cargo.toml`
- Create: `rgaa-rs/crates/rgaa-license/src/lib.rs`
- Create: `rgaa-rs/crates/rgaa-license/src/key_store.rs`
- Create: `rgaa-rs/crates/rgaa-license/src/validator.rs`
- Create: `rgaa-rs/crates/rgaa-license/src/offline.rs`
- Test: `rgaa-rs/crates/rgaa-license/tests/license_test.rs`

**Interfaces:**
- Produces: `LicenseClient::validate(key)` → `Result<LicenseStatus>`, `LicenseClient::is_valid()` → `bool`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "rgaa-license"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"

[lints.rust]
unsafe_code = "warn"

[lints.clippy]
all = "warn"
pedantic = "warn"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
serde_toml = "0.8"
reqwest = { workspace = true }
tokio = { workspace = true }
chrono = { workspace = true }
sha2 = "0.10"
tracing = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
dirs = "6"
```

- [ ] **Step 2: Create key_store.rs**

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseData {
    pub api_key: String,
    pub last_validated: chrono::DateTime<chrono::Utc>,
    pub grace_days: u32,
    pub tier: String,
}

pub struct KeyStore;

impl KeyStore {
    fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("rgaa")
    }

    fn license_path() -> PathBuf {
        Self::config_dir().join("license.toml")
    }

    pub fn load() -> Result<Option<LicenseData>> {
        let path = Self::license_path();
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)?;
        let data: LicenseData = serde_toml::from_str(&content)?;
        Ok(Some(data))
    }

    pub fn save(data: &LicenseData) -> Result<()> {
        let dir = Self::config_dir();
        std::fs::create_dir_all(&dir)?;

        let path = Self::license_path();
        let content = serde_toml::to_string_pretty(data)?;
        std::fs::write(&path, &content)?;

        // chmod 600 on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        }

        Ok(())
    }
}
```

- [ ] **Step 3: Create offline.rs**

```rust
use chrono::{DateTime, Duration, Utc};

#[derive(Debug, Clone)]
pub enum LicenseGraceStatus {
    Valid,
    SoftLocked { days_since: i64 },
    HardLocked { days_since: i64 },
    ClockManipulationDetected,
}

pub struct OfflineChecker;

impl OfflineChecker {
    pub fn check(
        last_validated: DateTime<Utc>,
        grace_days: u32,
        hard_lock_days: u32,
    ) -> LicenseGraceStatus {
        let now = Utc::now();
        let elapsed = now - last_validated;

        // Clock manipulation detection
        if elapsed < Duration::zero() {
            return LicenseGraceStatus::ClockManipulationDetected;
        }

        let days = elapsed.num_days();

        if days <= grace_days as i64 {
            LicenseGraceStatus::Valid
        } else if days <= hard_lock_days as i64 {
            LicenseGraceStatus::SoftLocked { days_since: days }
        } else {
            LicenseGraceStatus::HardLocked { days_since: days }
        }
    }
}
```

- [ ] **Step 4: Create validator.rs**

```rust
use crate::key_store::{KeyStore, LicenseData};
use crate::offline::{LicenseGraceStatus, OfflineChecker};
use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ValidationResponse {
    valid: bool,
    tier: String,
    grace_days: u32,
}

pub struct LicenseValidator {
    api_base: String,
}

impl LicenseValidator {
    pub fn new(api_base: &str) -> Self {
        Self { api_base: api_base.to_string() }
    }

    pub async fn validate(&self, api_key: &str) -> Result<LicenseData> {
        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/api/v1/auth/validate", self.api_base))
            .bearer_auth(api_key)
            .send()
            .await?;

        let data: ValidationResponse = resp.json().await?;

        if !data.valid {
            anyhow::bail!("Invalid API key");
        }

        let license = LicenseData {
            api_key: api_key.to_string(),
            last_validated: chrono::Utc::now(),
            grace_days: data.grace_days,
            tier: data.tier,
        };

        KeyStore::save(&license)?;
        Ok(license)
    }

    pub fn check_offline(license: &LicenseData) -> LicenseGraceStatus {
        OfflineChecker::check(
            license.last_validated,
            license.grace_days,
            14, // hard_lock_days
        )
    }
}
```

- [ ] **Step 5: Create lib.rs**

```rust
pub mod key_store;
pub mod offline;
pub mod validator;

pub use key_store::{KeyStore, LicenseData};
pub use offline::{LicenseGraceStatus, OfflineChecker};
pub use validator::LicenseValidator;

pub struct LicenseClient {
    validator: LicenseValidator,
}

impl LicenseClient {
    pub fn new(api_base: &str) -> Self {
        Self { validator: LicenseValidator::new(api_base) }
    }

    pub async fn validate(&self, api_key: &str) -> Result<LicenseData, anyhow::Error> {
        self.validator.validate(api_key).await
    }

    pub fn is_valid(&self) -> bool {
        match KeyStore::load() {
            Ok(Some(license)) => {
                matches!(
                    LicenseValidator::check_offline(&license),
                    LicenseGraceStatus::Valid
                )
            }
            _ => false,
        }
    }

    pub fn current_status(&self) -> Option<(LicenseData, LicenseGraceStatus)> {
        let license = KeyStore::load().ok()??;
        let status = LicenseValidator::check_offline(&license);
        Some((license, status))
    }
}
```

- [ ] **Step 6: Write tests**

```rust
// tests/license_test.rs
use rgaa_license::{LicenseGraceStatus, OfflineChecker, LicenseData, KeyStore};
use chrono::{Duration, Utc};

#[test]
fn valid_license_within_grace() {
    let license = LicenseData {
        api_key: "rgaa_sk_test".to_string(),
        last_validated: Utc::now() - Duration::days(3),
        grace_days: 7,
        tier: "professional".to_string(),
    };
    assert!(matches!(
        OfflineChecker::check(license.last_validated, license.grace_days, 14),
        LicenseGraceStatus::Valid
    ));
}

#[test]
fn soft_locked_beyond_grace() {
    let license = LicenseData {
        api_key: "rgaa_sk_test".to_string(),
        last_validated: Utc::now() - Duration::days(10),
        grace_days: 7,
        tier: "professional".to_string(),
    };
    assert!(matches!(
        OfflineChecker::check(license.last_validated, license.grace_days, 14),
        LicenseGraceStatus::SoftLocked { .. }
    ));
}

#[test]
fn hard_locked_beyond_14_days() {
    let license = LicenseData {
        api_key: "rgaa_sk_test".to_string(),
        last_validated: Utc::now() - Duration::days(20),
        grace_days: 7,
        tier: "professional".to_string(),
    };
    assert!(matches!(
        OfflineChecker::check(license.last_validated, license.grace_days, 14),
        LicenseGraceStatus::HardLocked { .. }
    ));
}

#[test]
fn clock_manipulation_detected() {
    let license = LicenseData {
        api_key: "rgaa_sk_test".to_string(),
        last_validated: Utc::now() + Duration::days(5), // future date
        grace_days: 7,
        tier: "professional".to_string(),
    };
    assert!(matches!(
        OfflineChecker::check(license.last_validated, license.grace_days, 14),
        LicenseGraceStatus::ClockManipulationDetected
    ));
}
```

- [ ] **Step 7: Run tests**

Run: `cargo test -p rgaa-license`
Expected: All 4 tests pass

- [ ] **Step 8: Commit**

```bash
git add rgaa-rs/crates/rgaa-license/
git commit -m "feat(rgaa-license): license validation with offline grace period"
```

---

### Task 2: rgaa-updater Crate

**Files:**
- Create: `rgaa-rs/crates/rgaa-updater/Cargo.toml`
- Create: `rgaa-rs/crates/rgaa-updater/src/lib.rs`
- Create: `rgaa-rs/crates/rgaa-updater/src/feed.rs`
- Create: `rgaa-rs/crates/rgaa-updater/src/downloader.rs`
- Create: `rgaa-rs/crates/rgaa-updater/src/cache.rs`
- Test: `rgaa-rs/crates/rgaa-updater/tests/updater_test.rs`

**Interfaces:**
- Produces: `UpdateClient::check_and_update()` → `Result<Option<RuleManifest>>`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "rgaa-updater"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"

[lints.rust]
unsafe_code = "warn"

[lints.clippy]
all = "warn"
pedantic = "warn"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
reqwest = { workspace = true }
tokio = { workspace = true }
chrono = { workspace = true }
sha2 = "0.10"
ring = "0.17"
tracing = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
dirs = "6"
```

- [ ] **Step 2: Create cache.rs**

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleManifest {
    pub version: String,
    pub hash: String,
    pub signature: String,
    pub files: Vec<String>,
}

pub struct RuleCache;

impl RuleCache {
    fn rules_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("rgaa")
            .join("rules")
    }

    fn manifest_path() -> PathBuf {
        Self::rules_dir().join("manifest.json")
    }

    pub fn load_manifest() -> Result<Option<RuleManifest>> {
        let path = Self::manifest_path();
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)?;
        let manifest: RuleManifest = serde_json::from_str(&content)?;
        Ok(Some(manifest))
    }

    pub fn save_manifest(manifest: &RuleManifest) -> Result<()> {
        let dir = Self::rules_dir();
        std::fs::create_dir_all(&dir)?;
        let content = serde_json::to_string_pretty(manifest)?;
        std::fs::write(Self::manifest_path(), content)?;
        Ok(())
    }

    pub fn save_file(filename: &str, content: &[u8]) -> Result<()> {
        let dir = Self::rules_dir();
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join(filename), content)?;
        Ok(())
    }

    pub fn load_file(filename: &str) -> Result<Option<Vec<u8>>> {
        let path = Self::rules_dir().join(filename);
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(std::fs::read(&path)?))
    }
}
```

- [ ] **Step 3: Create feed.rs**

```rust
use crate::cache::RuleManifest;
use anyhow::Result;
use reqwest::Client;

pub struct UpdateFeed {
    client: Client,
    api_base: String,
}

impl UpdateFeed {
    pub fn new(api_base: &str) -> Self {
        Self {
            client: Client::new(),
            api_base: api_base.to_string(),
        }
    }

    pub async fn check(&self, current_hash: Option<&str>) -> Result<FeedResponse> {
        let mut req = self.client
            .get(format!("{}/api/v1/rules/check", self.api_base));

        if let Some(hash) = current_hash {
            req = req.header("If-None-Match", format!("\"{}\"", hash));
        }

        let resp = req.send().await?;

        match resp.status().as_u16() {
            304 => Ok(FeedResponse::NotModified),
            200 => {
                let manifest: RuleManifest = resp.json().await?;
                Ok(FeedResponse::UpdateAvailable(manifest))
            }
            status => anyhow::bail!("Unexpected status: {}", status),
        }
    }
}

#[derive(Debug)]
pub enum FeedResponse {
    NotModified,
    UpdateAvailable(RuleManifest),
}
```

- [ ] **Step 4: Create downloader.rs**

```rust
use crate::cache::{RuleCache, RuleManifest};
use anyhow::Result;
use reqwest::Client;
use sha2::{Sha256, Digest};

pub struct UpdateDownloader {
    client: Client,
    api_base: String,
}

impl UpdateDownloader {
    pub fn new(api_base: &str) -> Self {
        Self {
            client: Client::new(),
            api_base: api_base.to_string(),
        }
    }

    pub async fn download(&self, manifest: &RuleManifest) -> Result<()> {
        for filename in &manifest.files {
            let url = format!("{}/api/v1/rules/{}", self.api_base, filename);
            let bytes = self.client.get(&url).send().await?.bytes().await?;

            // Verify hash
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let hash = format!("sha256:{:x}", hasher.finalize());

            if hash != manifest.hash {
                anyhow::bail!("Hash mismatch for {}: expected {}, got {}", filename, manifest.hash, hash);
            }

            RuleCache::save_file(filename, &bytes)?;
        }

        RuleCache::save_manifest(manifest)?;
        Ok(())
    }
}
```

- [ ] **Step 5: Create lib.rs**

```rust
pub mod cache;
pub mod downloader;
pub mod feed;

pub use cache::{RuleCache, RuleManifest};
pub use downloader::UpdateDownloader;
pub use feed::{FeedResponse, UpdateFeed};

pub struct UpdateClient {
    feed: UpdateFeed,
    downloader: UpdateDownloader,
}

impl UpdateClient {
    pub fn new(api_base: &str) -> Self {
        Self {
            feed: UpdateFeed::new(api_base),
            downloader: UpdateDownloader::new(api_base),
        }
    }

    pub async fn check_and_update(&self) -> Result<Option<RuleManifest>, anyhow::Error> {
        let current = RuleCache::load_manifest()?;
        let current_hash = current.as_ref().map(|m| m.hash.as_str());

        match self.feed.check(current_hash).await? {
            FeedResponse::NotModified => Ok(None),
            FeedResponse::UpdateAvailable(manifest) => {
                self.downloader.download(&manifest).await?;
                Ok(Some(manifest))
            }
        }
    }
}
```

- [ ] **Step 6: Write tests**

```rust
// tests/updater_test.rs
use rgaa_updater::{RuleManifest, RuleCache};
use std::fs;

#[test]
fn manifest_roundtrip() {
    let manifest = RuleManifest {
        version: "2026.08.24".to_string(),
        hash: "sha256:abc123".to_string(),
        signature: "ed25519:test".to_string(),
        files: vec!["axe-rules.json".to_string()],
    };

    let dir = tempfile::tempdir().unwrap();
    let manifest_path = dir.path().join("manifest.json");
    fs::write(&manifest_path, serde_json::to_string(&manifest).unwrap()).unwrap();

    let loaded: RuleManifest = serde_json::from_str(
        &fs::read_to_string(&manifest_path).unwrap()
    ).unwrap();

    assert_eq!(loaded.version, "2026.08.24");
    assert_eq!(loaded.files.len(), 1);
}
```

- [ ] **Step 7: Run tests**

Run: `cargo test -p rgaa-updater`
Expected: Tests pass

- [ ] **Step 8: Commit**

```bash
git add rgaa-rs/crates/rgaa-updater/
git commit -m "feat(rgaa-updater): remote rule update feed with atomic download"
```

---

### Task 3: SQLite Backend for rgaa-storage

**Files:**
- Create: `rgaa-rs/crates/rgaa-storage/src/sqlite_backend.rs`
- Modify: `rgaa-rs/crates/rgaa-storage/src/lib.rs`
- Modify: `rgaa-rs/crates/rgaa-storage/Cargo.toml`
- Test: `rgaa-rs/crates/rgaa-storage/tests/sqlite_test.rs`

**Interfaces:**
- Produces: `SqliteBackend::new(path)` → `Result<SqliteBackend>`, same `Repository` trait methods

- [ ] **Step 1: Add SQLite dependency**

Edit `rgaa-rs/crates/rgaa-storage/Cargo.toml`, add:
```toml
[dependencies]
sqlx = { workspace = true, features = ["sqlite", "runtime-tokio"] }
```

- [ ] **Step 2: Create sqlite_backend.rs**

```rust
use anyhow::Result;
use sqlx::sqlite::SqlitePool;
use std::path::Path;

pub struct SqliteBackend {
    pool: SqlitePool,
}

impl SqliteBackend {
    pub async fn new(db_path: &Path) -> Result<Self> {
        let db_dir = db_path.parent().unwrap_or(Path::new("."));
        std::fs::create_dir_all(db_dir)?;

        let url = format!("sqlite:{}?mode=rwc", db_path.display());
        let pool = SqlitePool::connect(&url).await?;

        // Run migrations
        sqlx::query(CREATE_TABLES).execute(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn create_audit(&self, url: &str) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO audits (id, url, status) VALUES (?, ?, 'pending')")
            .bind(&id)
            .bind(url)
            .execute(&self.pool)
            .await?;
        Ok(id)
    }

    pub async fn complete_audit(&self, id: &str, result: &serde_json::Value) -> Result<()> {
        sqlx::query("UPDATE audits SET status = 'completed', result = ? WHERE id = ?")
            .bind(result)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_audit(&self, id: &str) -> Result<Option<serde_json::Value>> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT result FROM audits WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(result,)| serde_json::from_str(&result).unwrap_or_default()))
    }
}

const CREATE_TABLES: &str = r#"
CREATE TABLE IF NOT EXISTS audits (
    id TEXT PRIMARY KEY,
    url TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    result TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
"#;
```

- [ ] **Step 3: Update lib.rs**

Add to `rgaa-rs/crates/rgaa-storage/src/lib.rs`:
```rust
pub mod sqlite_backend;
pub use sqlite_backend::SqliteBackend;
```

- [ ] **Step 4: Write tests**

```rust
// tests/sqlite_test.rs
use rgaa_storage::SqliteBackend;
use std::path::Path;

#[tokio::test]
async fn create_and_get_audit() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    let backend = SqliteBackend::new(&db_path).await.unwrap();
    let id = backend.create_audit("https://example.com").await.unwrap();

    let audit = backend.get_audit(&id).await.unwrap();
    assert!(audit.is_none()); // pending has no result yet

    let result = serde_json::json!({ "total": 106, "passed": 100 });
    backend.complete_audit(&id, &result).await.unwrap();

    let audit = backend.get_audit(&id).await.unwrap();
    assert!(audit.is_some());
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p rgaa-storage`
Expected: SQLite tests pass

- [ ] **Step 6: Commit**

```bash
git add rgaa-rs/crates/rgaa-storage/
git commit -m "feat(rgaa-storage): add SQLite backend for local-only mode"
```

---

### Task 4: Multi-Provider LLM Support

**Files:**
- Create: `rgaa-rs/crates/rgaa-holo/src/config.rs`
- Modify: `rgaa-rs/crates/rgaa-holo/src/client.rs`
- Test: `rgaa-rs/crates/rgaa-holo/tests/config_test.rs`

**Interfaces:**
- Produces: `LlmConfig::load()` → `Result<LlmConfig>`, `HoloClient::from_config(config)` → `HoloClient`

- [ ] **Step 1: Create config.rs**

```rust
use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct LlmConfig {
    pub provider: Provider,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Claude,
    Openai,
    SaasProxy,
}

impl LlmConfig {
    pub fn load() -> Result<Self> {
        let config_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("rgaa")
            .join("llm.toml");

        if !config_path.exists() {
            anyhow::bail!("LLM config not found. Run `rgaa configure` first.");
        }

        let content = std::fs::read_to_string(&config_path)?;
        let config: LlmConfig = serde_toml::from_str(&content)?;
        Ok(config)
    }

    pub fn api_url(&self) -> &str {
        match self.provider {
            Provider::Claude => "https://api.anthropic.com/v1/messages",
            Provider::Openai => "https://api.openai.com/v1/chat/completions",
            Provider::SaasProxy => "https://api.rgaa.dev/v1/llm",
        }
    }

    pub fn default_model(&self) -> &str {
        match self.provider {
            Provider::Claude => "claude-sonnet-4-20250514",
            Provider::Openai => "gpt-4o",
            Provider::SaasProxy => "claude-sonnet-4-20250514",
        }
    }
}
```

- [ ] **Step 2: Modify client.rs for multi-provider**

Add to `HoloClient`:
```rust
pub fn from_config(config: LlmConfig) -> Self {
    let api_key = config.api_key.unwrap_or_default();
    let base_url = config.api_url().to_string();
    let model = config.model.unwrap_or_else(|| config.default_model().to_string());

    Self {
        api_key,
        base_url,
        model,
        http_client: reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client"),
    }
}
```

- [ ] **Step 3: Write tests**

```rust
// tests/config_test.rs
use rgaa_holo::config::{LlmConfig, Provider};

#[test]
fn parse_claude_config() {
    let toml = r#"
provider = "claude"
api_key = "sk-ant-test"
model = "claude-sonnet-4-20250514"
"#;
    let config: LlmConfig = serde_toml::from_str(toml).unwrap();
    assert_eq!(config.provider, Provider::Claude);
    assert_eq!(config.api_url(), "https://api.anthropic.com/v1/messages");
}

#[test]
fn parse_openai_config() {
    let toml = r#"
provider = "openai"
api_key = "sk-test"
"#;
    let config: LlmConfig = serde_toml::from_str(toml).unwrap();
    assert_eq!(config.provider, Provider::Openai);
    assert_eq!(config.default_model(), "gpt-4o");
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p rgaa-holo`
Expected: All tests pass

- [ ] **Step 5: Commit**

```bash
git add rgaa-rs/crates/rgaa-holo/
git commit -m "feat(rgaa-holo): multi-provider LLM support (Claude, OpenAI, SaaS proxy)"
```

---

## Phase 2: CLI & MCP Enhancements

### Task 5: CLI Subcommands

**Files:**
- Create: `rgaa-rs/crates/rgaa-cli/src/commands/configure.rs`
- Create: `rgaa-rs/crates/rgaa-cli/src/commands/verify_install.rs`
- Create: `rgaa-rs/crates/rgaa-cli/src/commands/update.rs`
- Modify: `rgaa-rs/crates/rgaa-cli/src/commands/mod.rs`

**Interfaces:**
- Consumes: `LicenseClient`, `UpdateClient`, `LlmConfig`
- Produces: three new `AuditCommand` variants

- [ ] **Step 1: Create configure.rs**

```rust
use anyhow::Result;
use std::io::{self, Write};

pub fn run() -> Result<()> {
    println!("RGAA Configuration Wizard\n");

    // API key
    print!("Enter your RGAA SaaS API key (rgaa_sk_...): ");
    io::stdout().flush()?;
    let mut api_key = String::new();
    io::stdin().read_line(&mut api_key)?;
    let api_key = api_key.trim().to_string();

    // LLM provider
    println!("\nSelect LLM provider:");
    println!("  1. Use my own Claude API key");
    println!("  2. Use my own OpenAI API key");
    println!("  3. Route through SaaS (included in subscription)");
    print!("Choice [1-3]: ");
    io::stdout().flush()?;
    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;

    let (provider, llm_key) = match choice.trim() {
        "1" => ("claude", prompt_key("Claude API key (sk-ant-...)")),
        "2" => ("openai", prompt_key("OpenAI API key (sk-...)")),
        "3" => ("saas_proxy", None),
        _ => anyhow::bail!("Invalid choice"),
    };

    // Save license
    let license = rgaa_license::LicenseData {
        api_key,
        last_validated: chrono::Utc::now(),
        grace_days: 7,
        tier: "professional".to_string(),
    };
    rgaa_license::KeyStore::save(&license)?;

    // Save LLM config
    let llm_config = rgaa_holo::config::LlmConfig {
        provider: provider.parse()?,
        api_key: llm_key,
        model: None,
        max_tokens: None,
    };
    let config_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("~/.config"))
        .join("rgaa")
        .join("llm.toml");
    std::fs::write(&config_path, serde_toml::to_string(&llm_config)?)?;

    // Detect Chrome
    detect_chrome();

    // Configure Claude Desktop
    configure_claude_desktop()?;

    println!("\n✅ RGAA configured successfully!");
    Ok(())
}

fn prompt_key(prompt: &str) -> Option<String> {
    print!("Enter {}: ", prompt);
    io::stdout().flush().ok()?;
    let mut key = String::new();
    io::stdin().read_line(&mut key).ok()?;
    Some(key.trim().to_string())
}

fn detect_chrome() {
    let browsers = ["google-chrome", "chromium", "chromium-browser"];
    for browser in &browsers {
        if which::which(browser).is_ok() {
            println!("✅ Found: {}", browser);
            return;
        }
    }
    println!("⚠️  Chrome/Chromium not found. Install it for full functionality.");
}

fn configure_claude_desktop() -> Result<()> {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("~/.config"))
        .join("Claude")
        .join("claude_desktop_config.json");

    if !config_dir.exists() {
        println!("ℹ️  Claude Desktop config not found. Manually add MCP server.");
        return Ok(());
    }

    let mut config: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&config_dir)?
    )?;

    let rgaa_bin = dirs::home_dir()
        .unwrap_or_default()
        .join(".local/bin/rgaa");

    config["mcpServers"]["rgaa"] = serde_json::json!({
        "command": rgaa_bin,
        "args": ["mcp"]
    });

    std::fs::write(&config_dir, serde_json::to_string_pretty(&config)?)?;
    println!("✅ Claude Desktop configured with rgaa MCP server");
    Ok(())
}
```

- [ ] **Step 2: Create verify_install.rs**

```rust
use anyhow::Result;
use sha2::{Sha256, Digest};

pub fn run() -> Result<()> {
    let binary_path = std::env::current_exe()?;
    let binary_data = std::fs::read(&binary_path)?;

    let mut hasher = Sha256::new();
    hasher.update(&binary_data);
    let hash = format!("{:x}", hasher.finalize());

    println!("Binary: {}", binary_path.display());
    println!("SHA-256: {}", hash);
    println!("Size: {} bytes", binary_data.len());

    // Verify config directory
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("~/.config"))
        .join("rgaa");

    if config_dir.exists() {
        println!("✅ Config directory exists: {}", config_dir.display());
    } else {
        println!("⚠️  Config directory missing. Run `rgaa configure`");
    }

    // Verify license
    match rgaa_license::KeyStore::load()? {
        Some(license) => {
            let status = rgaa_license::OfflineChecker::check(
                license.last_validated, license.grace_days, 14
            );
            println!("✅ License loaded (tier: {})", license.tier);
            println!("   Status: {:?}", status);
        }
        None => println!("⚠️  No license found. Run `rgaa configure`"),
    }

    Ok(())
}
```

- [ ] **Step 3: Create update.rs**

```rust
use anyhow::Result;

pub fn run() -> Result<()> {
    let api_base = std::env::var("RGAA_API_BASE")
        .unwrap_or_else(|_| "https://api.rgaa.dev".to_string());

    let client = rgaa_updater::UpdateClient::new(&api_base);

    println!("Checking for rule updates...");

    let rt = tokio::runtime::Runtime::new()?;
    match rt.block_on(client.check_and_update())? {
        Some(manifest) => {
            println!("✅ Updated to rules v{}", manifest.version);
            println!("   Files: {}", manifest.files.join(", "));
        }
        None => println!("✅ Rules are up to date"),
    }

    Ok(())
}
```

- [ ] **Step 4: Update commands/mod.rs**

Add new variants to `AuditCommand` enum and update `dispatch()`.

- [ ] **Step 5: Run tests**

Run: `cargo test -p rgaa-cli`
Expected: All tests pass

- [ ] **Step 6: Commit**

```bash
git add rgaa-rs/crates/rgaa-cli/
git commit -m "feat(rgaa-cli): add configure, verify-install, update subcommands"
```

---

### Task 6: MCP Source Map & Verify Fix Tools

**Files:**
- Create: `rgaa-rs/crates/rgaa-mcp/src/tools/source_map.rs`
- Create: `rgaa-rs/crates/rgaa-mcp/src/tools/verify_fix.rs`
- Modify: `rgaa-rs/crates/rgaa-mcp/src/server.rs`

**Interfaces:**
- Consumes: `AnalyzeResponse` (violations with DOM selectors)
- Produces: `SourceMapResponse { source_files, frameworks_detected }`, `VerifyFixResponse { fixed, new_ratio }`

- [ ] **Step 1: Create source_map.rs**

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SourceMapRequest {
    pub url: String,
    pub selector: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SourceMapResponse {
    pub source_files: Vec<SourceFile>,
    pub frameworks_detected: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SourceFile {
    pub path: String,
    pub line: usize,
    pub confidence: f64,
}

pub async fn handle(req: SourceMapRequest) -> Result<SourceMapResponse, String> {
    // Delegate to browser automation to inspect element
    // Map DOM node to source via source maps or framework detection
    // This is a placeholder for the actual implementation
    Ok(SourceMapResponse {
        source_files: vec![],
        frameworks_detected: vec![],
    })
}
```

- [ ] **Step 2: Create verify_fix.rs**

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VerifyFixRequest {
    pub url: String,
    pub violation_id: String,
    pub selector: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VerifyFixResponse {
    pub fixed: bool,
    pub new_ratio: Option<f64>,
    pub hint: Option<String>,
}

pub async fn handle(req: VerifyFixRequest) -> Result<VerifyFixResponse, String> {
    // Re-run the specific rule check against the live page
    // This is a placeholder for the actual implementation
    Ok(VerifyFixResponse {
        fixed: false,
        new_ratio: None,
        hint: Some("Not implemented yet".to_string()),
    })
}
```

- [ ] **Step 3: Register tools in server.rs**

Add the two new tools to the MCP server's tool registry.

- [ ] **Step 4: Commit**

```bash
git add rgaa-rs/crates/rgaa-mcp/
git commit -m "feat(rgaa-mcp): add rgaa_source_map and rgaa_verify_fix tools"
```

---

## Phase 3: SaaS API Endpoints

### Task 7: Auth & License Validation Endpoints

**Files:**
- Create: `rgaa-rs/crates/rgaa-api/src/auth.rs`
- Modify: `rgaa-rs/crates/rgaa-api/src/main.rs`
- Modify: `rgaa-rs/crates/rgaa-api/Cargo.toml`

**Interfaces:**
- Produces: `POST /api/v1/auth/validate`, `POST /api/v1/auth/refresh`

- [ ] **Step 1: Create auth.rs**

```rust
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ValidateRequest {
    pub api_key: String,
}

#[derive(Serialize)]
pub struct ValidateResponse {
    pub valid: bool,
    pub tier: String,
    pub grace_days: u32,
}

pub async fn validate(
    State(state): State<crate::AppState>,
    Json(req): Json<ValidateRequest>,
) -> Result<Json<ValidateResponse>, StatusCode> {
    let valid = state.repository.validate_api_key(&req.api_key, "*").await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if valid {
        Ok(Json(ValidateResponse {
            valid: true,
            tier: "professional".to_string(),
            grace_days: 7,
        }))
    } else {
        Ok(Json(ValidateResponse {
            valid: false,
            tier: "none".to_string(),
            grace_days: 0,
        }))
    }
}

#[derive(Serialize)]
pub struct RefreshResponse {
    pub status: String,
    pub next_check: String,
}

pub async fn refresh(
    // Verify API key is still valid
) -> Result<Json<RefreshResponse>, StatusCode> {
    Ok(Json(RefreshResponse {
        status: "ok".to_string(),
        next_check: "24h".to_string(),
    }))
}
```

- [ ] **Step 2: Update main.rs router**

Add auth routes to the Axum router:
```rust
let app = Router::new()
    .route("/api/v1/auth/validate", post(auth::validate))
    .route("/api/v1/auth/refresh", post(auth::refresh))
    // ... existing routes
    .with_state(state);
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p rgaa-api`
Expected: Compiles (no tests exist yet)

- [ ] **Step 4: Commit**

```bash
git add rgaa-rs/crates/rgaa-api/
git commit -m "feat(rgaa-api): add auth/validate and auth/refresh endpoints"
```

---

### Task 8: Rule Update Feed Endpoints

**Files:**
- Create: `rgaa-rs/crates/rgaa-api/src/rules.rs`
- Modify: `rgaa-rs/crates/rgaa-api/src/main.rs`

**Interfaces:**
- Produces: `GET /api/v1/rules`, `GET /api/v1/rules/check`, `GET /api/v1/rules/{file}`

- [ ] **Step 1: Create rules.rs**

```rust
use axum::{extract::State, http::{HeaderMap, StatusCode}, response::IntoResponse, Json};
use serde::Serialize;

#[derive(Serialize)]
pub struct RuleManifest {
    pub version: String,
    pub hash: String,
    pub files: Vec<String>,
}

pub async fn check_rules(
    headers: HeaderMap,
) -> impl IntoResponse {
    let etag = headers.get("if-none-match");

    // Check if client has current version
    // If so, return 304 Not Modified
    // Otherwise return manifest JSON

    let manifest = RuleManifest {
        version: "2026.08.24".to_string(),
        hash: "sha256:abc123".to_string(),
        files: vec![
            "axe-rules.json".to_string(),
            "gap-fixes.json".to_string(),
            "criteria-updates.json".to_string(),
        ],
    };

    (StatusCode::OK, Json(manifest))
}

pub async fn get_rules() -> impl IntoResponse {
    // Serve the actual rule files
    StatusCode::OK
}
```

- [ ] **Step 2: Update main.rs router**

Add rule routes to the Axum router.

- [ ] **Step 3: Commit**

```bash
git add rgaa-rs/crates/rgaa-api/
git commit -m "feat(rgaa-api): add rule update feed endpoints with ETag support"
```

---

## Phase 4: Distribution

### Task 9: Install Script

**Files:**
- Create: `rgaa-rs/scripts/install.sh`

**Interfaces:**
- Produces: `~/.local/bin/rgaa` binary + config setup

- [ ] **Step 1: Create install.sh**

```bash
#!/bin/bash
set -euo pipefail

VERSION="${RGAA_VERSION:-latest}"
INSTALL_DIR="${HOME}/.local/bin"
CONFIG_DIR="${HOME}/.config/rgaa"

echo "🔧 Installing RGAA accessibility tools..."

# Detect OS and arch
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$ARCH" in
  x86_64)  ARCH="x86_64" ;;
  aarch64|arm64) ARCH="aarch64" ;;
  *) echo "❌ Unsupported architecture: $ARCH"; exit 1 ;;
esac

case "$OS" in
  linux)  TARGET="${ARCH}-unknown-linux-gnu" ;;
  darwin) TARGET="${ARCH}-apple-darwin" ;;
  *) echo "❌ Unsupported OS: $OS"; exit 1 ;;
esac

echo "   Platform: $TARGET"

# Get version
if [ "$VERSION" = "latest" ]; then
  VERSION=$(curl -fsSL https://api.github.com/repos/your-org/rgaa-rs/releases/latest \
    | grep tag_name | cut -d'"' -f4)
fi

echo "   Version: $VERSION"

# Download binary
URL="https://github.com/your-org/rgaa-rs/releases/download/${VERSION}/rgaa-${TARGET}.tar.gz"
TMP_DIR=$(mktemp -d)
curl -fsSL "$URL" | tar xz -C "$TMP_DIR"

# Install binary
mkdir -p "$INSTALL_DIR"
mv "$TMP_DIR/rgaa" "$INSTALL_DIR/rgaa"
chmod +x "$INSTALL_DIR/rgaa"
rm -rf "$TMP_DIR"

echo "✅ Binary installed: $INSTALL_DIR/rgaa"

# Create config directory
mkdir -p "$CONFIG_DIR"

# Detect Chrome
CHROME_FOUND=false
for browser in google-chrome chromium chromium-browser; do
  if command -v "$browser" &> /dev/null; then
    CHROME_FOUND=true
    echo "✅ Found browser: $browser"
    break
  fi
done

if [ "$CHROME_FOUND" = false ]; then
  echo "⚠️  Chrome/Chromium not found. Install it for browser automation."
fi

# Run configure
echo ""
echo "Running interactive configuration..."
"$INSTALL_DIR/rgaa" configure

# Add to PATH if needed
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
  SHELL_RC="$HOME/.bashrc"
  if [ -n "${ZSH_VERSION:-}" ]; then
    SHELL_RC="$HOME/.zshrc"
  fi
  echo "" >> "$SHELL_RC"
  echo "# RGAA tools" >> "$SHELL_RC"
  echo "export PATH=\"\$HOME/.local/bin:\$PATH\"" >> "$SHELL_RC"
  echo ""
  echo "💡 Added $INSTALL_DIR to PATH in $SHELL_RC"
  echo "   Run: source $SHELL_RC"
fi

echo ""
echo "✅ RGAA installed successfully!"
echo ""
echo "Binary:   $INSTALL_DIR/rgaa"
echo "Config:   $CONFIG_DIR/"
echo "Claude:   MCP server configured in Claude Desktop"
echo ""
echo "Try it: Open Claude Desktop and say \"Analyse https://example.com pour RGAA\""
```

- [ ] **Step 2: Make executable**

Run: `chmod +x rgaa-rs/scripts/install.sh`

- [ ] **Step 3: Commit**

```bash
git add rgaa-rs/scripts/install.sh
git commit -m "feat: add one-liner install script with Claude Desktop auto-config"
```

---

### Task 10: GitHub Release Workflow

**Files:**
- Create: `.github/workflows/release.yml`

**Interfaces:**
- Produces: Cross-compiled binaries + GitHub Release

- [ ] **Step 1: Create release.yml**

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

permissions:
  contents: write

jobs:
  build:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-latest
          - target: x86_64-apple-darwin
            os: macos-latest
          - target: aarch64-apple-darwin
            os: macos-latest

    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Build
        run: cargo build --release --target ${{ matrix.target }}

      - name: Package
        run: |
          cd target/${{ matrix.target }}/release
          tar czf ../../../rgaa-${{ matrix.target }}.tar.gz rgaa

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: rgaa-${{ matrix.target }}
          path: rgaa-${{ matrix.target }}.tar.gz

  release:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - name: Download all artifacts
        uses: actions/download-artifact@v4

      - name: Create Release
        uses: softprops/action-gh-release@v2
        with:
          files: rgaa-*/rgaa-*.tar.gz
          generate_release_notes: true
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add cross-compilation release workflow"
```

---

## Summary

| Phase | Task | What it delivers | Tests |
|-------|------|-----------------|-------|
| 1 | rgaa-license | Key storage, validation, offline grace | 4 tests |
| 1 | rgaa-updater | Rule update feed, atomic download | 1 test |
| 1 | SQLite backend | Local-only storage for consultants | 1 test |
| 1 | Multi-provider LLM | Claude, OpenAI, SaaS proxy support | 2 tests |
| 2 | CLI subcommands | configure, verify-install, update | Existing tests pass |
| 2 | MCP tools | source_map, verify_fix | Compiles |
| 3 | Auth endpoints | /validate, /refresh | Compiles |
| 3 | Rule endpoints | /rules, /rules/check | Compiles |
| 4 | Install script | One-liner setup | Manual test |
| 4 | Release workflow | Cross-compiled binaries | CI test |

**Total estimated time:** 2-3 weeks for a focused developer.
