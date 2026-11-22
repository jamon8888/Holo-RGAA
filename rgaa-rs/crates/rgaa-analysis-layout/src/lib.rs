//! P-07 — native `layout` analysis plugin.
//!
//! It compares the rendered page (DOM boxes + computed styles) with what is
//! exposed to assistive technology (AX tree). It returns [`rgaa_analysis::Preuve`]s
//! only: no status, no verdict (R-P1). It makes no LLM call; `avis_modele`
//! fields are left `None` as an extension point for a future second opinion.

pub mod actions;
pub mod headings;
pub mod landmarks;
pub mod mesures;
pub mod order;
pub mod schema;
pub mod sensory;
pub mod snapshot;
pub mod status;
pub mod tables;
pub mod widgets;

pub use actions::{DEFAULT_MAX_ACTIONS, FORBIDDEN_ACTION_WORDS};
pub use mesures::{Measurement, Mesures};
pub use snapshot::{Action, AxInfo, Node, Snapshot, Table, TableCell};

use async_trait::async_trait;
use rgaa_analysis::{
    analysis_error, sha256_hex, AnalysisPlugin, PageCapture, PluginOptions, Preuve, Result, Subject,
};

/// RGAA test identifiers served by this plugin (catalogue `rgaa-core`).
pub const LAYOUT_TESTS: &[&str] = &[
    "5.3.1", "5.6.1", "5.6.2", "7.1.1", "7.1.2", "7.1.3", "7.5.1", "7.5.2", "9.1.1", "9.1.3",
    "9.2.1", "10.3.1", "10.9.1", "10.10.1",
];
pub const LAYOUT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The P-07 plugin. Stateless apart from its configurable action bound.
#[cfg(feature = "plugin-layout")]
pub struct LayoutPlugin {
    max_actions: usize,
}

#[cfg(feature = "plugin-layout")]
impl Default for LayoutPlugin {
    fn default() -> Self {
        Self {
            max_actions: DEFAULT_MAX_ACTIONS,
        }
    }
}

#[cfg(feature = "plugin-layout")]
impl LayoutPlugin {
    /// Builds a plugin with a custom bound on simulated actions (spec §6.3).
    #[must_use]
    pub fn with_max_actions(max_actions: usize) -> Self {
        Self { max_actions }
    }
}

fn to_proof(measurement: Measurement, page: &PageCapture<'_>) -> Result<Preuve> {
    let input = serde_json::to_string(&measurement.mesures)
        .map_err(|error| analysis_error("layout", format!("mesures sérialisables : {error}")))?;
    let mesures = serde_json::from_str(&input)
        .map_err(|error| analysis_error("layout", format!("mesures sérialisables : {error}")))?;
    let fingerprint = format!(
        "layout\0{}\0{}\0{}",
        page.url,
        measurement.tests.join(","),
        input
    );
    Ok(Preuve {
        plugin: "layout".to_owned(),
        version_plugin: LAYOUT_VERSION.to_owned(),
        version_modele: None,
        tests: measurement.tests,
        sujet: Subject::Page,
        portee: measurement.portee,
        mesures,
        indice: measurement.indice,
        artefacts: measurement.artefacts,
        empreinte_entree: sha256_hex(fingerprint.as_bytes()),
    })
}

#[cfg(feature = "plugin-layout")]
#[async_trait]
impl AnalysisPlugin for LayoutPlugin {
    fn id(&self) -> &'static str {
        "layout"
    }

    fn version(&self) -> &'static str {
        LAYOUT_VERSION
    }

    fn tests(&self) -> &'static [&'static str] {
        LAYOUT_TESTS
    }

    async fn analyze(
        &self,
        page: &PageCapture<'_>,
        _options: &PluginOptions,
    ) -> Result<Vec<Preuve>> {
        let snapshot: Snapshot =
            serde_json::from_str(&page.dom_snapshot.json).map_err(|error| {
                analysis_error("layout", format!("DOM snapshot illisible : {error}"))
            })?;
        let lang = page
            .lang_declared
            .or(snapshot.lang.as_deref())
            .unwrap_or("fr");
        let rtl = snapshot.dir.as_deref() == Some("rtl");

        let mut proofs = Vec::with_capacity(8);
        proofs.push(to_proof(
            headings::analyze(&snapshot, page.ax_tree, lang),
            page,
        )?);
        proofs.push(to_proof(landmarks::analyze(&snapshot, page.ax_tree), page)?);
        proofs.push(to_proof(order::reading_measurement(&snapshot, rtl), page)?);
        for mesure in tables::header_measurements(&snapshot) {
            proofs.push(to_proof(mesure, page)?);
        }
        for mesure in tables::linearisation_measurements(&snapshot, rtl) {
            proofs.push(to_proof(mesure, page)?);
        }
        proofs.push(to_proof(sensory::analyze(&snapshot, lang), page)?);
        let widget = widgets::analyze(&snapshot, self.max_actions);
        if matches!(&widget.mesures, Mesures::Widget { observes, .. } if !observes.is_empty()) {
            proofs.push(to_proof(widget, page)?);
        }
        let status = status::analyze(&snapshot, self.max_actions);
        if matches!(
            &status.mesures,
            Mesures::Status { messages, actions_ignorees }
                if !messages.is_empty() || *actions_ignorees > 0
        ) {
            proofs.push(to_proof(status, page)?);
        }
        Ok(proofs)
    }
}

#[cfg(all(test, feature = "plugin-layout"))]
mod tests {
    use super::*;

    #[test]
    fn plugin_metadata_is_stable() {
        let plugin = LayoutPlugin::default();
        assert_eq!(plugin.id(), "layout");
        assert_eq!(plugin.tests(), LAYOUT_TESTS);
        assert_eq!(plugin.version(), LAYOUT_VERSION);
    }

    #[test]
    fn an_unreadable_snapshot_is_a_domain_error_not_a_panic() {
        let dom = rgaa_analysis::DomSnapshot::new("not json");
        let resources = rgaa_analysis::NoopResourceFetcher;
        let page = PageCapture {
            url: "https://example.test",
            dom_snapshot: &dom,
            ax_tree: None,
            screenshot: None,
            dpr: 1.0,
            resources: &resources,
            lang_declared: None,
        };
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime");
        let error = runtime
            .block_on(LayoutPlugin::default().analyze(&page, &PluginOptions::default()))
            .expect_err("invalid snapshot");
        assert!(error.to_string().contains("DOM snapshot illisible"));
    }
}
