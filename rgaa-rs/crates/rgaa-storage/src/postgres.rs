use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rgaa_core::{AuditBundle, AuditResult};
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

        // Single schema source: migrations/001 (the union of legacy /audit
        // columns and every /v1 table/column). Replaces the old inline DDL
        // that created only `audits`+`audit_logs` and left /v1 500-ing.
        sqlx::migrate!("./migrations").run(&pool).await?;

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

    fn pool(&self) -> &PgPool {
        &self.pool
    }

    async fn put_bundle(&self, bundle: &AuditBundle) -> Result<(), StorageError> {
        let mut tx = self.pool.begin().await?;

        // Upsert audit bundle - compute taux_global from findings
        let passed = bundle.summary.passed;
        let failed = bundle.summary.failed;
        let total_checked = passed + failed;
        let taux_global = if total_checked > 0 {
            (passed as f64 / total_checked as f64) * 100.0
        } else {
            0.0
        };
        let etat_conformite = if taux_global >= 100.0 {
            "totale"
        } else if taux_global >= 50.0 {
            "partielle"
        } else {
            "non conforme"
        };

        sqlx::query(
            r#"
            INSERT INTO audits (id, url, data, taux_global, etat_conformite, schema_version, audit_id, config, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), NOW())
            ON CONFLICT (id) DO UPDATE SET
                url = EXCLUDED.url,
                data = EXCLUDED.data,
                taux_global = EXCLUDED.taux_global,
                etat_conformite = EXCLUDED.etat_conformite,
                audit_id = EXCLUDED.audit_id,
                schema_version = EXCLUDED.schema_version,
                config = EXCLUDED.config,
                updated_at = NOW()
            "#,
        )
        .bind(&bundle.audit_id)
        .bind(&bundle.url)
        .bind(serde_json::to_value(bundle)?)
        .bind(taux_global)
        .bind(etat_conformite)
        .bind(&bundle.schema_version)
        .bind(&bundle.audit_id)
        .bind(serde_json::to_value(&bundle.config)?)
        .execute(&mut *tx)
        .await?;

        // Store findings
        for finding in &bundle.findings {
            sqlx::query(
                r#"
                INSERT INTO findings (id, audit_id, finding_id, rule, criterion_id, url, target, component_path, status, severity, fingerprint, evidence_kind, evidence_hash, source, details, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, NOW())
                ON CONFLICT (audit_id, finding_id) DO UPDATE SET
                    rule = EXCLUDED.rule,
                    criterion_id = EXCLUDED.criterion_id,
                    url = EXCLUDED.url,
                    target = EXCLUDED.target,
                    component_path = EXCLUDED.component_path,
                    status = EXCLUDED.status,
                    severity = EXCLUDED.severity,
                    fingerprint = EXCLUDED.fingerprint,
                    evidence_kind = EXCLUDED.evidence_kind,
                    evidence_hash = EXCLUDED.evidence_hash,
                    source = EXCLUDED.source,
                    details = EXCLUDED.details
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(&bundle.audit_id)
            .bind(&finding.id)
            .bind(&finding.rule)
            .bind(&finding.criterion_id)
            .bind(&finding.url)
            .bind(&finding.target)
            .bind(&finding.component_path)
            .bind(format!("{:?}", finding.status))
            .bind(&finding.severity)
            .bind(rgaa_core::FindingFingerprint::from_finding(finding))
            .bind(finding.evidence.iter().map(|e| e.kind.clone()).collect::<Vec<_>>())
            .bind(finding.evidence.iter().map(|e| e.hash.clone()).collect::<Vec<_>>())
            .bind(&finding.source)
            .bind(serde_json::to_value(&finding.details)?)
            .execute(&mut *tx)
            .await?;
        }

        // Store checkpoints
        for checkpoint in &bundle.checkpoints {
            sqlx::query(
                r#"
                INSERT INTO checkpoints (id, audit_id, checkpoint_id, criterion_id, status, evidence, summary, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
                ON CONFLICT (audit_id, checkpoint_id) DO UPDATE SET
                    criterion_id = EXCLUDED.criterion_id,
                    status = EXCLUDED.status,
                    evidence = EXCLUDED.evidence,
                    summary = EXCLUDED.summary
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(&bundle.audit_id)
            .bind(&checkpoint.checkpoint_id)
            .bind(&checkpoint.criterion_id)
            .bind(format!("{:?}", checkpoint.status))
            .bind(serde_json::to_value(&checkpoint.evidence)?)
            .bind(&checkpoint.summary)
            .execute(&mut *tx)
            .await?;
        }

        // Record bundle version
        let _version = sqlx::query_scalar::<_, i32>(
            r#"
            INSERT INTO audit_bundle_versions (id, audit_id, version, bundle_hash, schema_version, uploaded_at)
            VALUES ($1, $2, COALESCE((SELECT MAX(version) FROM audit_bundle_versions WHERE audit_id = $2), 0) + 1, $3, $4, NOW())
            ON CONFLICT (audit_id, version) DO NOTHING
            RETURNING version
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(&bundle.audit_id)
        .bind(compute_bundle_hash(bundle))
        .bind(&bundle.schema_version)
        .fetch_optional(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn get_bundle_by_audit_id(
        &self,
        audit_id: &str,
    ) -> Result<Option<AuditBundle>, StorageError> {
        let row: Option<(Value,)> = sqlx::query_as(
            r#"
            SELECT data FROM audits WHERE audit_id = $1 ORDER BY updated_at DESC LIMIT 1
            "#,
        )
        .bind(audit_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(data,)| serde_json::from_value(data).unwrap()))
    }
}

fn compute_bundle_hash(bundle: &AuditBundle) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    bundle.audit_id.hash(&mut hasher);
    bundle.url.hash(&mut hasher);
    bundle.schema_version.hash(&mut hasher);
    for f in &bundle.findings {
        f.id.hash(&mut hasher);
        f.rule.hash(&mut hasher);
        let status_str = match &f.status {
            rgaa_core::CriterionStatus::Pass => "Pass",
            rgaa_core::CriterionStatus::Fail => "Fail",
            rgaa_core::CriterionStatus::NotApplicable => "NotApplicable",
            rgaa_core::CriterionStatus::Error => "Error",
            rgaa_core::CriterionStatus::NeedsReview => "NeedsReview",
            rgaa_core::CriterionStatus::NotTested => "NotTested",
        };
        status_str.hash(&mut hasher);
    }
    format!("{:x}", hasher.finish())
}
