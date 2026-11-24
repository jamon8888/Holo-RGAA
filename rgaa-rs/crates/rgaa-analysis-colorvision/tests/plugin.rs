//! The plugin produces proofs only (R-P1), with the `mesures` shape of P-02 §5.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use rgaa_analysis::capture::{NoopResourceFetcher, PageCapture, RgbaImage};
use rgaa_analysis::types::{PluginOptions, Portee};
use rgaa_analysis::AnalysisPlugin;
use rgaa_analysis_colorvision::plugin::{
    images_for_model, ColorVisionOptions, ColorVisionPlugin, TESTS,
};
use serde_json::Value;

/// Owns the DOM snapshot and resource fetcher a `PageCapture` borrows from.
struct PageParts {
    dom: rgaa_analysis::DomSnapshot,
    resources: NoopResourceFetcher,
}

impl PageParts {
    fn new(dom: &str) -> Self {
        Self {
            dom: rgaa_analysis::DomSnapshot::new(dom),
            resources: NoopResourceFetcher,
        }
    }
}

fn page<'a>(parts: &'a PageParts, screenshot: Option<&'a RgbaImage>) -> PageCapture<'a> {
    PageCapture {
        url: "https://example.test/page",
        dom_snapshot: &parts.dom,
        ax_tree: None,
        screenshot,
        dpr: 1.0,
        resources: &parts.resources,
        lang_declared: Some("fr"),
    }
}

fn link_dom() -> String {
    r##"{
      "elements": [{
        "tag": "a", "selecteur": "p a.more", "xpath": "/html/body/p/a",
        "bbox": {"x": 10, "y": 20, "width": 80, "height": 18},
        "styles": {"color": "#0000ee", "text-decoration-line": "none",
                   "font-weight": "400", "font-size": "16px"},
        "text": "En savoir plus", "parent_text": "Lisez cet article. ",
        "parent_styles": {"color": "#454545", "font-weight": "400", "font-size": "16px"},
        "icon": false
      }]
    }"##
    .to_owned()
}

fn error_dom(message: &str) -> String {
    format!(
        r##"{{
          "elements": [{{
            "tag": "input", "selecteur": "#email", "xpath": "/html/body/form/input[1]",
            "bbox": {{"x": 0, "y": 0, "width": 200, "height": 30}},
            "styles": {{"border-color": "#ff0000", "color": "#000000"}},
            "error_message": "{message}"
          }}]
        }}"##
    )
}

fn assert_no_status(value: &Value) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let key = key.to_ascii_lowercase();
                assert!(
                    key != "status" && key != "verdict" && key != "statut",
                    "a proof must never carry a status: found {key}"
                );
                assert_no_status(child);
            }
        }
        Value::Array(items) => items.iter().for_each(assert_no_status),
        _ => {}
    }
}

#[tokio::test]
async fn the_plugin_serves_31_and_produces_only_proofs() {
    let plugin = ColorVisionPlugin::default();
    assert!(TESTS.iter().all(|test| test.starts_with("3.1.")));
    assert_eq!(plugin.tests(), TESTS);
    assert!(plugin.models().is_empty());

    let dom = link_dom();
    let parts = PageParts::new(&dom);
    let page = page(&parts, None);
    let proofs = plugin
        .analyze(&page, &PluginOptions::default())
        .await
        .expect("analyze");
    assert_eq!(
        proofs.len(),
        2,
        "one measured + one estimated proof per candidate"
    );
    assert_eq!(proofs[0].portee, Portee::Mesuree);
    assert_eq!(proofs[1].portee, Portee::Estimee);
    assert_eq!(proofs[1].tests, vec!["3.1.1"]);
    assert_eq!(proofs[1].plugin, "colorvision");
    assert_eq!(proofs[1].version_plugin, "0.1.0");
    assert!(!proofs[1].empreinte_entree.is_empty());

    let serialized = serde_json::to_value(&proofs).expect("serialize");
    assert_no_status(&serialized);

    let mesures = &proofs[1].mesures;
    assert_eq!(mesures["candidat"], "lien-sans-soulignement");
    // #0000ee on #454545 is 1.02:1 (F73 threshold is 3:1). The WebAIM
    // #0000ee on #000000 = 2.23:1 reference is checked in `oracle.rs`.
    assert!((mesures["ratio_lien_texte"].as_f64().expect("ratio") - 1.02).abs() < 0.01);
    let sims = mesures["simulations"].as_array().expect("simulations");
    assert_eq!(sims.len(), 4);
    for sim in sims {
        assert!(sim["type"].is_string());
        assert!(sim["delta_e"].is_number());
        assert!(sim["indiscernable"].is_boolean());
    }
    // The grey link/text pair is indiscernible under achromatopsia.
    let grey = sims
        .iter()
        .find(|sim| sim["type"] == "achromatopsia")
        .expect("achromatopsia simulation");
    assert_eq!(grey["indiscernable"], true);
}

#[tokio::test]
async fn an_error_field_with_a_text_message_is_rassurant_not_a_verdict() {
    let dom = error_dom("Adresse électronique invalide");
    let parts = PageParts::new(&dom);
    let page = page(&parts, None);
    let proofs = ColorVisionPlugin::default()
        .analyze(&page, &PluginOptions::default())
        .await
        .expect("analyze");
    let measured = &proofs[0];
    assert_eq!(measured.portee, Portee::Mesuree);
    let json = serde_json::to_value(measured).expect("serialize");
    let reason = json["indice"]["Rassurant"]["raison"]
        .as_str()
        .expect("raison");
    assert_eq!(reason, "texte d'erreur présent");
    assert_eq!(measured.mesures["candidat"], "champ-erreur");
    assert_no_status(&json);
}

#[tokio::test]
async fn a_simulation_where_information_collapses_yields_vignettes() {
    let dom = link_dom();
    let image = RgbaImage {
        width: 100,
        height: 40,
        pixels: vec![255; 100 * 40 * 4],
    };
    let parts = PageParts::new(&dom);
    let page = page(&parts, Some(&image));
    let proofs = ColorVisionPlugin::default()
        .analyze(&page, &PluginOptions::default())
        .await
        .expect("analyze");
    let estimated = &proofs[1];
    assert_eq!(
        estimated.artefacts.len(),
        2,
        "original + achromatopsia vignette"
    );
    for artefact in &estimated.artefacts {
        assert_eq!(artefact.media_type, "image/png");
        assert!(artefact.chemin_bundle.starts_with("analysis/colorvision/"));
        assert_eq!(artefact.sha256.len(), 64);
        let bytes = STANDARD
            .decode(artefact.contenu_base64.as_deref().expect("PNG payload"))
            .expect("base64 PNG");
        assert_eq!(rgaa_analysis::sha256_hex(&bytes), artefact.sha256);
    }
    assert_eq!(images_for_model(&proofs), 2);
}

#[tokio::test]
async fn severity_and_threshold_are_configurable() {
    let plugin = ColorVisionPlugin::new(ColorVisionOptions {
        severite: 0.5,
        seuil_delta_e: 0.0,
        hamming_max: 0,
        llm: false,
    });
    let dom = link_dom();
    let parts = PageParts::new(&dom);
    let page = page(&parts, None);
    let proofs = plugin
        .analyze(&page, &PluginOptions::default())
        .await
        .expect("analyze");
    let sims = proofs[1].mesures["simulations"]
        .as_array()
        .expect("simulations");
    assert_eq!(sims.len(), 4);
    assert!(
        sims.iter().all(|sim| sim["indiscernable"] == false),
        "a zero threshold must mark nothing indiscernible"
    );
}

#[test]
fn only_indiscernible_subjects_are_sent_to_the_model() {
    // 10 candidates on a full-page capture, one of which collapses in grey.
    let mut elements = String::new();
    for i in 0..10 {
        if i > 0 {
            elements.push(',');
        }
        let colour = if i == 0 { "#4a4a4a" } else { "#ff0000" };
        elements.push_str(&format!(
            r##"{{
              "tag": "a", "selecteur": "p a:nth-of-type({i})",
              "xpath": "/html/body/p/a[{i}]",
              "bbox": {{"x": {i}, "y": 0, "width": 20, "height": 10}},
              "styles": {{"color": "{colour}", "text-decoration-line": "none",
                          "font-weight": "400", "font-size": "16px"}},
              "text": "Lien", "parent_text": "Texte ",
              "parent_styles": {{"color": "#454545", "font-weight": "400", "font-size": "16px"}},
              "icon": false
            }}"##
        ));
    }
    let dom = format!("{{\"elements\": [{elements}]}}");
    let image = RgbaImage {
        width: 200,
        height: 20,
        pixels: vec![255; 200 * 20 * 4],
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let proofs = runtime.block_on(async {
        let parts = PageParts::new(&dom);
        let page = page(&parts, Some(&image));
        ColorVisionPlugin::default()
            .analyze(&page, &PluginOptions::default())
            .await
            .expect("analyze")
    });
    let candidates = proofs.len() / 2;
    assert_eq!(candidates, 10);
    let sent = images_for_model(&proofs);
    // Full-page capture would be 10 images; the plugin sends at most 3 per
    // subject and only for collapsed subjects — here 2 (original + grey).
    assert!(
        sent as f64 <= (candidates as f64) * 0.3,
        "expected >= 70% fewer model images, got {sent} for {candidates} candidates"
    );
}
