//! Dumps the proof and `mesures` JSON Schemas as one JSON document on stdout.
//!
//! Kept as a binary rather than a library call for the same reason as
//! `dump-tool-schemas`: the stdio MCP server writes protocol frames to stdout,
//! so schema output must never share that stream.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let document = rgaa_analysis::schema::schemas()?;
    println!("{}", serde_json::to_string_pretty(&document)?);
    Ok(())
}
