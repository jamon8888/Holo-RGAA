use rgaa_api::batch::{system_clock, BatchService, OrchestratorRunner, PostgresBatchStore};
use rgaa_api::{build_app, AppState};
use rgaa_license::LicenseGate;
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
    RGAA_LICENSE_KEY Licence token (else ~/.config/rgaa/license.toml; none = Free tier)
    RGAA_UPGRADE_URL URL returned in 402 responses as `upgrade_url`

Runs until stopped (Ctrl-C or SIGTERM); does not exit on its own once serving.";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Loads `.env` from the working directory (or any parent) so a local
    // checkout is configured by that one file. Variables already present in
    // the real environment always win, so a container or systemd unit keeps
    // precedence over a stray `.env`. Deliberately in the binary only: a
    // library must never reach for the filesystem behind its caller.
    let _ = dotenvy::dotenv();

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

    // Licence (#87): quota enforced (a server counter is not user-editable,
    // unlike the CLI's). Never fails at startup; invalid tokens are reported
    // per request as 401, tier/expiry refusals as 402, and quota as 429.
    let upgrade_url = std::env::var("RGAA_UPGRADE_URL")
        .ok()
        .filter(|u| !u.trim().is_empty());
    let license = Arc::new(LicenseGate::from_env(true, upgrade_url));
    let snapshot = license.snapshot();
    tracing::info!(
        tier = snapshot.tier.as_str(),
        status = snapshot.status.as_str(),
        "licence loaded"
    );

    // Postgres-backed, not in-memory: a batch is readable for 24h and the
    // API process is restarted far more often than that, so in-memory state
    // would hand clients a batch id that stops resolving after a deploy.
    let batches = BatchService::new(
        Arc::new(PostgresBatchStore::new(storage.pool().clone())),
        Arc::new(OrchestratorRunner::new(orchestrator.clone())),
        system_clock(),
        license.clone(),
    );

    let state = AppState {
        orchestrator,
        storage,
        batches,
    };

    let app = build_app(state, license)?;

    let addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".into());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Listening on {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}
