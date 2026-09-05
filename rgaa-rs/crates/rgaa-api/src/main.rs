use rgaa_api::{build_app, AppState};
use rgaa_orchestrator::Orchestrator;
use rgaa_storage::PostgresStorage;
use std::sync::Arc;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
