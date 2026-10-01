//! Retrieve external remediation guidance for one RGAA criterion.
//!
//! ```bash
//! export EXA_API_KEY=...
//! cargo run -p rgaa-exa --example criterion_guidance -- 1.3 "img sans attribut alt"
//! ```

use rgaa_core::RgaaCriteria;
use rgaa_exa::{guidance, remediation_guidance, ExaClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber_init();

    let mut args = std::env::args().skip(1);
    let id = args.next().unwrap_or_else(|| "1.3".to_string());
    let context = args.next();

    let criterion = RgaaCriteria::all()
        .iter()
        .find(|c| c.id == id)
        .ok_or_else(|| format!("critère RGAA inconnu : {id}"))?;

    let client = ExaClient::from_env()?;
    let refs = remediation_guidance(&client, criterion, context.as_deref()).await?;

    if refs.is_empty() {
        println!("Aucune source externe exploitable pour le critère {id}.");
        return Ok(());
    }
    print!("{}", guidance::render(&refs));
    Ok(())
}

fn tracing_subscriber_init() {
    // Kept dependency-free: the example only needs the crate's own output.
}
