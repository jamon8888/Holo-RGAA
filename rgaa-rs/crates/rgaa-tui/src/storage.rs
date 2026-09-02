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
        let mut stmt = self
            .conn
            .prepare("SELECT data FROM audits WHERE id = ?1")?;
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

pub async fn storage() -> Result<Storage, StorageError> {
    let db_path = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".rgaa")
        .join("audits.db");
    Storage::new(&db_path)
}
