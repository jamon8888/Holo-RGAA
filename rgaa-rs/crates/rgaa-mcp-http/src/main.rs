use clap::Parser;
use rgaa_mcp_http::McpServerArgs;

/// Standalone entry point for the same server `rgaa mcp-server` starts.
#[derive(Debug, Parser)]
#[command(name = "rgaa-mcp-http", version, about = "MCP HTTP/SSE transport")]
struct Cli {
    #[command(flatten)]
    args: McpServerArgs,
}

#[tokio::main]
async fn main() -> Result<(), rgaa_mcp_http::ServeError> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    rgaa_mcp_http::run(cli.args).await
}
