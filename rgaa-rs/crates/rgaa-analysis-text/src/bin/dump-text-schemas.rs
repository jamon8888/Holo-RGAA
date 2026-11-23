//! Dump des schémas P-08 sur stdout (hors protocole MCP, comme P-00).

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let document = rgaa_analysis_text::schema::schemas()?;
    println!("{}", serde_json::to_string_pretty(&document)?);
    Ok(())
}
