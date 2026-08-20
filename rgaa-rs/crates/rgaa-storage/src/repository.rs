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

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct FindingRow {
    pub id: Uuid,
    pub audit_id: Uuid,
    pub finding_id: String,
    pub rule: String,
    pub criterion_id: Option<String>,
    pub url: String,
    pub target: String,
    pub component_path: Option<String>,
    pub status: String,
    pub severity: Option<String>,
    pub fingerprint: String,
    pub evidence_kind: Vec<String>,
    pub evidence_hash: Vec<String>,
    pub source: String,
    pub details: Option<Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ApiKeyRow {
    pub id: Uuid,
    pub key_hash: String,
    pub name: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
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

    pub async fn complete_audit_with_results(
        &self,
        id: Uuid,
        result: &AuditResult,
    ) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;

        let result_json = serde_json::to_value(result)?;
        sqlx::query(
            r#"
            UPDATE audits SET status = 'completed', result = $1, updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(result_json)
        .bind(id)
        .execute(&mut *tx)
        .await?;

        for page in &result.pages {
            for criterion in &page.criteria {
                let violations_json = serde_json::to_value(&criterion.violations)?;
                sqlx::query(
                    r#"
                    INSERT INTO criterion_results
                        (id, audit_id, criterion_id, title, classification, status, violations, confidence, justification, source, created_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())
                    "#,
                )
                .bind(Uuid::new_v4())
                .bind(id)
                .bind(&criterion.criterion_id)
                .bind(&criterion.title)
                .bind(format!("{:?}", criterion.classification))
                .bind(format!("{:?}", criterion.status))
                .bind(violations_json)
                .bind(criterion.confidence)
                .bind(&criterion.justification)
                .bind(&criterion.source)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
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

    pub async fn get_bundle(&self, id: Uuid) -> anyhow::Result<Option<Value>> {
        self.get_audit(id).await
    }

    pub async fn list_audits_paginated(
        &self,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<Vec<AuditRow>> {
        self.list_audits(limit, offset).await
    }

    // API key validation
    pub async fn validate_api_key(
        &self,
        api_key: &str,
        required_scope: &str,
    ) -> anyhow::Result<Option<ApiKeyRow>> {
        let key_hash = hash_api_key(api_key);
        let row = sqlx::query_as::<_, ApiKeyRow>(
            r#"
            SELECT * FROM api_keys WHERE key_hash = $1 AND (expires_at IS NULL OR expires_at > NOW())
            "#,
        )
        .bind(&key_hash)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(key_row) = row {
            if key_row.scopes.contains(&required_scope.to_string())
                || key_row.scopes.contains(&"*".to_string())
            {
                // Update last_used_at
                sqlx::query("UPDATE api_keys SET last_used_at = NOW() WHERE id = $1")
                    .bind(key_row.id)
                    .execute(&self.pool)
                    .await?;
                Ok(Some(key_row))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub async fn create_api_key(
        &self,
        name: &str,
        scopes: &[String],
        expires_at: Option<DateTime<Utc>>,
    ) -> anyhow::Result<(Uuid, String)> {
        let key = Uuid::new_v4().to_string();
        let key_hash = hash_api_key(&key);
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO api_keys (id, key_hash, name, scopes, expires_at, created_at)
            VALUES ($1, $2, $3, $4, $5, NOW())
            "#,
        )
        .bind(id)
        .bind(&key_hash)
        .bind(name)
        .bind(scopes)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        Ok((id, key))
    }

    pub async fn get_bundle_by_audit_id(&self, audit_id: &str) -> anyhow::Result<Option<Value>> {
        let row: Option<(Value,)> = sqlx::query_as(
            r#"
            SELECT result FROM audits WHERE audit_id = $1
            "#,
        )
        .bind(audit_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|(result,)| result))
    }

    pub async fn put_bundle(&self, bundle: &rgaa_core::AuditBundle) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;

        // Upsert audit
        let audit_id = &bundle.audit_id;
        sqlx::query(
            r#"
            INSERT INTO audits (id, url, status, result, schema_version, audit_id, config, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
            ON CONFLICT (audit_id, schema_version) DO UPDATE SET
                url = EXCLUDED.url,
                status = EXCLUDED.status,
                result = EXCLUDED.result,
                config = EXCLUDED.config,
                updated_at = NOW()
            "#,
        )
        .bind(audit_id)
        .bind(&bundle.url)
        .bind(compute_audit_status(bundle))  // Use computed status instead of bundle.status
        .bind(serde_json::to_value(bundle)?)
        .bind(&bundle.schema_version)
        .bind(audit_id)
        .bind(serde_json::to_value(&bundle.config)?)
        .execute(&mut *tx)
        .await?;

        // Store findings
        let audit_id = &bundle.audit_id;
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
            .bind(audit_id)
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
            .bind(audit_id)
            .bind(&checkpoint.checkpoint_id)
            .bind(&checkpoint.criterion_id)
            .bind(format!("{:?}", checkpoint.status))
            .bind(serde_json::to_value(&checkpoint.evidence)?)
            .bind(&checkpoint.summary)
            .execute(&mut *tx)
            .await?;
        }

        // Record bundle version
        let audit_id_str = &bundle.audit_id;
        let _version = sqlx::query_scalar::<_, i32>(
            r#"
            INSERT INTO audit_bundle_versions (id, audit_id, version, bundle_hash, schema_version, uploaded_at)
            VALUES ($1, $2, COALESCE((SELECT MAX(version) FROM audit_bundle_versions WHERE audit_id = $2), 0) + 1, $3, $4, NOW())
            ON CONFLICT (audit_id, version) DO NOTHING
            RETURNING version
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(audit_id_str)
        .bind(compute_bundle_hash_static(bundle))
        .bind(&bundle.schema_version)
        .fetch_optional(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn list_findings(
        &self,
        audit_id: Uuid,
        status: Option<&str>,
        severity: Option<&str>,
        rule: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<Vec<FindingRow>> {
        let mut query = String::from(
            r#"
            SELECT id, audit_id, finding_id, rule, criterion_id, url, target, component_path, status, severity, fingerprint, evidence_kind, evidence_hash, source, details, created_at
            FROM findings
            WHERE audit_id = $1
            "#,
        );
        let mut param_count = 2;

        if let Some(_status) = status {
            param_count += 1;
            query.push_str(&format!(" AND status = ${}", param_count - 1));
        }
        if let Some(_severity) = severity {
            param_count += 1;
            query.push_str(&format!(" AND severity = ${}", param_count - 1));
        }
        if let Some(_rule) = rule {
            param_count += 1;
            query.push_str(&format!(" AND rule = ${}", param_count - 1));
        }

        query.push_str(" ORDER BY created_at DESC LIMIT $");
        query.push_str(&param_count.to_string());
        param_count += 1;
        query.push_str(" OFFSET $");
        query.push_str(&param_count.to_string());

        let mut q = sqlx::query_as(&query).bind(audit_id);
        if let Some(status) = status {
            q = q.bind(status);
        }
        if let Some(severity) = severity {
            q = q.bind(severity);
        }
        if let Some(rule) = rule {
            q = q.bind(rule);
        }
        q = q.bind(limit).bind(offset);

        let rows = q.fetch_all(&self.pool).await?;
        Ok(rows)
    }

    pub async fn store_policy_evaluation(
        &self,
        eval: &rgaa_remediation::PolicyResult,
        audit_id: Uuid,
        baseline_id: Option<Uuid>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO policy_evaluations (id, audit_id, baseline_id, passed, failures, warnings, counts, evaluated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(audit_id)
        .bind(baseline_id)
        .bind(eval.passed)
        .bind(serde_json::to_value(&eval.failures)?)
        .bind(serde_json::to_value(&eval.warnings)?)
        .bind(serde_json::to_value(&eval.counts)?)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

pub fn hash_api_key(key: &str) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn compute_audit_status(bundle: &rgaa_core::AuditBundle) -> String {
    let total = bundle.summary.passed
        + bundle.summary.failed
        + bundle.summary.needs_review
        + bundle.summary.errors;
    if total == 0 {
        "completed".to_string()
    } else if bundle.summary.failed > 0 || bundle.summary.errors > 0 {
        "failed".to_string()
    } else if bundle.summary.needs_review > 0 {
        "needs_review".to_string()
    } else {
        "passed".to_string()
    }
}

fn compute_bundle_hash_static(bundle: &rgaa_core::AuditBundle) -> String {
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
