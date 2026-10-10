//! Batch state persistence (#167) against a real Postgres, so the AC
//! "persistance d'état du lot" is backed by the store the server actually
//! runs with — the in-memory store used by the HTTP tests proves the state
//! machine, not durability.
//!
//! Requires `DATABASE_URL` pointing at a **disposable** database (rows are
//! created and deleted). Skipped when unset, like `union_schema.rs`.

use chrono::{Duration as ChronoDuration, Utc};
use rgaa_api::batch::{BatchRecord, BatchState, BatchStore, PostgresBatchStore, UrlState};
use rgaa_core::{AuditResult, CrawlConfig};
use rgaa_storage::PostgresStorage;

fn audit(url: &str) -> AuditResult {
    AuditResult {
        audit_id: format!("audit-for-{url}"),
        url: url.to_string(),
        pages: vec![],
        total_criteria: 106,
        passed: 50,
        failed: 10,
        na: 46,
        overall_compliance: 77.0,
        taux_global: 77.0,
        coverage_percent: 56.6,
        etat_conformite: "partielle".to_string(),
        duration_ms: 1,
        audit_complete: false,
    }
}

#[tokio::test]
async fn batch_state_survives_a_round_trip_through_postgres() {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping postgres batch store test: DATABASE_URL not set");
        return;
    };

    // Goes through `PostgresStorage::new` rather than raw `PgPool::connect`
    // so the batch table is provisioned by the same migration path the
    // server uses at startup — a missing migration must fail here, not in
    // production.
    let storage = PostgresStorage::new(&url)
        .await
        .expect("connect and provision schema");
    let store = PostgresBatchStore::new(storage.pool().clone());

    let now = Utc::now();
    let batch_id = uuid::Uuid::new_v4().to_string();
    let record = BatchRecord::new(
        batch_id.clone(),
        vec!["https://a.test".to_string(), "https://b.test".to_string()],
        CrawlConfig::default(),
        now,
    );
    store.insert(&record).await.expect("insert");

    let loaded = store
        .load(&batch_id)
        .await
        .expect("load")
        .expect("inserted batch is readable");
    assert_eq!(loaded.urls.len(), 2);
    assert_eq!(loaded.state, BatchState::Running);
    assert_eq!(loaded.expires_at, record.expires_at);

    // Two outcomes applied as separate updates: the second must not erase
    // the first, which is the whole reason `update` mutates under
    // `SELECT ... FOR UPDATE` instead of overwriting a caller-held copy.
    let first = audit("https://a.test");
    store
        .update(&batch_id, &move |r: &mut BatchRecord| {
            r.record_outcome("https://a.test", Ok(&first));
        })
        .await
        .expect("update first");
    store
        .update(&batch_id, &|r: &mut BatchRecord| {
            r.record_outcome("https://b.test", Err("navigation timed out"));
        })
        .await
        .expect("update second");

    let settled = store
        .load(&batch_id)
        .await
        .expect("load")
        .expect("batch still readable");
    assert_eq!(settled.state, BatchState::Completed);
    assert_eq!(settled.urls[0].state, UrlState::Completed);
    assert_eq!(settled.urls[0].taux_global, Some(77.0));
    assert_eq!(settled.urls[1].state, UrlState::Failed);
    assert_eq!(
        settled.urls[1].error.as_deref(),
        Some("navigation timed out")
    );

    // Purge at T+24h: the record is gone, and purging is scoped by
    // `expires_at`, not "everything older than the newest row".
    let fresh_id = uuid::Uuid::new_v4().to_string();
    store
        .insert(&BatchRecord::new(
            fresh_id.clone(),
            vec!["https://c.test".to_string()],
            CrawlConfig::default(),
            now + ChronoDuration::hours(23),
        ))
        .await
        .expect("insert fresh");

    store
        .purge_expired(now + ChronoDuration::hours(24))
        .await
        .expect("purge");
    assert!(store.load(&batch_id).await.expect("load").is_none());
    assert!(store.load(&fresh_id).await.expect("load").is_some());

    store
        .purge_expired(now + ChronoDuration::hours(48))
        .await
        .expect("purge remaining");
}
