use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use rgaa_core::{AuditResult, CriterionResult};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuditRow {
    pub id: Uuid,
    pub url: String,
    pub status: String,
    pub result: Option<Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriterionResultRow {
    pub id: Uuid,
    pub audit_id: Uuid,
    pub criterion_id: String,
    pub title: String,
    pub classification: String,
    pub status: String,
    pub violations: Value,
    pub confidence: Option<f64>,
    pub justification: Option<String>,
    pub source: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct Repository {
    pool: PgPool,
}

impl Repository {
    pub fn new(pool: &PgPool) -> Self {
        Self { pool: pool.clone() }
    }

    pub async fn create_audit(&self, url: &str) -> anyhow::Result<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO audits (id, url, status, created_at, updated_at)
            VALUES ($1, $2, 'pending', NOW(), NOW())
            "#,
        )
        .bind(id)
        .bind(url)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn update_audit_status(&self, id: Uuid, status: &str) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            UPDATE audits SET status = $1, updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(status)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn complete_audit(&self, id: Uuid, result: &AuditResult) -> anyhow::Result<()> {
        let result_json = serde_json::to_value(result)?;
        sqlx::query(
            r#"
            UPDATE audits SET status = 'completed', result = $1, updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(result_json)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn store_criterion_results(
        &self,
        audit_id: Uuid,
        criteria: &[CriterionResult],
    ) -> anyhow::Result<()> {
        for criterion in criteria {
            let violations_json = serde_json::to_value(&criterion.violations)?;
            sqlx::query(
                r#"
                INSERT INTO criterion_results
                    (id, audit_id, criterion_id, title, classification, status, violations, confidence, justification, source, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(audit_id)
            .bind(&criterion.criterion_id)
            .bind(&criterion.title)
            .bind(format!("{:?}", criterion.classification))
            .bind(format!("{:?}", criterion.status))
            .bind(violations_json)
            .bind(criterion.confidence)
            .bind(&criterion.justification)
            .bind(&criterion.source)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn get_audit(&self, id: Uuid) -> anyhow::Result<Option<Value>> {
        let row: Option<(Value,)> = sqlx::query_as(
            r#"
            SELECT result FROM audits WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|(result,)| result))
    }

    pub async fn list_audits(&self, limit: i64, offset: i64) -> anyhow::Result<Vec<AuditRow>> {
        let rows = sqlx::query_as::<_, AuditRow>(
            r#"
            SELECT id, url, status, result, created_at, updated_at
            FROM audits
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
