use anyhow::Result;
use std::path::PathBuf;

mod automatability;
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

    tracing::info!("Validating axe-core → RGAA mapping...");
    let existing_mapping = parse::load_existing_mapping()?;
    let validated = validate::validate_mapping(&axe_rules, &existing_mapping);
    let mapping_json = serde_json::to_string_pretty(&validated)?;
    std::fs::write(out_dir.join("axe_mapping.json"), &mapping_json)?;
    tracing::info!("Saved {} validated mappings", validated.len());

    tracing::info!("Analyzing automatability from criteres.json...");
    let automatability = automatability::analyze_automatability()?;
    let auto_json = serde_json::to_string_pretty(&automatability)?;
    std::fs::write(out_dir.join("automatable_criteres.json"), &auto_json)?;
    tracing::info!(
        "Automatability: {} fully, {} partial, {} not automatable",
        automatability.fully_automatable,
        automatability.partially_automatable,
        automatability.not_automatable
    );

    tracing::info!("Done.");
    Ok(())
}
