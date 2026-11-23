//! Dumps the published matrices as the JSON shared with the P-09 review page.
//!
//! (cd rgaa-rs && cargo run -p rgaa-analysis-colorvision --bin dump-colorvision-matrices) \
//!   > types/colorvision-matrices.json

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        serde_json::to_string_pretty(&rgaa_analysis_colorvision::matrices::matrices_document())?
    );
    Ok(())
}
