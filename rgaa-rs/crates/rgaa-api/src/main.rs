use rgaa_api::{build_app, AppState};
use rgaa_orchestrator::Orchestrator;
use rgaa_storage::PostgresStorage;
use std::sync::Arc;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

const HELP: &str = "\
rgaa-api - Axum HTTP API for RGAA audit operations

USAGE:
    rgaa-api [OPTIONS]

OPTIONS:
    -h, --help       Print this help and exit
    -V, --version    Print version and exit

ENVIRONMENT:
    DATABASE_URL     Postgres connection string (default: postgres://localhost/rgaa)
    LISTEN_ADDR      Address to bind the HTTP server on (default: 0.0.0.0:3000)

Runs until stopped (Ctrl-C or SIGTERM); does not exit on its own once serving.";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Handled before any I/O (DB connect, socket bind) so `--help`/`--version`
    // return immediately instead of falling through to the server startup
    // path, which never returns on its own.
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{HELP}");
                return Ok(());
            }
            "-V" | "--version" => {
                println!("rgaa-api {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            _ => {}
        }
    }

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost/rgaa".into());

    let storage: Arc<dyn rgaa_storage::Storage> =
        Arc::new(PostgresStorage::new(&database_url).await?);
    let orchestrator = Arc::new(Orchestrator::with_storage(storage.clone()));

    let state = AppState {
        orchestrator,
        storage,
    };

    let app = build_app(state);

    let addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".into());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Listening on {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}
