use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rgaa_core::AuditResult;
use sqlx::postgres::{PgPool, PgPoolOptions};
use uuid::Uuid;

use crate::{AuditSummary, Storage, StorageError};

pub struct PostgresStorage {
    pool: PgPool,
}

impl PostgresStorage {
    pub async fn new(database_url: &str) -> Result<Self, StorageError> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
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

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[derive(sqlx::FromRow)]
struct AuditDbRow {
    id: String,
    url: String,
    data: sqlx::types::Json<AuditResult>,
    taux_global: f64,
    etat_conformite: String,
    created_at: DateTime<Utc>,
}

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
        let row: Option<AuditDbRow> = sqlx::query_as(
            r#"
            SELECT id, url, data, taux_global, etat_conformite, created_at
            FROM audits WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.data.0))
    }

    async fn list_audits(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AuditSummary>, StorageError> {
        let rows: Vec<AuditDbRow> = sqlx::query_as(
            r#"
            SELECT id, url, data, taux_global, etat_conformite, created_at
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
}
