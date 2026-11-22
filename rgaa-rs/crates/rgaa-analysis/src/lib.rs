//! P-00 — contract for analysis plugins.
//!
//! An analysis plugin produces [`Preuve`]s: measurements and artefacts, never
//! a status. The orchestrator decides the status from those proofs (R-P1).
//! Native plugins and sidecars share the same [`AnalysisPlugin`] interface;
//! callers assemble the ones they linked into a [`Registry`].

pub mod capture;
/// Single source of colour maths for every analysis plugin (WCAG 2.1,
/// CIEDE2000, alpha composition).
pub mod color;
pub mod error;
pub mod models;
pub mod plugin;
pub mod registry;
pub mod schema;
pub mod sidecar;
pub mod types;

pub use capture::{
    AxNode, AxTree, DomSnapshot, NoopResourceFetcher, PageCapture, ResourceFetcher, RgbaImage,
};
pub use error::{analysis_error, missing_proof, Result, RgaaError};
#[cfg(feature = "plugin-echo")]
pub use plugin::{echo_plugin, EchoPlugin};
pub use plugin::{AnalysisPlugin, ECHO_TEST, ECHO_VERSION};
pub use registry::{Registry, RegistryBuilder};
pub use types::{
    Artefact, Indice, ModelInfo, ModelSpec, PluginInfo, PluginKind, PluginOptions, Portee, Preuve,
    Rect, Subject,
};

use serde_json::Value;
use sha2::{Digest, Sha256};

/// Hex SHA-256, without a `sha256:` prefix.
pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Runs a plugin with a minimal capture built from a URL.
///
/// This is the P-00 convenience path used by the CLI and MCP tools, which do
/// not own a browser capture yet. Real plugins are called by the orchestrator
/// with a fully populated [`PageCapture`].
///
/// # Errors
///
/// Returns [`RgaaError::Analysis`] when a required model is absent, options
/// cannot be parsed, or the plugin fails.
pub async fn run_plugin(
    plugin: &dyn AnalysisPlugin,
    url: &str,
    options: &Value,
) -> Result<Vec<Preuve>> {
    for model in plugin.models() {
        models::ensure_present(plugin.id(), model)?;
    }
    let options: PluginOptions = serde_json::from_value(options.clone()).map_err(|error| {
        analysis_error(
            plugin.id(),
            format!("options de plugin invalides : {error}"),
        )
    })?;
    let dom = DomSnapshot::default();
    let resources = NoopResourceFetcher;
    let page = PageCapture {
        url,
        dom_snapshot: &dom,
        ax_tree: None,
        screenshot: None,
        dpr: 1.0,
        resources: &resources,
        lang_declared: None,
    };
    plugin.analyze(&page, &options).await
}
