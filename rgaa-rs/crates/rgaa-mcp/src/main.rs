use rgaa_mcp::{
    LazyObscuraBridge, NoOpStorageService, ObscuraAnalyzeService, ObscuraGuidedService,
    OrchestrationService, RemediationServiceImpl, ToolServer,
};
use rmcp::{transport::io::stdio, ServiceExt};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    let bridge = Arc::new(LazyObscuraBridge::new(rgaa_obscura::ObscuraBridge::new()));
    let service = ToolServer::new(
        Arc::new(ObscuraAnalyzeService::new(Arc::clone(&bridge))),
        Arc::new(RemediationServiceImpl::default()),
        Arc::new(ObscuraGuidedService::new(bridge)),
        Arc::new(OrchestrationService::new()),
        Arc::new(NoOpStorageService),
    );
    service.serve(stdio()).await?.waiting().await?;
    Ok(())
}
