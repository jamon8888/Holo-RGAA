use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rgaa_core::AuditResult;
use serde_json::Value;
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Executor;
use std::time::Duration;
use uuid::Uuid;

use crate::{AuditSummary, Storage, StorageError};

/// Max simultaneous connections in the pool. Sized for the batch operating
/// point (#42: 8 concurrent audits) plus headroom for `save_audit_log` and
/// admin/listing calls sharing the pool, without dedicating a whole
/// connection to every one of the 8.
const MAX_POOL_CONNECTIONS: u32 = 10;
/// How long a caller waits for a free connection before giving up loudly
/// instead of queueing indefinitely under a connection-pool burst.
const POOL_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(10);
/// Per-statement ceiling, set on every pooled connection via
/// `SET statement_timeout`, so one pathological query can't hold a
/// connection (and thus shrink the effective pool) indefinitely.
const STATEMENT_TIMEOUT_MS: i64 = 30_000;

pub struct PostgresStorage {
    pool: PgPool,
}

impl PostgresStorage {
    pub async fn new(database_url: &str) -> Result<Self, StorageError> {
        let pool = PgPoolOptions::new()
            .max_connections(MAX_POOL_CONNECTIONS)
            .acquire_timeout(POOL_ACQUIRE_TIMEOUT)
            .after_connect(|conn, _meta| {
                Box::pin(async move {
                    conn.execute(
                        format!("SET statement_timeout = {STATEMENT_TIMEOUT_MS}").as_str(),
                    )
                    .await?;
                    Ok(())
                })
            })
            .connect(database_url)
            .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS audits (
                id TEXT PRIMARY KEY,
                url TEXT NOT NULL,
                data JSONB NOT NULL,
                taux_global REAL NOT NULL,
                etat_conformite TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS audit_logs (
                id TEXT PRIMARY KEY,
                audit_id TEXT NOT NULL,
                action TEXT NOT NULL,
                timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                details JSONB
            )
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

/// Listing-only projection: never selects the `data` JSONB column, which
/// holds the whole `AuditResult` (every page, finding, and violation) —
/// `list_audits` only ever needed the summary fields already in
/// `AuditSummary`, so reading `data` on every row was pure waste that grows
/// with the number of audits listed, not just the number of pages shown.
#[derive(sqlx::FromRow)]
struct AuditSummaryDbRow {
    id: String,
    url: String,
    taux_global: f64,
    etat_conformite: String,
    created_at: DateTime<Utc>,
}

/// Hard ceiling on `list_audits`' page size, regardless of what a caller
/// requests, so listing stays fast even at thousands of audits.
const MAX_LIST_PAGE_SIZE: usize = 200;

#[async_trait]
impl Storage for PostgresStorage {
    async fn save_audit(&self, audit: &AuditResult) -> Result<String, StorageError> {
        let id = if audit.audit_id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            audit.audit_id.clone()
        };

        let data = serde_json::to_value(audit)?;

        sqlx::query(
            r#"
            INSERT INTO audits (id, url, data, taux_global, etat_conformite, created_at)
            VALUES ($1, $2, $3, $4, $5, NOW())
            ON CONFLICT (id) DO UPDATE SET
                url = EXCLUDED.url,
                data = EXCLUDED.data,
                taux_global = EXCLUDED.taux_global,
                etat_conformite = EXCLUDED.etat_conformite
            "#,
        )
        .bind(&id)
        .bind(&audit.url)
        .bind(data)
        .bind(audit.taux_global)
        .bind(&audit.etat_conformite)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    async fn get_audit(&self, id: &str) -> Result<Option<AuditResult>, StorageError> {
        let row: Option<(sqlx::types::Json<AuditResult>,)> =
            sqlx::query_as(r#"SELECT data FROM audits WHERE id = $1"#)
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(row.map(|(data,)| data.0))
    }

    async fn list_audits(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AuditSummary>, StorageError> {
        // 0 is a legitimate "give me nothing" request (e.g. a pagination
        // probe), not a lower bound to round up to 1 — only cap the upper
        // end.
        let limit = limit.min(MAX_LIST_PAGE_SIZE);
        let rows: Vec<AuditSummaryDbRow> = sqlx::query_as(
            r#"
            SELECT id, url, taux_global, etat_conformite, created_at
            FROM audits
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| AuditSummary {
                id: r.id,
                url: r.url,
                taux_global: r.taux_global,
                etat_conformite: r.etat_conformite,
                created_at: r.created_at,
            })
            .collect())
    }

    async fn delete_audit(&self, id: &str) -> Result<(), StorageError> {
        sqlx::query(r#"DELETE FROM audits WHERE id = $1"#)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn save_audit_log(
        &self,
        audit_id: &str,
        action: &str,
        details: Option<Value>,
    ) -> Result<String, StorageError> {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO audit_logs (id, audit_id, action, timestamp, details)
            VALUES ($1, $2, $3, NOW(), $4)
            "#,
        )
        .bind(&id)
        .bind(audit_id)
        .bind(action)
        .bind(details)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }
}
