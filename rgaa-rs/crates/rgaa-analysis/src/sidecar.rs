//! Sidecar declaration, discovery and JSON-RPC stdio protocol (P-00 §7 bis).
//!
//! Heavy files travel as paths in a temporary audit directory, never as
//! base64 in JSON. A rejected proof, a timeout or a non-zero exit becomes an
//! explicit `RgaaError::Analysis`; the orchestrator puts the tests in review.

use crate::capture::PageCapture;
use crate::error::{analysis_error, missing_proof, Result};
use crate::types::{PluginKind, PluginOptions, Preuve};
use crate::{sha256_hex, AnalysisPlugin};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Child;
use tokio::time::Duration;

const PROTOCOL_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SidecarMode {
    /// One process per audit.
    #[default]
    Persistant,
    /// One process per call.
    ParAppel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinaireRequis {
    pub nom: String,
    pub version_min: String,
    pub verif: Vec<String>,
}

/// `plugins/<id>/sidecar.toml`, searched under `$RGAA_PLUGINS_DIR` then
/// `~/.config/rgaa/plugins`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidecarManifest {
    pub id: String,
    pub version: String,
    pub tests: Vec<String>,
    pub commande: Vec<String>,
    #[serde(default)]
    pub mode: SidecarMode,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub binaires_requis: Vec<BinaireRequis>,
    #[serde(default)]
    pub licence: String,
}

fn default_timeout_ms() -> u64 {
    30_000
}

/// Rejects any proof carrying a `status` or `verdict` field, at any depth.
///
/// The check runs before deserialisation so an otherwise-valid `Preuve` with
/// one extra status field cannot slip through (R-P1).
pub fn validate_preuve(value: &Value) -> std::result::Result<Preuve, String> {
    if let Some(field) = forbidden_field(value) {
        return Err(format!("champ `{field}` interdit dans une preuve (R-P1)"));
    }
    serde_json::from_value(value.clone()).map_err(|error| format!("preuve invalide : {error}"))
}

fn forbidden_field(value: &Value) -> Option<&'static str> {
    match value {
        Value::Object(map) => {
            for key in ["status", "verdict"] {
                if map.contains_key(key) {
                    return Some(key);
                }
            }
            map.values().find_map(forbidden_field)
        }
        Value::Array(items) => items.iter().find_map(forbidden_field),
        _ => None,
    }
}

fn resolve_program(program: &str, manifest_dir: &Path) -> Option<PathBuf> {
    let candidate = Path::new(program);
    if candidate.components().count() > 1 {
        let resolved = if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            manifest_dir.join(candidate)
        };
        return resolved.is_file().then_some(resolved);
    }
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(program))
            .find(|path| path.is_file())
    })
}

fn executable_sha256(manifest: &SidecarManifest, manifest_dir: &Path) -> Option<String> {
    let program = manifest.commande.first()?;
    let bytes = fs::read(resolve_program(program, manifest_dir)?).ok()?;
    Some(sha256_hex(&bytes))
}

/// A discovered sidecar, presented to the orchestrator as any other plugin.
#[derive(Clone)]
pub struct SidecarPlugin {
    manifest: SidecarManifest,
    manifest_dir: PathBuf,
    id: &'static str,
    version: &'static str,
    tests: &'static [&'static str],
    executable_sha256: Option<String>,
    /// Extra `analyser` parameters (file paths, timed actions) set by the
    /// caller. P-10 uses this to hand a speech journal to its sidecar without
    /// changing the shared `PluginOptions` contract.
    parametres_analyser: Value,
}

impl std::fmt::Debug for SidecarPlugin {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SidecarPlugin")
            .field("manifest", &self.manifest)
            .field("manifest_dir", &self.manifest_dir)
            .finish()
    }
}

impl SidecarPlugin {
    /// Loads a manifest. Invalid manifests are logged and skipped by
    /// [`discover`], so one bad plugin never breaks an audit.
    pub fn from_manifest(manifest: SidecarManifest, manifest_dir: &Path) -> Result<Self> {
        if manifest.id.is_empty() || manifest.commande.is_empty() {
            return Err(analysis_error(
                "sidecar",
                format!("manifeste invalide dans {}", manifest_dir.display()),
            ));
        }
        let executable_sha256 = executable_sha256(&manifest, manifest_dir);
        Ok(Self {
            id: Box::leak(manifest.id.clone().into_boxed_str()),
            version: Box::leak(manifest.version.clone().into_boxed_str()),
            tests: Box::leak(
                manifest
                    .tests
                    .iter()
                    .map(|test| Box::leak(test.clone().into_boxed_str()) as &str)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
            manifest,
            manifest_dir: manifest_dir.to_path_buf(),
            executable_sha256,
            parametres_analyser: Value::Null,
        })
    }

    pub fn manifest(&self) -> &SidecarManifest {
        &self.manifest
    }

    /// Merges extra JSON parameters into every `analyser` request, so a
    /// sidecar can receive heavy inputs by path (P-00 §7 bis) and the timed
    /// actions needed by P-10's correlation. Later keys win; object keys are
    /// merged shallowly while arrays replace earlier arrays.
    #[must_use]
    pub fn avec_parametres(mut self, parametres: Value) -> Self {
        if let (Some(existant), Some(ajoutes)) = (
            self.parametres_analyser.as_object_mut(),
            parametres.as_object(),
        ) {
            for (cle, valeur) in ajoutes {
                existant.insert(cle.clone(), valeur.clone());
            }
        } else {
            self.parametres_analyser = parametres;
        }
        self
    }

    /// Discovers `sidecar.toml` files directly under `root`, or one level down
    /// in `plugins/<id>/sidecar.toml` layout.
    pub fn discover_in(root: &Path) -> Vec<Self> {
        let mut paths: Vec<PathBuf> = Vec::new();
        if root.join("sidecar.toml").is_file() {
            paths.push(root.join("sidecar.toml"));
        } else if let Ok(entries) = fs::read_dir(root) {
            paths.extend(entries.filter_map(|entry| entry.ok()).filter_map(|entry| {
                let path = entry.path().join("sidecar.toml");
                path.is_file().then_some(path)
            }));
        }
        paths.sort();
        paths
            .into_iter()
            .filter_map(|path| match fs::read_to_string(&path) {
                Ok(text) => match toml::from_str::<SidecarManifest>(&text) {
                    Ok(manifest) => {
                        let dir = path.parent().unwrap_or(root).to_path_buf();
                        match Self::from_manifest(manifest, &dir) {
                            Ok(plugin) => Some(plugin),
                            Err(error) => {
                                tracing::warn!(%error, path = %path.display(), "sidecar ignoré");
                                None
                            }
                        }
                    }
                    Err(error) => {
                        tracing::warn!(%error, path = %path.display(), "manifeste sidecar invalide");
                        None
                    }
                },
                Err(error) => {
                    tracing::warn!(%error, path = %path.display(), "manifeste sidecar illisible");
                    None
                }
            })
            .collect()
    }

    async fn exchange(
        &self,
        method: &str,
        params: Value,
        options: &PluginOptions,
    ) -> Result<Value> {
        let command = self
            .manifest
            .commande
            .first()
            .ok_or_else(|| analysis_error(self.id, "commande sidecar absente du manifeste"))?;
        let args: Vec<String> = self.manifest.commande.iter().skip(1).cloned().collect();
        let mut child = tokio::process::Command::new(command)
            .args(&args)
            .current_dir(&self.manifest_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| {
                missing_proof(self.id, &format!("impossible de lancer {command}: {error}"))
            })?;

        let budget = Duration::from_millis(self.manifest.timeout_ms.max(1));
        let conversation = Self::converse(&mut child, method, params, options);
        let outcome = match tokio::time::timeout(budget, conversation).await {
            Ok(result) => result,
            Err(_) => {
                let _ = child.kill().await;
                return Err(missing_proof(
                    self.id,
                    &format!("timeout après {} ms", self.manifest.timeout_ms),
                ));
            }
        };
        if child.try_wait().ok().flatten().is_none() {
            let _ = child.kill().await;
        }
        outcome
    }

    async fn converse(
        child: &mut Child,
        method: &str,
        params: Value,
        options: &PluginOptions,
    ) -> Result<Value> {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| analysis_error("sidecar", "stdin du sidecar indisponible"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| analysis_error("sidecar", "stdout du sidecar indisponible"))?;
        let mut lines = BufReader::new(stdout).lines();

        let initialiser = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialiser",
            "params": { "protocole": PROTOCOL_VERSION, "options": options },
        });
        write_line(&mut stdin, &initialiser).await?;
        let init = read_response(&mut lines, 1).await?;
        tracing::debug!(plugin = ?init.get("id"), "sidecar initialisé");

        let request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": method,
            "params": params,
        });
        write_line(&mut stdin, &request).await?;
        let response = read_response(&mut lines, 2).await?;

        let stop = json!({ "jsonrpc": "2.0", "method": "arreter" });
        let _ = write_line(&mut stdin, &stop).await;
        let _ = stdin.shutdown().await;
        response
            .get("result")
            .cloned()
            .ok_or_else(|| analysis_error("sidecar", "réponse JSON-RPC sans resultat"))
    }
}

async fn write_line(stdin: &mut tokio::process::ChildStdin, value: &Value) -> Result<()> {
    let mut line = serde_json::to_string(value)
        .map_err(|error| analysis_error("sidecar", format!("JSON-RPC invalide : {error}")))?;
    line.push('\n');
    stdin
        .write_all(line.as_bytes())
        .await
        .map_err(|error| analysis_error("sidecar", format!("écriture JSON-RPC : {error}")))?;
    stdin
        .flush()
        .await
        .map_err(|error| analysis_error("sidecar", format!("flush JSON-RPC : {error}")))
}

async fn read_response<R: AsyncBufReadExt + Unpin>(
    lines: &mut tokio::io::Lines<R>,
    id: u64,
) -> Result<Value> {
    loop {
        let line = lines
            .next_line()
            .await
            .map_err(|error| analysis_error("sidecar", format!("lecture JSON-RPC : {error}")))?
            .ok_or_else(|| analysis_error("sidecar", "le sidecar a fermé stdout"))?;
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(&line)
            .map_err(|error| analysis_error("sidecar", format!("JSON invalide : {error}")))?;
        if value.get("method").and_then(Value::as_str) == Some("journal") {
            let params = value.get("params");
            tracing::info!(
                niveau = ?params.and_then(|p| p.get("niveau")),
                message = ?params.and_then(|p| p.get("message")),
                "sidecar"
            );
            continue;
        }
        if value.get("id").and_then(Value::as_u64) != Some(id) {
            continue;
        }
        if let Some(error) = value.get("error") {
            return Err(analysis_error(
                "sidecar",
                format!("erreur JSON-RPC : {error}"),
            ));
        }
        return Ok(value);
    }
}

#[async_trait]
impl AnalysisPlugin for SidecarPlugin {
    fn id(&self) -> &'static str {
        self.id
    }
    fn version(&self) -> &'static str {
        self.version
    }
    fn tests(&self) -> &'static [&'static str] {
        self.tests
    }
    fn kind(&self) -> PluginKind {
        PluginKind::Sidecar
    }
    fn commande(&self) -> Option<String> {
        Some(self.manifest.commande.join(" "))
    }
    fn executable_sha256(&self) -> Option<String> {
        self.executable_sha256.clone()
    }

    async fn analyze(
        &self,
        page: &PageCapture<'_>,
        options: &PluginOptions,
    ) -> Result<Vec<Preuve>> {
        let mut params = json!({
            "url": page.url,
            "sujets": [{ "type": "Page" }],
            "fichiers": {},
        });
        if let (Some(base), Some(extra)) =
            (params.as_object_mut(), self.parametres_analyser.as_object())
        {
            for (key, value) in extra {
                base.insert(key.clone(), value.clone());
            }
        }
        let result = self.exchange("analyser", params, options).await?;
        let proofs = result
            .get("preuves")
            .and_then(Value::as_array)
            .ok_or_else(|| missing_proof(self.id, "réponse sans preuves"))?;

        let mut accepted = Vec::with_capacity(proofs.len());
        let mut rejected = Vec::new();
        for value in proofs {
            match validate_preuve(value) {
                Ok(proof) => accepted.push(proof),
                Err(reason) => {
                    tracing::warn!(plugin = self.id, %reason, "preuve sidecar rejetée");
                    rejected.push(reason);
                }
            }
        }
        if accepted.is_empty() {
            let reason = if rejected.is_empty() {
                "aucune preuve".to_owned()
            } else {
                rejected.join(" ; ")
            };
            if rejected.iter().any(|reason| reason.contains("R-P1")) {
                return Err(analysis_error(
                    self.id,
                    format!("preuve rejetée : {reason}"),
                ));
            }
            return Err(missing_proof(self.id, &reason));
        }
        Ok(accepted)
    }
}

/// Discovers sidecars in `$RGAA_PLUGINS_DIR` then `~/.config/rgaa/plugins`.
pub fn discover_default() -> Vec<SidecarPlugin> {
    let mut roots = Vec::new();
    if let Some(dir) = std::env::var_os("RGAA_PLUGINS_DIR") {
        roots.push(PathBuf::from(dir));
    }
    roots.push(
        PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_owned()))
            .join(".config/rgaa/plugins"),
    );
    let mut plugins: Vec<SidecarPlugin> = Vec::new();
    for root in roots {
        for plugin in SidecarPlugin::discover_in(&root) {
            if plugins.iter().any(|known| known.id() == plugin.id()) {
                continue;
            }
            plugins.push(plugin);
        }
    }
    plugins.sort_by(|left, right| left.id().cmp(right.id()));
    plugins
}

/// Alias kept for callers that only need the discovery root.
pub fn discover(dir: &Path) -> Vec<SidecarPlugin> {
    SidecarPlugin::discover_in(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_status_is_rejected_too() {
        let value = json!({
            "plugin": "echo", "version_plugin": "0.1.0", "version_modele": null,
            "tests": [], "sujet": "Page", "portee": "Mesuree",
            "mesures": { "nested": { "status": "pass" } },
            "indice": null, "artefacts": [], "empreinte_entree": "x"
        });
        assert!(validate_preuve(&value).is_err());
    }
}
