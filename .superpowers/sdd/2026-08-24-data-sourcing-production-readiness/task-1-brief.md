# Task 1: Create `rgaa-data` build crate scaffold

**Files:**
- Create: `rgaa-rs/crates/rgaa-data/Cargo.toml`
- Create: `rgaa-rs/crates/rgaa-data/src/main.rs`
- Modify: `rgaa-rs/Cargo.toml` (add `rgaa-data` to workspace members)

**Interfaces:**
- Produces: Binary that fetches + parses RGAA data sources, outputs JSON files

## Steps

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "rgaa-data"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "rgaa-data"
path = "src/main.rs"

[dependencies]
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
```

- [ ] **Step 2: Create src/main.rs with fetch skeleton**

```rust
use anyhow::Result;
use std::path::PathBuf;

mod fetch;
mod parse;
mod validate;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let out_dir = PathBuf::from("crates/rgaa-core/data/rgaa-4.1.2");
    std::fs::create_dir_all(&out_dir)?;

    tracing::info!("Fetching axe-core rule descriptions...");
    let axe_rules = fetch::axe_core_rules().await?;
    let axe_json = serde_json::to_string_pretty(&axe_rules)?;
    std::fs::write(out_dir.join("axe_rules.json"), &axe_json)?;
    tracing::info!("Saved {} axe-core rules", axe_rules.len());

    tracing::info!("Done.");
    Ok(())
}
```

- [ ] **Step 3: Add `rgaa-data` to workspace members in root Cargo.toml**

Add `"crates/rgaa-data"` to the `members` list in `/home/jamin/Documents/Holo-RGAA/rgaa-rs/Cargo.toml`.

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p rgaa-data`
Expected: compiles with warnings about unused modules

- [ ] **Step 5: Commit**

```bash
git add rgaa-rs/crates/rgaa-data/ rgaa-rs/Cargo.toml
git commit -m "Add rgaa-data build crate scaffold for automated data sourcing"
```
