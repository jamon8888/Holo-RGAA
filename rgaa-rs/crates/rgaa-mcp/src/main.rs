use rgaa_mcp::{ObscuraAnalyzeService, ObscuraGuidedService, RemediationServiceImpl, ToolServer};
use rmcp::{transport::io::stdio, ServiceExt};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    let mut bridge = rgaa_obscura::ObscuraBridge::new();
    bridge.start_server().await.map_err(std::io::Error::other)?;
    let bridge = Arc::new(bridge);
    let service = ToolServer::new(
        Arc::new(ObscuraAnalyzeService::new(Arc::clone(&bridge))),
        Arc::new(RemediationServiceImpl::default()),
        Arc::new(ObscuraGuidedService::new(bridge)),
    );
    service.serve(stdio()).await?.waiting().await?;
    Ok(())
}
