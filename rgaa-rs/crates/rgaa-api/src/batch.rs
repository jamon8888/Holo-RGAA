//! Multi-URL batch audits over REST (#167): create a batch, poll its
//! per-URL progress, read the aggregated result.
//!
//! A batch outlives the request that created it — a 1000-URL batch takes
//! hours, far past any HTTP timeout — so creation only records the batch
//! and returns its id; the audits run on a detached task that reports each
//! URL's outcome back into the store as it lands.
//!
//! The fan-out itself is [`rgaa_orchestrator::Orchestrator::run_batch_observed`],
//! not a second scheduler living here: this module owns batch *state*, the
//! orchestrator owns batch *execution* and its concurrency bound.

use async_trait::async_trait;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rgaa_core::{AuditResult, CrawlConfig};
use rgaa_orchestrator::Orchestrator;

/// How long a batch stays readable after creation. Batch records hold every
/// audit summary in the lot, so they are kept bounded by age rather than
/// forever; 24h is the window the API contract promises (#167).
pub const BATCH_TTL_HOURS: i64 = 24;

/// Refused above this many URLs in one batch. Matches the pipeline's own
/// documented operating point (#42: 1000-audit batches) — a request for
/// more would be accepted and then never finish inside its own TTL.
pub const MAX_BATCH_URLS: usize = 1000;

/// Supplies "now". Injected rather than calling [`Utc::now`] directly so
/// expiry can be tested by moving the clock instead of sleeping 24 hours.
pub type Clock = Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>;

/// Real wall clock, used everywhere outside tests.
#[must_use]
pub fn system_clock() -> Clock {
    Arc::new(Utc::now)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UrlState {
    Pending,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchState {
    Running,
    Completed,
}

/// One URL's slot in a batch. `audit_id` is the handle into the ordinary
/// audit storage, so a client can fetch the full `AuditResult` through the
/// existing endpoints without the batch record duplicating it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlProgress {
    pub url: String,
    pub state: UrlState,
    pub audit_id: Option<String>,
    pub taux_global: Option<f64>,
    pub etat_conformite: Option<String>,
    pub error: Option<String>,
}

/// The whole persisted state of a batch. Deliberately one self-contained
/// serializable value: the store only has to round-trip it, and a
/// subscriber (#168's SSE stream / webhook) can diff two snapshots of it to
/// derive progress events without this module growing an event log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRecord {
    pub batch_id: String,
    pub state: BatchState,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub config: CrawlConfig,
    pub urls: Vec<UrlProgress>,
}

impl BatchRecord {
    /// Builds a fresh record with every URL pending and `expires_at` set
    /// from `now`, so the TTL is anchored to the injected clock rather than
    /// to wall time.
    #[must_use]
    pub fn new(
        batch_id: String,
        urls: Vec<String>,
        config: CrawlConfig,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            batch_id,
            state: BatchState::Running,
            created_at: now,
            expires_at: now + ChronoDuration::hours(BATCH_TTL_HOURS),
            config,
            urls: urls
                .into_iter()
                .map(|url| UrlProgress {
                    url,
                    state: UrlState::Pending,
                    audit_id: None,
                    taux_global: None,
                    etat_conformite: None,
                    error: None,
                })
                .collect(),
        }
    }

    #[must_use]
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }

    fn completed_count(&self) -> usize {
        self.urls
            .iter()
            .filter(|u| u.state != UrlState::Pending)
            .count()
    }

    /// Records one URL's outcome and promotes the batch to `Completed` once
    /// nothing is pending. Applied inside the store's update so two audits
    /// finishing at the same time cannot lose each other's write.
    pub fn record_outcome(&mut self, url: &str, outcome: Result<&AuditResult, &str>) {
        if let Some(slot) = self.urls.iter_mut().find(|u| u.url == url) {
            match outcome {
                Ok(audit) => {
                    slot.state = UrlState::Completed;
                    slot.audit_id = Some(audit.audit_id.clone());
                    slot.taux_global = Some(audit.taux_global);
                    slot.etat_conformite = Some(audit.etat_conformite.clone());
                }
                Err(e) => {
                    slot.state = UrlState::Failed;
                    slot.error = Some(e.to_string());
                }
            }
        }
        if self.completed_count() == self.urls.len() {
            self.state = BatchState::Completed;
        }
    }

    /// Marks every still-pending URL failed. Needed because the orchestrator
    /// can fail before it ever reaches the per-URL fan-out (no browser, bad
    /// agent config): without this the batch would sit in `Running` until it
    /// expired, with no way for a client to tell it from a slow audit.
    fn fail_pending(&mut self, error: &str) {
        for slot in self
            .urls
            .iter_mut()
            .filter(|u| u.state == UrlState::Pending)
        {
            slot.state = UrlState::Failed;
            slot.error = Some(error.to_string());
        }
        self.state = BatchState::Completed;
    }
}

#[derive(Debug)]
pub struct BatchStoreError(pub String);

impl std::fmt::Display for BatchStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for BatchStoreError {}

/// Persistence seam for batch state.
///
/// `update` takes a mutator rather than a whole record so the backend can
/// serialize concurrent writes itself (the Postgres impl reads the row
/// `FOR UPDATE`); a read-then-write from the caller would drop outcomes
/// whenever two URLs of the same batch finished together.
#[async_trait]
// `for<'r>` on `mutate` is not decoration. Without it, `async_trait`'s
// desugaring ties the closure's `&mut BatchRecord` to the method's own
// `'async_trait` lifetime instead of making it higher-ranked, and the
// Postgres impl then cannot borrow the record again after calling
// `mutate` — "cannot borrow `record` as immutable because it is also
// borrowed as mutable", on code that is plainly sequential.
pub trait BatchStore: Send + Sync {
    async fn insert(&self, record: &BatchRecord) -> Result<(), BatchStoreError>;
    async fn load(&self, batch_id: &str) -> Result<Option<BatchRecord>, BatchStoreError>;
    async fn update(
        &self,
        batch_id: &str,
        mutate: &(dyn for<'r> Fn(&'r mut BatchRecord) + Send + Sync),
    ) -> Result<(), BatchStoreError>;
    /// Drops records whose `expires_at` has passed. Reclaims storage; it is
    /// not what enforces expiry — [`BatchRecord::is_expired`] is, on every
    /// read, so a batch is unreadable at T+24h whether or not a purge ran.
    async fn purge_expired(&self, now: DateTime<Utc>) -> Result<u64, BatchStoreError>;
}

/// Process-local store. Used by tests and by any deployment run without a
/// database; batch state does not survive a restart here, so production
/// wiring uses [`PostgresBatchStore`].
#[derive(Default)]
pub struct InMemoryBatchStore {
    records: Mutex<HashMap<String, BatchRecord>>,
}

impl InMemoryBatchStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, BatchRecord>> {
        // A panic inside a mutator would otherwise poison the map and take
        // down every later batch request; the map itself stays consistent
        // because mutators only ever edit one record they already hold.
        self.records.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[async_trait]
impl BatchStore for InMemoryBatchStore {
    async fn insert(&self, record: &BatchRecord) -> Result<(), BatchStoreError> {
        self.lock().insert(record.batch_id.clone(), record.clone());
        Ok(())
    }

    async fn load(&self, batch_id: &str) -> Result<Option<BatchRecord>, BatchStoreError> {
        Ok(self.lock().get(batch_id).cloned())
    }

    async fn update(
        &self,
        batch_id: &str,
        mutate: &(dyn for<'r> Fn(&'r mut BatchRecord) + Send + Sync),
    ) -> Result<(), BatchStoreError> {
        match self.lock().get_mut(batch_id) {
            Some(record) => {
                mutate(record);
                Ok(())
            }
            None => Err(BatchStoreError(format!("unknown batch {batch_id}"))),
        }
    }

    async fn purge_expired(&self, now: DateTime<Utc>) -> Result<u64, BatchStoreError> {
        let mut records = self.lock();
        let before = records.len();
        records.retain(|_, r| !r.is_expired(now));
        Ok((before - records.len()) as u64)
    }
}

/// Postgres-backed store: the whole record as one JSONB document, with the
/// two fields the server queries on (`batch_id`, `expires_at`) lifted into
/// columns. The record's shape is owned by this module and read by nothing
/// else in SQL, so a column-per-field schema would only add migrations
/// every time a progress field is added.
pub struct PostgresBatchStore {
    pool: PgPool,
}

impl PostgresBatchStore {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn store_err(e: impl std::fmt::Display) -> BatchStoreError {
    BatchStoreError(e.to_string())
}

#[async_trait]
impl BatchStore for PostgresBatchStore {
    async fn insert(&self, record: &BatchRecord) -> Result<(), BatchStoreError> {
        let json = serde_json::to_value(record).map_err(store_err)?;
        sqlx::query(
            "INSERT INTO audit_batches (batch_id, record, created_at, expires_at) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(&record.batch_id)
        .bind(json)
        .bind(record.created_at)
        .bind(record.expires_at)
        .execute(&self.pool)
        .await
        .map_err(store_err)?;
        Ok(())
    }

    async fn load(&self, batch_id: &str) -> Result<Option<BatchRecord>, BatchStoreError> {
        let row: Option<(sqlx::types::Json<BatchRecord>,)> =
            sqlx::query_as("SELECT record FROM audit_batches WHERE batch_id = $1")
                .bind(batch_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(store_err)?;
        Ok(row.map(|(r,)| r.0))
    }

    async fn update(
        &self,
        batch_id: &str,
        mutate: &(dyn for<'r> Fn(&'r mut BatchRecord) + Send + Sync),
    ) -> Result<(), BatchStoreError> {
        // `FOR UPDATE` inside the transaction: two URLs of the same batch
        // finishing concurrently would otherwise both read the pre-update
        // document and the second write would erase the first's progress.
        let mut tx = self.pool.begin().await.map_err(store_err)?;
        let row: Option<(sqlx::types::Json<BatchRecord>,)> =
            sqlx::query_as("SELECT record FROM audit_batches WHERE batch_id = $1 FOR UPDATE")
                .bind(batch_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(store_err)?;
        let Some((record,)) = row else {
            return Err(BatchStoreError(format!("unknown batch {batch_id}")));
        };
        let mut record = record.0;
        mutate(&mut record);
        let json = serde_json::to_value(&record).map_err(store_err)?;
        sqlx::query("UPDATE audit_batches SET record = $2 WHERE batch_id = $1")
            .bind(batch_id)
            .bind(json)
            .execute(&mut *tx)
            .await
            .map_err(store_err)?;
        tx.commit().await.map_err(store_err)?;
        Ok(())
    }

    async fn purge_expired(&self, now: DateTime<Utc>) -> Result<u64, BatchStoreError> {
        let result = sqlx::query("DELETE FROM audit_batches WHERE expires_at <= $1")
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(store_err)?;
        Ok(result.rows_affected())
    }
}

/// Executes the audits for a batch, reporting each URL's outcome as it
/// lands. A trait, not the concrete [`Orchestrator`], so the HTTP layer can
/// be tested end-to-end without a browser, an LLM, or a database.
#[async_trait]
pub trait BatchRunner: Send + Sync {
    async fn run(
        &self,
        urls: Vec<String>,
        config: CrawlConfig,
        on_url: rgaa_orchestrator::BatchObserver,
    ) -> Result<(), String>;
}

/// Production runner: delegates straight to the existing batch pipeline and
/// its concurrency bound.
pub struct OrchestratorRunner {
    orchestrator: Arc<Orchestrator>,
}

impl OrchestratorRunner {
    #[must_use]
    pub fn new(orchestrator: Arc<Orchestrator>) -> Self {
        Self { orchestrator }
    }
}

#[async_trait]
impl BatchRunner for OrchestratorRunner {
    async fn run(
        &self,
        urls: Vec<String>,
        config: CrawlConfig,
        on_url: rgaa_orchestrator::BatchObserver,
    ) -> Result<(), String> {
        self.orchestrator
            .run_batch_observed(&urls, &config, on_url)
            .await
            .map(|_| ())
    }
}

/// Everything the batch endpoints need, independent of [`crate::AppState`]
/// so the batch router can be built and served on its own.
#[derive(Clone)]
pub struct BatchService {
    store: Arc<dyn BatchStore>,
    runner: Arc<dyn BatchRunner>,
    clock: Clock,
}

impl BatchService {
    #[must_use]
    pub fn new(store: Arc<dyn BatchStore>, runner: Arc<dyn BatchRunner>, clock: Clock) -> Self {
        Self {
            store,
            runner,
            clock,
        }
    }

    fn now(&self) -> DateTime<Utc> {
        (self.clock)()
    }

    /// Loads a batch, treating an expired one as absent.
    ///
    /// Expiry is enforced here rather than by a sweeper task: a sweeper that
    /// has not run yet (or died) would keep serving a batch past its
    /// advertised `expires_at`, and the 24h guarantee would then depend on
    /// the sweeper's liveness instead of on the record itself.
    async fn load_live(&self, batch_id: &str) -> Result<BatchRecord, StatusCode> {
        let record = self
            .store
            .load(batch_id)
            .await
            .map_err(|e| {
                tracing::error!(batch_id, error = %e, "batch store read failed");
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .ok_or(StatusCode::NOT_FOUND)?;

        if record.is_expired(self.now()) {
            // 410, not 404: the id was valid and the client's own poll loop
            // should stop rather than retry a batch that will never return.
            return Err(StatusCode::GONE);
        }
        Ok(record)
    }
}

#[derive(Deserialize)]
pub struct CreateBatchRequest {
    pub urls: Vec<String>,
    #[serde(default)]
    pub config: Option<CrawlConfig>,
}

#[derive(Serialize)]
pub struct CreateBatchResponse {
    pub batch_id: String,
    pub state: BatchState,
    pub url_count: usize,
    pub expires_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct BatchStatusResponse {
    pub batch_id: String,
    pub state: BatchState,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub pending: usize,
    pub urls: Vec<UrlProgress>,
}

#[derive(Serialize)]
pub struct BatchResultsResponse {
    pub batch_id: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub audited: usize,
    pub failed: usize,
    /// Mean `taux_global` over the URLs that produced a result. `None` when
    /// every URL failed — an average of nothing is not zero conformity.
    pub taux_global_moyen: Option<f64>,
    pub urls: Vec<UrlProgress>,
}

/// Public because it appears in a public handler's return type; clients
/// only ever see it as `{"error": "..."}`.
#[derive(Serialize)]
pub struct ErrorBody {
    pub error: String,
}

fn bad_request(message: &str) -> (StatusCode, Json<ErrorBody>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorBody {
            error: message.to_string(),
        }),
    )
}

pub async fn create_batch(
    State(service): State<BatchService>,
    Json(payload): Json<CreateBatchRequest>,
) -> Result<(StatusCode, Json<CreateBatchResponse>), (StatusCode, Json<ErrorBody>)> {
    if payload.urls.is_empty() {
        return Err(bad_request("urls must not be empty"));
    }
    if payload.urls.len() > MAX_BATCH_URLS {
        return Err(bad_request(&format!(
            "at most {MAX_BATCH_URLS} urls per batch"
        )));
    }

    let now = service.now();

    // Opportunistic, on the create path rather than on a timer: it is the
    // only path that grows the table, and piggybacking keeps the server free
    // of a background task whose failure would be invisible.
    if let Err(e) = service.store.purge_expired(now).await {
        tracing::warn!(error = %e, "expired batch purge failed; expiry still enforced on read");
    }

    let record = BatchRecord::new(
        uuid::Uuid::new_v4().to_string(),
        payload.urls.clone(),
        payload.config.unwrap_or_default(),
        now,
    );

    service.store.insert(&record).await.map_err(|e| {
        tracing::error!(error = %e, "batch store insert failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorBody {
                error: "failed to persist batch".to_string(),
            }),
        )
    })?;

    let response = CreateBatchResponse {
        batch_id: record.batch_id.clone(),
        state: record.state,
        url_count: record.urls.len(),
        expires_at: record.expires_at,
    };

    spawn_batch(service.clone(), record);

    // 202: the batch is recorded and will run, but nothing has been audited
    // yet — a 201 would imply the results resource is already readable.
    Ok((StatusCode::ACCEPTED, Json(response)))
}

/// Detaches the audit work from the creating request. The task owns only
/// `Arc`s, so it keeps running (and keeps reporting progress into the
/// store) after the HTTP response has been sent.
fn spawn_batch(service: BatchService, record: BatchRecord) {
    let urls: Vec<String> = record.urls.iter().map(|u| u.url.clone()).collect();
    let batch_id = record.batch_id.clone();
    let config = record.config.clone();

    tokio::spawn(async move {
        // The orchestrator's observer is synchronous while a store write is
        // not. Outcomes go over a channel to one writer task instead of a
        // detached write per outcome: a detached write can still be in
        // flight when the run returns, which would let the results endpoint
        // see a batch that is "finished" but missing its last URLs.
        let (tx, mut rx) =
            tokio::sync::mpsc::unbounded_channel::<(String, Result<AuditResult, String>)>();

        let writer_store = Arc::clone(&service.store);
        let writer_batch_id = batch_id.clone();
        let writer = tokio::spawn(async move {
            while let Some((url, outcome)) = rx.recv().await {
                let update = writer_store
                    .update(&writer_batch_id, &move |record: &mut BatchRecord| {
                        record.record_outcome(&url, outcome.as_ref().map_err(String::as_str));
                    })
                    .await;
                if let Err(e) = update {
                    tracing::error!(batch_id = writer_batch_id, error = %e, "failed to record batch progress");
                }
            }
        });

        let observer: rgaa_orchestrator::BatchObserver =
            Arc::new(move |url: &str, outcome: Result<&AuditResult, &str>| {
                let _ = tx.send((url.to_string(), outcome.cloned().map_err(str::to_string)));
            });

        let run = service.runner.run(urls, config, observer).await;

        // Dropping the last sender ends the writer loop; awaiting it means
        // every reported outcome is durable before the batch is settled.
        let _ = writer.await;

        let failure = match run {
            Ok(()) => None,
            Err(e) => {
                tracing::error!(batch_id, error = %e, "batch run failed");
                Some(e)
            }
        };

        // Covers both a run that never reached the fan-out and a run that
        // returned without reporting some URL: either way the batch must
        // not be left `Running` until it expires.
        let reason = failure.unwrap_or_else(|| "audit produced no result".to_string());
        let _ = service
            .store
            .update(&batch_id, &move |record: &mut BatchRecord| {
                if record.state != BatchState::Completed {
                    record.fail_pending(&reason);
                }
            })
            .await;
    });
}

pub async fn get_batch_status(
    State(service): State<BatchService>,
    Path(batch_id): Path<String>,
) -> Result<Json<BatchStatusResponse>, StatusCode> {
    let record = service.load_live(&batch_id).await?;

    let completed = count_state(&record, UrlState::Completed);
    let failed = count_state(&record, UrlState::Failed);
    let pending = count_state(&record, UrlState::Pending);

    Ok(Json(BatchStatusResponse {
        batch_id: record.batch_id.clone(),
        state: record.state,
        created_at: record.created_at,
        expires_at: record.expires_at,
        total: record.urls.len(),
        completed,
        failed,
        pending,
        urls: record.urls,
    }))
}

pub async fn get_batch_results(
    State(service): State<BatchService>,
    Path(batch_id): Path<String>,
) -> Result<Json<BatchResultsResponse>, StatusCode> {
    let record = service.load_live(&batch_id).await?;

    if record.state != BatchState::Completed {
        // 409, not an empty 200: these are the *final* aggregated results,
        // and a partial average silently read as final is the failure mode
        // worth refusing. Progress lives on the status endpoint.
        return Err(StatusCode::CONFLICT);
    }

    let scores: Vec<f64> = record.urls.iter().filter_map(|u| u.taux_global).collect();
    let taux_global_moyen = if scores.is_empty() {
        None
    } else {
        Some(scores.iter().sum::<f64>() / scores.len() as f64)
    };

    Ok(Json(BatchResultsResponse {
        batch_id: record.batch_id.clone(),
        created_at: record.created_at,
        expires_at: record.expires_at,
        audited: count_state(&record, UrlState::Completed),
        failed: count_state(&record, UrlState::Failed),
        taux_global_moyen,
        urls: record.urls,
    }))
}

fn count_state(record: &BatchRecord, state: UrlState) -> usize {
    record.urls.iter().filter(|u| u.state == state).count()
}

/// The three batch routes, already bound to their own state.
///
/// Returns a stateless `Router` so callers can merge it into a larger app
/// (and wrap it in auth) without [`BatchService`] leaking into that app's
/// state type — and so tests can serve exactly these routes.
pub fn batch_router(service: BatchService) -> Router {
    Router::new()
        .route("/v1/batches", post(create_batch))
        .route("/v1/batches/:id", get(get_batch_status))
        .route("/v1/batches/:id/results", get(get_batch_results))
        .with_state(service)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record_at(now: DateTime<Utc>) -> BatchRecord {
        BatchRecord::new(
            "b1".to_string(),
            vec!["https://a.test".to_string(), "https://b.test".to_string()],
            CrawlConfig::default(),
            now,
        )
    }

    #[test]
    fn a_batch_expires_exactly_24_hours_after_creation() {
        let now = Utc::now();
        let record = record_at(now);
        assert!(!record.is_expired(now + ChronoDuration::hours(23)));
        assert!(record.is_expired(now + ChronoDuration::hours(24)));
    }

    #[test]
    fn a_batch_completes_only_once_every_url_has_an_outcome() {
        let mut record = record_at(Utc::now());
        record.record_outcome("https://a.test", Err("boom"));
        assert_eq!(record.state, BatchState::Running);
        record.record_outcome("https://b.test", Err("boom"));
        assert_eq!(record.state, BatchState::Completed);
    }

    #[test]
    fn a_runner_failure_before_fan_out_fails_the_pending_urls() {
        let mut record = record_at(Utc::now());
        record.fail_pending("no browser");
        assert_eq!(record.state, BatchState::Completed);
        assert!(record.urls.iter().all(|u| u.state == UrlState::Failed));
    }
}
