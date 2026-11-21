//! P-01 — native `contrast` analysis plugin.
//!
//! It measures WCAG 2.1 / RGAA 3.2 and 3.3 contrast on a page already captured
//! by the orchestrator and returns [`Preuve`]s only: no status, no verdict
//! (R-P1). The WCAG formula lives in [`rgaa_analysis::color`], the single
//! implementation shared by every analysis plugin.
//!
//! # Snapshot format
//!
//! [`rgaa_analysis::DomSnapshot::json`] must be an object with `texts` and
//! `components` arrays, as documented in [`snapshot`]. Field names match the
//! computed-style collector of PR #456 so the same DOM walk feeds both tools.

pub mod components;
pub mod css;
pub mod mesures;
pub mod pixels;
pub mod schema;
pub mod snapshot;

// Convenience re-exports of the canonical module in `rgaa-analysis`.
pub use mesures::{Etape, Etat, Mesures};
pub use rgaa_analysis::color::{contrast_ratio, parse_color, relative_luminance, Color};

use async_trait::async_trait;
use indexmap::IndexMap;
use mesures::Measurement;
use rgaa_analysis::{
    analysis_error, AnalysisPlugin, PageCapture, PluginOptions, Preuve, Result, Subject,
};
use snapshot::{ComponentNode, Snapshot, TextNode};

/// RGAA test identifiers served by this plugin (catalogue `rgaa-core`,
/// critères 3.2 et 3.3).
pub const CONTRAST_TESTS: &[&str] = &[
    "3.2.1", "3.2.2", "3.2.3", "3.2.4", "3.3.1", "3.3.2", "3.3.3",
];
pub const CONTRAST_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The P-01 plugin. Stateless, like every native analysis plugin.
pub struct ContrastPlugin;

fn to_proof(
    measurement: Measurement,
    subject: Subject,
    page: &PageCapture<'_>,
    node_json: &str,
) -> Result<Preuve> {
    let mesures = serde_json::to_value(&measurement.mesures)
        .map_err(|error| analysis_error("contrast", format!("mesures sérialisables : {error}")))?;
    let fingerprint = format!(
        "contrast\0{}\0{}\0{node_json}",
        page.url,
        serde_json::to_string(&measurement.mesures.etat).map_err(|error| {
            analysis_error("contrast", format!("état sérialisable : {error}"))
        })?
    );
    Ok(Preuve {
        plugin: "contrast".to_owned(),
        version_plugin: CONTRAST_VERSION.to_owned(),
        version_modele: None,
        tests: measurement.tests,
        sujet: subject,
        portee: measurement.portee,
        mesures,
        indice: measurement.indice,
        artefacts: measurement.artefacts,
        empreinte_entree: rgaa_analysis::sha256_hex(fingerprint.as_bytes()),
    })
}

fn subject_of(selecteur: &str, xpath: &str, bbox: Option<rgaa_analysis::Rect>) -> Subject {
    Subject::Element {
        selecteur: selecteur.to_owned(),
        xpath: xpath.to_owned(),
        bbox: bbox.unwrap_or_default(),
    }
}

fn text_proof(node: &TextNode, page: &PageCapture<'_>, etat: Etat) -> Result<(String, Preuve)> {
    let measurement = css::measure_text(node, etat, page);
    let key = format!("text\0{}\0{}", node.selecteur, node.xpath);
    let node_json = serde_json::to_string(node).map_err(|error| {
        analysis_error("contrast", format!("nœud textuel sérialisable : {error}"))
    })?;
    let proof = to_proof(
        measurement,
        subject_of(&node.selecteur, &node.xpath, node.bbox),
        page,
        &node_json,
    )?;
    Ok((key, proof))
}

fn component_proof(
    node: &ComponentNode,
    page: &PageCapture<'_>,
    etat: Etat,
) -> Result<(String, Preuve)> {
    let measurement = components::measure_component(node, etat, page);
    let key = format!("component\0{}\0{}", node.selecteur, node.xpath);
    let node_json = serde_json::to_string(node)
        .map_err(|error| analysis_error("contrast", format!("composant sérialisable : {error}")))?;
    let proof = to_proof(
        measurement,
        subject_of(&node.selecteur, &node.xpath, node.bbox),
        page,
        &node_json,
    )?;
    Ok((key, proof))
}

#[async_trait]
impl AnalysisPlugin for ContrastPlugin {
    fn id(&self) -> &'static str {
        "contrast"
    }

    fn version(&self) -> &'static str {
        CONTRAST_VERSION
    }

    fn tests(&self) -> &'static [&'static str] {
        CONTRAST_TESTS
    }

    async fn analyze(
        &self,
        page: &PageCapture<'_>,
        _options: &PluginOptions,
    ) -> Result<Vec<Preuve>> {
        let snapshot: Snapshot =
            serde_json::from_str(&page.dom_snapshot.json).map_err(|error| {
                analysis_error("contrast", format!("DOM snapshot illisible : {error}"))
            })?;
        let etat = snapshot.etat.unwrap_or_default();
        // IndexMap keeps DOM order for proofs, including across replayed states.
        let mut grouped: IndexMap<String, Vec<Preuve>> = IndexMap::new();
        for node in &snapshot.texts {
            let (key, proof) = text_proof(node, page, etat)?;
            grouped.entry(key).or_default().push(proof);
        }
        for node in &snapshot.components {
            let (key, proof) = component_proof(node, page, etat)?;
            grouped.entry(key).or_default().push(proof);
        }
        Ok(grouped.into_values().flatten().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_metadata_is_stable() {
        let plugin = ContrastPlugin;
        assert_eq!(plugin.id(), "contrast");
        assert_eq!(plugin.tests(), CONTRAST_TESTS);
        assert_eq!(plugin.version(), CONTRAST_VERSION);
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
            .block_on(ContrastPlugin.analyze(&page, &PluginOptions::default()))
            .expect_err("invalid snapshot");
        assert!(error.to_string().contains("DOM snapshot illisible"));
    }
}
