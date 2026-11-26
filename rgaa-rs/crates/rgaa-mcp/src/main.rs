use rgaa_mcp::{
    LazyObscuraBridge, NoOpStorageService, ObscuraAnalyzeService, ObscuraGuidedService,
    OrchestrationService, RegistryAnalysisService, RemediationServiceImpl, ToolServer,
};
use rmcp::{transport::io::stdio, ServiceExt};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Loads `.env` from the working directory (or any parent) so a local
    // checkout is configured by that one file. Variables already present in
    // the real environment always win, so a container or systemd unit keeps
    // precedence over a stray `.env`. Deliberately in the binary only: a
    // library must never reach for the filesystem behind its caller.
    let _ = dotenvy::dotenv();

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
    // Own CDP port, allocated on the first browser tool call: a failed
    // allocation must not stop browser-free tools from being served.
    let bridge = Arc::new(LazyObscuraBridge::new(
        rgaa_obscura::ObscuraBridge::from_env().with_deferred_ephemeral_port(),
    ));
    let service = ToolServer::new_with_analysis(
        Arc::new(ObscuraAnalyzeService::new(Arc::clone(&bridge))),
        Arc::new(RemediationServiceImpl::default()),
        Arc::new(ObscuraGuidedService::new(Arc::clone(&bridge))),
        Arc::new(OrchestrationService::new()),
        Arc::new(NoOpStorageService),
        Arc::new(RegistryAnalysisService::with_bridge(bridge)),
    );
    service.serve(stdio()).await?.waiting().await?;
    Ok(())
}
