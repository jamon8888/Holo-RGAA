use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSummary {
    pub id: String,
    pub url: String,
    pub taux_global: f64,
    pub etat_conformite: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("not found: {0}")]
    NotFound(String),
}

pub struct Storage {
    conn: Connection,
}

impl Storage {
    pub fn new(db_path: &Path) -> Result<Self, StorageError> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS audits (
                id TEXT PRIMARY KEY,
                url TEXT NOT NULL,
                data TEXT NOT NULL,
                taux_global REAL NOT NULL,
                etat_conformite TEXT NOT NULL,
                created_at TEXT NOT NULL
            )",
            [],
        )?;
        Ok(Self { conn })
    }

    pub fn save_audit(&self, audit: &rgaa_core::AuditResult) -> Result<String, StorageError> {
        let id = uuid::Uuid::new_v4().to_string();
        let data = serde_json::to_string(audit)?;
        let created_at = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO audits (id, url, data, taux_global, etat_conformite, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id,
                &audit.url,
                &data,
                audit.taux_global,
                &audit.etat_conformite,
                created_at
            ],
        )?;
        Ok(id)
    }

    pub fn get_audit(&self, id: &str) -> Result<Option<rgaa_core::AuditResult>, StorageError> {
        let mut stmt = self.conn.prepare("SELECT data FROM audits WHERE id = ?1")?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let data: String = row.get(0)?;
            let audit: rgaa_core::AuditResult = serde_json::from_str(&data)?;
            Ok(Some(audit))
        } else {
            Ok(None)
        }
    }

    pub fn list_audits(&self, limit: usize) -> Result<Vec<AuditSummary>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, url, taux_global, etat_conformite, created_at FROM audits ORDER BY created_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            let created_str: String = row.get(4)?;
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
            Ok(AuditSummary {
                id: row.get(0)?,
                url: row.get(1)?,
                taux_global: row.get(2)?,
                etat_conformite: row.get(3)?,
                created_at,
            })
        })?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn delete_audit(&self, id: &str) -> Result<(), StorageError> {
        let n = self
            .conn
            .execute("DELETE FROM audits WHERE id = ?1", params![id])?;
        if n == 0 {
            return Err(StorageError::NotFound(id.to_string()));
        }
        Ok(())
    }
}

/// Record a finished audit in the local database that the TUI History view
/// and `rgaa history` both read.
///
/// Failures are logged, never propagated: the audit itself succeeded, and
/// losing the bookkeeping must not lose the result the caller is holding.
pub async fn record_audit(audit: &rgaa_core::AuditResult) {
    match storage().await {
        Ok(storage) => {
            if let Err(e) = storage.save_audit(audit) {
                tracing::warn!(error = %e, "failed to record audit in local history");
            }
        }
        Err(e) => tracing::warn!(error = %e, "failed to open the local audit database"),
    }
}

pub async fn storage() -> Result<Storage, StorageError> {
    let db_path = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".rgaa")
        .join("audits.db");
    Storage::new(&db_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A database of its own per test, so the suite never touches
    /// `~/.rgaa/audits.db`.
    fn temp_db() -> std::path::PathBuf {
        std::env::temp_dir()
            .join("rgaa-tui-tests")
            .join(format!("{}.db", uuid::Uuid::new_v4()))
    }

    fn audit(url: &str, taux: f64) -> rgaa_core::AuditResult {
        rgaa_core::AuditResult {
            audit_id: "pipeline-id".to_string(),
            url: url.to_string(),
            pages: Vec::new(),
            total_criteria: 0,
            passed: 0,
            failed: 0,
            na: 0,
            overall_compliance: taux,
            taux_global: taux,
            coverage_percent: 0.0,
            etat_conformite: "partielle".to_string(),
            duration_ms: 12,
        }
    }

    #[test]
    fn saved_audits_round_trip() {
        let path = temp_db();
        let storage = Storage::new(&path).expect("open database");

        let first = storage
            .save_audit(&audit("https://example.com/one", 40.0))
            .expect("save first");
        let second = storage
            .save_audit(&audit("https://example.com/two", 90.0))
            .expect("save second");
        assert_ne!(first, second, "each audit gets its own id");

        let listed = storage.list_audits(10).expect("list audits");
        assert_eq!(listed.len(), 2);
        let urls: Vec<&str> = listed.iter().map(|a| a.url.as_str()).collect();
        assert!(urls.contains(&"https://example.com/one"));
        assert!(urls.contains(&"https://example.com/two"));

        let stored = storage.get_audit(&second).expect("get audit");
        assert_eq!(
            stored.map(|a| a.taux_global),
            Some(90.0),
            "the full result round-trips, not just the summary"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn list_honours_the_limit_and_delete_reports_a_miss() {
        let path = temp_db();
        let storage = Storage::new(&path).expect("open database");
        for i in 0..3 {
            storage
                .save_audit(&audit(&format!("https://example.com/{i}"), 50.0))
                .expect("save");
        }

        assert_eq!(storage.list_audits(2).expect("list").len(), 2);
        assert!(matches!(
            storage.delete_audit("no-such-id"),
            Err(StorageError::NotFound(_))
        ));

        let _ = std::fs::remove_file(&path);
    }
}
