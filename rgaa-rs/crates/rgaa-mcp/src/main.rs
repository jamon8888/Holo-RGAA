use rgaa_mcp::{
    LazyObscuraBridge, NoOpStorageService, ObscuraAnalyzeService, ObscuraGuidedService,
    OrchestrationService, RemediationServiceImpl, ToolServer,
};
use rmcp::{transport::io::stdio, ServiceExt};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Help and version must not start the stdio server.
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("rgaa-mcp: RGAA MCP server over stdio");
        println!("Usage: rgaa-mcp [--help] [--version]");
        return Ok(());
    }
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("rgaa-mcp {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    let bridge = Arc::new(LazyObscuraBridge::new(
        rgaa_obscura::ObscuraBridge::from_env(),
    ));
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
