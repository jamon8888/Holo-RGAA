//! The single interface every analysis plugin implements (P-00 §4).

use crate::capture::PageCapture;
use crate::error::Result;
use crate::types::{
    Artefact, ModelInfo, PluginInfo, PluginKind, PluginOptions, Portee, Preuve, Subject,
};
use crate::{models, sha256_hex};
use async_trait::async_trait;
use serde_json::json;

/// RGAA test served by the P-00 dummy plugin.
pub const ECHO_TEST: &str = "1.1.1";
/// Version shared by the native plugin and `examples/sidecar-echo/sidecar.toml`.
pub const ECHO_VERSION: &str = "0.1.0";

/// The contract. Implementations produce proofs only — never a status (R-P1).
#[async_trait]
pub trait AnalysisPlugin: Send + Sync {
    /// Stable, kebab-case identifier.
    fn id(&self) -> &'static str;
    fn version(&self) -> &'static str;
    /// RGAA tests served, as keys of the `rgaa-core` catalogue.
    fn tests(&self) -> &'static [&'static str];
    /// Local models required; none are downloaded during an audit (R-P3).
    fn models(&self) -> &'static [crate::types::ModelSpec] {
        &[]
    }
    fn kind(&self) -> PluginKind {
        PluginKind::Natif
    }
    /// Exact command line, shown by `rgaa plugins list` for sidecars.
    fn commande(&self) -> Option<String> {
        None
    }
    /// SHA-256 of the executable, for change warnings between audits.
    fn executable_sha256(&self) -> Option<String> {
        None
    }

    /// Description used by `rgaa plugins list` and the MCP listing tool.
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: self.id().to_owned(),
            version: self.version().to_owned(),
            tests: self.tests().iter().map(|test| (*test).to_owned()).collect(),
            models: self
                .models()
                .iter()
                .map(|model| ModelInfo {
                    nom: model.nom.to_owned(),
                    licence: model.licence.to_owned(),
                    present: models::is_present(model),
                })
                .collect(),
            kind: self.kind(),
            commande: self.commande(),
            sha256_executable: self.executable_sha256(),
        }
    }

    async fn analyze(&self, page: &PageCapture<'_>, options: &PluginOptions)
        -> Result<Vec<Preuve>>;
}

/// The canonical echo proof. The Python example sidecar reproduces these
/// bytes exactly, so acceptance criterion 7 compares two equal proofs.
pub fn echo_proof(url: &str, options: &PluginOptions) -> Preuve {
    let message = options.message.as_deref().unwrap_or("echo");
    let empreinte_entree = sha256_hex(format!("echo\0{url}\0{message}").as_bytes());
    let artefact_hash = sha256_hex(format!("echo:{url}:{message}").as_bytes());
    Preuve {
        plugin: "echo".to_owned(),
        version_plugin: ECHO_VERSION.to_owned(),
        version_modele: None,
        tests: vec![ECHO_TEST.to_owned()],
        sujet: Subject::Page,
        portee: Portee::Mesuree,
        mesures: json!({ "echo": { "url": url, "message": message } }),
        indice: None,
        artefacts: vec![Artefact {
            nom: "echo.txt".to_owned(),
            media_type: "text/plain".to_owned(),
            chemin_bundle: format!("analysis/echo/{artefact_hash}.txt"),
            sha256: artefact_hash,
        }],
        empreinte_entree,
    }
}

/// Builds the P-00 dummy plugin for an explicit [`crate::Registry`].
#[cfg(feature = "plugin-echo")]
#[must_use]
pub fn echo_plugin() -> Box<dyn AnalysisPlugin> {
    Box::new(EchoPlugin)
}

/// P-00 dummy native plugin, compiled under `feature = "plugin-echo"`.
#[cfg(feature = "plugin-echo")]
pub struct EchoPlugin;

#[cfg(feature = "plugin-echo")]
#[async_trait]
impl AnalysisPlugin for EchoPlugin {
    fn id(&self) -> &'static str {
        "echo"
    }
    fn version(&self) -> &'static str {
        ECHO_VERSION
    }
    fn tests(&self) -> &'static [&'static str] {
        &[ECHO_TEST]
    }
    async fn analyze(
        &self,
        page: &PageCapture<'_>,
        options: &PluginOptions,
    ) -> Result<Vec<Preuve>> {
        Ok(vec![echo_proof(page.url, options)])
    }
}
