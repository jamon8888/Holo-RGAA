//! Build/deploy-time seeding of the versioned regulatory-corpus RAG index
//! (`rag_referentiel`) from the RGAA criteria catalog — ticket #128.
//!
//! Run this whenever the referentiel version changes (a `criteres.json`
//! update, or a new embedding model): it rebuilds the whole table from
//! [`rgaa_core::RgaaCatalog`], never migrating rows in place.
//!
//! ```text
//! LANCEDB_PATH=./data/lancedb cargo run -p rgaa-agent --bin seed_referentiel
//! ```
//!
//! `LANCEDB_PATH` defaults to `./data/lancedb` (matching
//! [`rgaa_agent::config::AgentConfig::default`]'s `lancedb_path`) if unset.

use rgaa_agent::config::AgentConfig;
use rgaa_agent::embeddings::HybridEmbeddingProvider;
use rgaa_agent::rag::{RagStore, ReferentielSeeder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let lancedb_path =
        std::env::var("LANCEDB_PATH").unwrap_or_else(|_| "./data/lancedb".to_string());

    // `AgentConfig::default()` already selects FastEmbed all-MiniLM-L6-v2
    // at 384 dimensions — the embedded, locally-run model this index is
    // seeded and queried with (see [`REFERENTIEL_VERSION`]'s doc comment:
    // one embedding model equals one index version).
    let embedder = HybridEmbeddingProvider::new(&AgentConfig::default())?;

    let store = RagStore::open(&lancedb_path).await?;
    let seeder = ReferentielSeeder::new(&embedder);

    println!(
        "Seeding referentiel index (version {}) at {lancedb_path} …",
        rgaa_agent::rag::REFERENTIEL_VERSION
    );
    let count = seeder.seed(&store).await?;
    println!("Seeded {count} rows.");

    Ok(())
}
