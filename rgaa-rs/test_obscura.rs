use rgaa_obscura::ObscuraBridge;

#[tokio::main]
async fn main() {
    let bridge = ObscuraBridge::new();
    
    println!("Testing extract_page_context...");
    match bridge.extract_page_context("https://example.com").await {
        Ok(ctx) => println!("Context: {}", serde_json::to_string_pretty(&ctx).unwrap()),
        Err(e) => println!("Error: {}", e),
    }
}