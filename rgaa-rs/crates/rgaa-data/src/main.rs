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
