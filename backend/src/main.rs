//! RGAA Audit Backend - Phase 1 MVP
//! Orchestration pipeline: URL → Playwright/axe-core → Results → PostgreSQL

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, FromRow};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use tracing::{info, error};
use anyhow::Result;

// ──────────────────────────────────────────────
// Domain Models
// ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Audit {
    pub id: Uuid,
    pub url: String,
    pub status: AuditStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub total_criteria: i32,
    pub passed_criteria: i32,
    pub failed_criteria: i32,
    pub na_criteria: i32,
    pub compliance_rate: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone, Copy, PartialEq)]
#[sqlx(type_name = "audit_status", rename_all = "lowercase")]
pub enum AuditStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct CriterionResult {
    pub id: Uuid,
    pub audit_id: Uuid,
    pub criterion_id: String,        // ex: "1.1", "9.1"
    pub criterion_title: String,
    pub classification: Classification,
    pub status: CriterionStatus,
    pub axe_rule: Option<String>,
    pub impact: Option<String>,
    pub description: Option<String>,
    pub nodes_affected: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone, Copy, PartialEq)]
#[sqlx(type_name = "classification", rename_all = "lowercase")]
pub enum Classification {
    Deterministe,
    IaAssiste,
    Manuel,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone, Copy, PartialEq)]
#[sqlx(type_name = "criterion_status", rename_all = "lowercase")]
pub enum CriterionStatus {
    Pass,
    Fail,
    Na,
    Error,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditRequest {
    pub url: String,
    pub sample_mode: Option<bool>,  // DINUM sampling
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditResponse {
    pub audit_id: Uuid,
    pub status: AuditStatus,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditDetailResponse {
    pub audit: Audit,
    pub criteria: Vec<CriterionResult>,
}

// ──────────────────────────────────────────────
// Database
// ──────────────────────────────────────────────

pub async fn init_db(pool: &PgPool) -> Result<()> {
    // Create types (ignore errors if they exist)
    let _ = sqlx::query("CREATE TYPE audit_status AS ENUM ('pending', 'running', 'completed', 'failed')")
        .execute(pool)
        .await;
    let _ = sqlx::query("CREATE TYPE classification AS ENUM ('deterministe', 'ia_assiste', 'manuel')")
        .execute(pool)
        .await;
    let _ = sqlx::query("CREATE TYPE criterion_status AS ENUM ('pass', 'fail', 'na', 'error')")
        .execute(pool)
        .await;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS audits (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            url TEXT NOT NULL,
            status audit_status NOT NULL DEFAULT 'pending',
            started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            completed_at TIMESTAMPTZ,
            total_criteria INT NOT NULL DEFAULT 0,
            passed_criteria INT NOT NULL DEFAULT 0,
            failed_criteria INT NOT NULL DEFAULT 0,
            na_criteria INT NOT NULL DEFAULT 0,
            compliance_rate DOUBLE PRECISION NOT NULL DEFAULT 0.0,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS criterion_results (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            audit_id UUID NOT NULL REFERENCES audits(id) ON DELETE CASCADE,
            criterion_id TEXT NOT NULL,
            criterion_title TEXT NOT NULL,
            classification classification NOT NULL,
            status criterion_status NOT NULL,
            axe_rule TEXT,
            impact TEXT,
            description TEXT,
            nodes_affected INT NOT NULL DEFAULT 0,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_criterion_results_audit_id ON criterion_results(audit_id);"
    )
    .execute(pool)
    .await?;

    info!("Database initialized");
    Ok(())
}

// ──────────────────────────────────────────────
// State
// ──────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

// ──────────────────────────────────────────────
// Handlers
// ──────────────────────────────────────────────

async fn create_audit(
    State(state): State<AppState>,
    Json(payload): Json<AuditRequest>,
) -> Result<Json<AuditResponse>, (StatusCode, String)> {
    let audit_id = Uuid::new_v4();
    
    sqlx::query(
        "INSERT INTO audits (id, url, status) VALUES ($1, $2, 'pending')"
    )
    .bind(audit_id)
    .bind(&payload.url)
    .execute(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Spawn background audit task
    let pool = state.pool.clone();
    let url = payload.url.clone();
    tokio::spawn(async move {
        if let Err(e) = run_audit(pool, audit_id, url).await {
            error!("Audit {} failed: {}", audit_id, e);
        }
    });

    Ok(Json(AuditResponse {
        audit_id,
        status: AuditStatus::Pending,
        message: "Audit démarré en arrière-plan".to_string(),
    }))
}

async fn get_audit(
    State(state): State<AppState>,
    Path(audit_id): Path<Uuid>,
) -> Result<Json<AuditDetailResponse>, (StatusCode, String)> {
    let audit = sqlx::query_as::<_, Audit>("SELECT * FROM audits WHERE id = $1")
        .bind(audit_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Audit non trouvé".to_string()))?;

    let criteria = sqlx::query_as::<_, CriterionResult>(
        "SELECT * FROM criterion_results WHERE audit_id = $1 ORDER BY criterion_id"
    )
    .bind(audit_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(AuditDetailResponse { audit, criteria }))
}

async fn list_audits(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<Audit>>, (StatusCode, String)> {
    let limit = params.limit.unwrap_or(50).min(200);
    let offset = params.offset.unwrap_or(0);

    let audits = sqlx::query_as::<_, Audit>(
        "SELECT * FROM audits ORDER BY created_at DESC LIMIT $1 OFFSET $2"
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(audits))
}

#[derive(Deserialize)]
struct ListParams {
    limit: Option<i64>,
    offset: Option<i64>,
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "service": "rgaa-audit-backend" }))
}

// ──────────────────────────────────────────────
// Audit Runner (calls Node.js POC)
// ──────────────────────────────────────────────

async fn run_audit(pool: PgPool, audit_id: Uuid, url: String) -> Result<()> {
    info!("Starting audit {} for {}", audit_id, url);

    // Update status to running
    sqlx::query("UPDATE audits SET status = 'running' WHERE id = $1")
        .bind(audit_id)
        .execute(&pool)
        .await?;

    // Call Node.js POC via command line
    let output = tokio::process::Command::new("node")
        .args(["poc.js", &url])
        .current_dir("/home/jamin/Documents/RGAA")
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("POC failed: {}", stderr);
        sqlx::query("UPDATE audits SET status = 'failed' WHERE id = $1")
            .bind(audit_id)
            .execute(&pool)
            .await?;
        return Ok(());
    }

    // Parse results from JSON file (latest)
    let stdout = String::from_utf8_lossy(&output.stdout);
    info!("POC output: {}", stdout);

    // Find the generated JSON file
    let entries = std::fs::read_dir("/home/jamin/Documents/RGAA")?;
    let mut result_file: Option<(std::path::PathBuf, std::time::SystemTime)> = None;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("rgaa-results-") && name.ends_with(".json") {
            let meta = entry.metadata()?;
            let modified = meta.modified()?;
            if result_file.is_none() || modified > result_file.as_ref().unwrap().1 {
                result_file = Some((entry.path(), modified));
            }
        }
    }

    let (total, passed, failed, na, compliance) = if let Some((ref path, _)) = result_file {
        let content = std::fs::read_to_string(path)?;
        let results: serde_json::Value = serde_json::from_str(&content)?;
        let mut t = 0; let mut p = 0; let mut f = 0; let mut n = 0;
        for (_, v) in results.as_object().unwrap_or(&serde_json::Map::new()) {
            t += 1;
            match v["status"].as_str() {
                Some("FAIL") => f += 1,
                Some("PASS") => p += 1,
                _ => n += 1,
            }
        }
        let rate = if t > n { (p as f64 / (t - n) as f64) * 100.0 } else { 0.0 };
        (t, p, f, n, rate)
    } else {
        (0, 0, 0, 0, 0.0)
    };

    // Update audit with summary
    sqlx::query(
        r#"UPDATE audits SET 
            status = 'completed', 
            completed_at = NOW(),
            total_criteria = $1, passed_criteria = $2, failed_criteria = $3, na_criteria = $4, compliance_rate = $5
            WHERE id = $6"#
    )
    .bind(total)
    .bind(passed)
    .bind(failed)
    .bind(na)
    .bind(compliance)
    .bind(audit_id)
    .execute(&pool)
    .await?;

    // Insert detailed criterion results from JSON
    if let Some((ref path, _)) = result_file {
        let content = std::fs::read_to_string(path)?;
        let results: serde_json::Value = serde_json::from_str(&content)?;
        
        // Empty violations placeholder
        let empty_violations: Vec<serde_json::Value> = vec![];
        
        for (criterion_id, criterion_data) in results.as_object().unwrap_or(&serde_json::Map::new()) {
            let status = criterion_data["status"].as_str().unwrap_or("ERROR");
            let violations = criterion_data["violations"].as_array().unwrap_or(&empty_violations);
            let _passes = criterion_data["passes"].as_i64().unwrap_or(0);
            let _inapplicable = criterion_data["inapplicable"].as_i64().unwrap_or(0);
            
            // Get criterion title and classification from our mapping
            let (criterion_title, classification) = get_criterion_info(criterion_id)
                .unwrap_or((criterion_id, Classification::Deterministe));
            
            let criterion_status = match status {
                "FAIL" => CriterionStatus::Fail,
                "PASS" => CriterionStatus::Pass,
                _ => CriterionStatus::Na,
            };
            
            // Extract first violation details if any
            let (axe_rule, impact, description, nodes_affected) = if let Some(first_viol) = violations.first() {
                (
                    first_viol["rule"].as_str().map(|s| s.to_string()),
                    first_viol["impact"].as_str().map(|s| s.to_string()),
                    first_viol["description"].as_str().map(|s| s.to_string()),
                    first_viol["nodes"].as_i64().unwrap_or(0) as i32,
                )
            } else {
                (None, None, None, 0)
            };
            
            sqlx::query(
                r#"INSERT INTO criterion_results 
                (audit_id, criterion_id, criterion_title, classification, status, axe_rule, impact, description, nodes_affected)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#
            )
            .bind(audit_id)
            .bind(criterion_id)
            .bind(criterion_title)
            .bind(classification)
            .bind(criterion_status)
            .bind(axe_rule)
            .bind(impact)
            .bind(description)
            .bind(nodes_affected)
            .execute(&pool)
            .await?;
        }
    }

    info!("Audit {} completed: {}/{} passed ({:.1}%)", audit_id, passed, total, compliance);
    Ok(())
}

// ──────────────────────────────────────────────
// Criterion Mapping (auto-generated from CSV)
// ──────────────────────────────────────────────

fn get_criterion_info(criterion_id: &str) -> Option<(&'static str, Classification)> {
    match criterion_id {
        "1.1" => Some(("Alternative textuelle présente", Classification::Deterministe)),
        "1.2" => Some(("Image décorative ignorée", Classification::Deterministe)),
        "1.3" => Some(("Alternative textuelle pertinente", Classification::IaAssiste)),
        "1.4" => Some(("Alternative CAPTCHA/image-test", Classification::Deterministe)),
        "1.5" => Some(("Solution accès alternatif CAPTCHA", Classification::Deterministe)),
        "1.6" => Some(("Description détaillée présente", Classification::Deterministe)),
        "1.7" => Some(("Description détaillée pertinente", Classification::IaAssiste)),
        "1.8" => Some(("Image texte remplacée par texte stylé", Classification::Deterministe)),
        "1.9" => Some(("Légende reliée à l'image", Classification::Deterministe)),
        "2.1" => Some(("Cadre a un titre", Classification::Deterministe)),
        "2.2" => Some(("Titre de cadre pertinent", Classification::IaAssiste)),
        "3.1" => Some(("Information non donnée uniquement par couleur", Classification::Deterministe)),
        "3.2" => Some(("Contraste texte/fond suffisant", Classification::Deterministe)),
        "3.3" => Some(("Contraste composants graphiques suffisant", Classification::Deterministe)),
        "4.1" => Some(("Transcription/audiodescription présente", Classification::Deterministe)),
        "4.2" => Some(("Transcription/audiodescription pertinente", Classification::IaAssiste)),
        "4.3" => Some(("Sous-titres synchronisés présents", Classification::Deterministe)),
        "4.4" => Some(("Sous-titres pertinents", Classification::IaAssiste)),
        "4.5" => Some(("Audiodescription présente", Classification::Deterministe)),
        "4.6" => Some(("Audiodescription pertinente", Classification::IaAssiste)),
        "4.7" => Some(("Média temporel identifiable", Classification::Deterministe)),
        "4.8" => Some(("Alternative média non temporel", Classification::Deterministe)),
        "4.9" => Some(("Alternative pertinente média non temporel", Classification::IaAssiste)),
        "4.10" => Some(("Son contrôlable", Classification::Deterministe)),
        "4.11" => Some(("Média temporel contrôlable clavier", Classification::Deterministe)),
        "4.12" => Some(("Média non temporel contrôlable clavier", Classification::Deterministe)),
        "4.13" => Some(("Média compatible AT", Classification::Deterministe)),
        "5.1" => Some(("Tableau complexe a résumé", Classification::Deterministe)),
        "5.2" => Some(("Résumé pertinent tableau complexe", Classification::IaAssiste)),
        "5.3" => Some(("Contenu linéarisé compréhensible", Classification::IaAssiste)),
        "5.4" => Some(("Titre tableau correctement associé", Classification::Deterministe)),
        "5.5" => Some(("Titre pertinent tableau", Classification::IaAssiste)),
        "5.6" => Some(("En-têtes déclarés correctement", Classification::Deterministe)),
        "5.7" => Some(("Association cellules/en-têtes", Classification::Deterministe)),
        "5.8" => Some(("Tableau mise en forme sans éléments données", Classification::Deterministe)),
        "6.1" => Some(("Lien explicite", Classification::Deterministe)),
        "6.2" => Some(("Lien a un intitulé", Classification::Deterministe)),
        "7.1" => Some(("Script compatible AT", Classification::Deterministe)),
        "7.2" => Some(("Alternative script pertinente", Classification::IaAssiste)),
        "7.3" => Some(("Script contrôlable clavier", Classification::Deterministe)),
        "7.4" => Some(("Changement de contexte averti/contrôlé", Classification::Deterministe)),
        "7.5" => Some(("Messages de statut restitués AT", Classification::Manuel)),
        "8.1" => Some(("Type de document défini", Classification::Deterministe)),
        "8.2" => Some(("Code valide selon doctype", Classification::Deterministe)),
        "8.3" => Some(("Langue par défaut présente", Classification::Deterministe)),
        "8.4" => Some(("Code de langue pertinent", Classification::IaAssiste)),
        "8.5" => Some(("Titre de page", Classification::Deterministe)),
        "8.6" => Some(("Titre de page pertinent", Classification::IaAssiste)),
        "8.7" => Some(("Changement de langue indiqué", Classification::Deterministe)),
        "8.8" => Some(("Code de langue changement pertinent", Classification::IaAssiste)),
        "8.9" => Some(("Balises pas uniquement présentation", Classification::Deterministe)),
        "8.10" => Some(("Changements sens lecture signalés", Classification::Deterministe)),
        "9.1" => Some(("Structure par titres appropriée", Classification::Deterministe)),
        "9.2" => Some(("Structure document cohérente", Classification::IaAssiste)),
        "9.3" => Some(("Liste correctement structurée", Classification::Deterministe)),
        "9.4" => Some(("Citation correctement indiquée", Classification::Deterministe)),
        "10.1" => Some(("CSS pour présentation", Classification::Deterministe)),
        "10.2" => Some(("Contenu visible sans CSS", Classification::Deterministe)),
        "10.3" => Some(("Information compréhensible sans CSS", Classification::IaAssiste)),
        "10.4" => Some(("Texte lisible zoom 200%", Classification::Deterministe)),
        "10.5" => Some(("Déclarations CSS couleurs correctes", Classification::Deterministe)),
        "10.6" => Some(("Lien visible vs texte environnant", Classification::Deterministe)),
        "10.7" => Some(("Focus visible", Classification::Deterministe)),
        "10.8" => Some(("Contenus cachés ignorés AT", Classification::Deterministe)),
        "10.9" => Some(("Info non donnée par forme/taille/position", Classification::Deterministe)),
        "10.10" => Some(("Implémentation pertinente forme/taille/position", Classification::IaAssiste)),
        "10.11" => Some(("Reflow 320px/256px", Classification::Deterministe)),
        "10.12" => Some(("Espacement texte redéfinissable", Classification::Deterministe)),
        "10.13" => Some(("Contenus additionnels focus/survol contrôlables", Classification::Deterministe)),
        "10.14" => Some(("Contenus CSS only accessibles clavier", Classification::Deterministe)),
        "11.1" => Some(("Champ a étiquette", Classification::Deterministe)),
        "11.2" => Some(("Étiquette champ pertinente", Classification::IaAssiste)),
        "11.3" => Some(("Étiquettes cohérentes même fonction", Classification::IaAssiste)),
        "11.4" => Some(("Étiquette et champ accolés", Classification::Deterministe)),
        "11.5" => Some(("Champs même nature regroupés", Classification::Deterministe)),
        "11.6" => Some(("Regroupement a légende", Classification::Deterministe)),
        "11.7" => Some(("Légende regroupement pertinente", Classification::IaAssiste)),
        "11.8" => Some(("Items liste choix regroupés pertinemment", Classification::IaAssiste)),
        "11.9" => Some(("Intitulé bouton pertinent", Classification::IaAssiste)),
        "11.10" => Some(("Contrôle saisie utilisé pertinemment", Classification::IaAssiste)),
        "11.11" => Some(("Suggestions correction erreurs", Classification::Deterministe)),
        "11.12" => Some(("Données modifiables/récupérables", Classification::Deterministe)),
        "11.13" => Some(("Finalité champ déductible", Classification::Deterministe)),
        "12.1" => Some(("Deux systèmes navigation", Classification::Deterministe)),
        "12.2" => Some(("Navigation même place", Classification::Deterministe)),
        "12.3" => Some(("Plan du site pertinent", Classification::IaAssiste)),
        "12.4" => Some(("Plan site accessible identique", Classification::Deterministe)),
        "12.5" => Some(("Moteur recherche atteignable identiquement", Classification::Deterministe)),
        "12.6" => Some(("Zones regroupement atteignables", Classification::Deterministe)),
        "12.7" => Some(("Lien évitement contenu principal", Classification::Deterministe)),
        "12.8" => Some(("Ordre tabulation cohérent", Classification::IaAssiste)),
        "12.9" => Some(("Pas de piège clavier", Classification::Deterministe)),
        "12.10" => Some(("Raccourcis clavier contrôlables", Classification::Deterministe)),
        "12.11" => Some(("Contenus additionnels atteignables clavier", Classification::Deterministe)),
        "13.1" => Some(("Contrôle limites temps", Classification::Deterministe)),
        "13.2" => Some(("Pas ouverture fenêtre sans action", Classification::Deterministe)),
        "13.3" => Some(("Document bureautique version accessible", Classification::Deterministe)),
        "13.4" => Some(("Version accessible même information", Classification::Deterministe)),
        "13.5" => Some(("Contenu cryptique a alternative", Classification::Deterministe)),
        "13.6" => Some(("Alternative pertinente contenu cryptique", Classification::IaAssiste)),
        "13.7" => Some(("Flash/luminosité corrects", Classification::Deterministe)),
        "13.8" => Some(("Contenu mouvement/clignotant contrôlable", Classification::Deterministe)),
        "13.9" => Some(("Orientation portrait/paysage", Classification::Deterministe)),
        "13.10" => Some(("Geste complexe = geste simple", Classification::Deterministe)),
        "13.11" => Some(("Annulation action pointage", Classification::Deterministe)),
        "13.12" => Some(("Mouvement appareil alternative", Classification::Deterministe)),
        _ => None,
    }
}

// ──────────────────────────────────────────────
// Main
// ──────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/rgaa".to_string());

    let pool = PgPool::connect(&database_url).await?;
    init_db(&pool).await?;

    let state = AppState { pool };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/audits", post(create_audit).get(list_audits))
        .route("/audits/:id", get(get_audit))
        .with_state(state)
        .layer(tower_http::cors::CorsLayer::permissive())
        .layer(tower_http::trace::TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    info!("Server running on http://0.0.0.0:3000");
    axum::serve(listener, app).await?;

    Ok(())
}