//! P-06 acceptance tests. Expected values come from `06-langue.md` §4, not from
//! the implementation: a French page declared English is Suspect, an unmarked
//! English quote is a 8.7 candidate, a marked one is not, a French paragraph
//! marked German is a 8.8 Suspect, and a loanword phrase is excluded.

use rgaa_analysis::{
    AnalysisPlugin, DomSnapshot, NoopResourceFetcher, PageCapture, PluginOptions, Preuve,
};
use rgaa_analysis_lang::{LangPlugin, NoCandidateFilter};
use serde_json::{json, Value};

const FR_PAGE: &str = "L'accessibilité numérique est un droit pour toutes les personnes, notamment les personnes handicapées.";
const FR_SUPPORT: &str =
    "Ce document présente les critères du référentiel général d'amélioration de l'accessibilité.";
const FR_PARAGRAPH: &str =
    "La langue par défaut de la page doit être pertinente au regard du contenu.";
const EN_QUOTE: &str = "Stay hungry, stay foolish";

fn n(
    id: &str,
    tag: &str,
    parent: Option<&str>,
    text: &str,
    lang: Option<&str>,
    selector: Option<&str>,
) -> Value {
    json!({
        "id": id,
        "tag": tag,
        "parent": parent,
        "text": text,
        "lang": lang,
        "selector": selector,
    })
}

fn snapshot(nodes: Vec<Value>) -> String {
    json!({ "nodes": nodes }).to_string()
}

fn page<'a>(
    dom: &'a DomSnapshot,
    declared: Option<&'a str>,
    resources: &'a NoopResourceFetcher,
) -> PageCapture<'a> {
    PageCapture {
        url: "https://example.test",
        dom_snapshot: dom,
        ax_tree: None,
        screenshot: None,
        dpr: 1.0,
        resources,
        lang_declared: declared,
    }
}

fn proofs_for<'a>(proofs: &'a [Preuve], test: &str) -> Vec<&'a Preuve> {
    proofs
        .iter()
        .filter(|proof| proof.tests.iter().any(|candidate| candidate == test))
        .collect()
}

#[tokio::test]
async fn a_french_page_declared_english_is_suspect_for_84() {
    let dom = DomSnapshot::new(snapshot(vec![
        n("html", "html", None, "", Some("en"), None),
        n("p1", "p", Some("html"), FR_PAGE, None, Some("#p1")),
        n("p2", "p", Some("html"), FR_SUPPORT, None, Some("#p2")),
        n("p3", "p", Some("html"), FR_PARAGRAPH, None, Some("#p3")),
    ]));
    let resources = NoopResourceFetcher;
    let capture = page(&dom, Some("en"), &resources);

    let proofs = LangPlugin::new()
        .analyze(&capture, &PluginOptions::default())
        .await
        .expect("proofs");
    let page_proofs = proofs_for(&proofs, "8.4.1");
    assert_eq!(page_proofs.len(), 1, "{proofs:?}");
    let mesures = &page_proofs[0].mesures;
    assert_eq!(mesures["declaree"], "en");
    assert_eq!(mesures["detectee"], "fr");
    assert!(
        mesures["confiance"].as_f64().unwrap_or(0.0) > 0.8,
        "{mesures}"
    );
    assert_eq!(
        page_proofs[0].indice,
        Some(rgaa_analysis::Indice::Suspect {
            raison: "page détectée en fr, déclarée en".to_owned(),
        })
    );
}

#[tokio::test]
async fn an_unmarked_english_quote_is_a_87_candidate() {
    let dom = DomSnapshot::new(snapshot(vec![
        n("html", "html", None, "", Some("fr"), None),
        n("p1", "p", Some("html"), FR_PAGE, None, Some("#p1")),
        n("p2", "p", Some("html"), FR_SUPPORT, None, Some("#p2")),
        n(
            "quote-3",
            "blockquote",
            Some("html"),
            EN_QUOTE,
            None,
            Some("#quote-3"),
        ),
    ]));
    let resources = NoopResourceFetcher;
    let capture = page(&dom, Some("fr"), &resources);

    let proofs = LangPlugin::new()
        .analyze(&capture, &PluginOptions::default())
        .await
        .expect("proofs");
    let candidates = proofs_for(&proofs, "8.7.1");
    assert_eq!(candidates.len(), 1, "{proofs:?}");
    let mesures = &candidates[0].mesures;
    assert_eq!(mesures["detectee"], "en");
    assert_eq!(mesures["lang_herite"], "fr");
    assert_eq!(mesures["extrait"], EN_QUOTE);
    assert!(
        mesures["confiance"].as_f64().unwrap_or(0.0) > 0.9,
        "{mesures}"
    );

    let page_proof = &proofs_for(&proofs, "8.4.1")[0];
    let divergents = page_proof.mesures["blocs_divergents"]
        .as_array()
        .expect("blocs_divergents");
    assert_eq!(divergents.len(), 1);
    assert_eq!(divergents[0]["selecteur"], "#quote-3");
}

#[tokio::test]
async fn the_same_quote_marked_lang_en_is_not_a_candidate() {
    let dom = DomSnapshot::new(snapshot(vec![
        n("html", "html", None, "", Some("fr"), None),
        n("p1", "p", Some("html"), FR_PAGE, None, Some("#p1")),
        n("p2", "p", Some("html"), FR_SUPPORT, None, Some("#p2")),
        n(
            "quote-3",
            "blockquote",
            Some("html"),
            EN_QUOTE,
            Some("en"),
            Some("#quote-3"),
        ),
    ]));
    let resources = NoopResourceFetcher;
    let capture = page(&dom, Some("fr"), &resources);

    let proofs = LangPlugin::new()
        .analyze(&capture, &PluginOptions::default())
        .await
        .expect("proofs");
    assert!(proofs_for(&proofs, "8.7.1").is_empty(), "{proofs:?}");
    assert!(proofs_for(&proofs, "8.8.1").is_empty(), "{proofs:?}");
    let page_proof = &proofs_for(&proofs, "8.4.1")[0];
    assert_eq!(
        page_proof.mesures["blocs_divergents"]
            .as_array()
            .expect("blocs_divergents")
            .len(),
        0
    );
}

#[tokio::test]
async fn a_french_paragraph_marked_lang_de_is_a_88_suspect() {
    let dom = DomSnapshot::new(snapshot(vec![
        n("html", "html", None, "", Some("fr"), None),
        n("p1", "p", Some("html"), FR_PAGE, None, Some("#p1")),
        n(
            "p2",
            "p",
            Some("html"),
            FR_PARAGRAPH,
            Some("de"),
            Some("#p2"),
        ),
    ]));
    let resources = NoopResourceFetcher;
    let capture = page(&dom, Some("fr"), &resources);

    let proofs = LangPlugin::new()
        .analyze(&capture, &PluginOptions::default())
        .await
        .expect("proofs");
    let suspects = proofs_for(&proofs, "8.8.1");
    assert_eq!(suspects.len(), 1, "{proofs:?}");
    assert_eq!(suspects[0].mesures["lang_herite"], "de");
    assert_eq!(suspects[0].mesures["detectee"], "fr");
    assert!(matches!(
        suspects[0].indice,
        Some(rgaa_analysis::Indice::Suspect { .. })
    ));
}

#[tokio::test]
async fn a_loanword_phrase_produces_no_candidate() {
    // Declared English on purpose: without the exclusions, this French sentence
    // would be a 8.7 candidate. « week-end » and « jogging » are excluded.
    let dom = DomSnapshot::new(snapshot(vec![
        n("html", "html", None, "", Some("en"), None),
        n(
            "p1",
            "p",
            Some("html"),
            "Le week-end, je fais du jogging",
            None,
            Some("#p1"),
        ),
    ]));
    let resources = NoopResourceFetcher;
    let capture = page(&dom, Some("en"), &resources);

    let proofs = LangPlugin::new()
        .analyze(&capture, &PluginOptions::default())
        .await
        .expect("proofs");
    assert!(proofs_for(&proofs, "8.7.1").is_empty(), "{proofs:?}");
}

#[tokio::test]
async fn declared_language_is_normalised_to_its_primary_bcp47_subtag() {
    let dom = DomSnapshot::new(snapshot(vec![
        n("html", "html", None, "", Some("fr"), None),
        n("p1", "p", Some("html"), FR_PAGE, None, Some("#p1")),
    ]));
    let resources = NoopResourceFetcher;
    let capture = page(&dom, Some("fr-CA"), &resources);

    let proofs = LangPlugin::new()
        .analyze(&capture, &PluginOptions::default())
        .await
        .expect("proofs");
    assert_eq!(proofs_for(&proofs, "8.4.1")[0].mesures["declaree"], "fr");
}

#[tokio::test]
async fn the_extension_point_can_suppress_every_candidate() {
    struct DropAll;
    impl rgaa_analysis_lang::CandidateFilter for DropAll {
        fn retain(&self, candidats: &mut Vec<rgaa_analysis_lang::mesures::BlocDivergent>) {
            candidats.clear();
        }
    }
    let _ = NoCandidateFilter;

    let dom = DomSnapshot::new(snapshot(vec![
        n("html", "html", None, "", Some("fr"), None),
        n("p1", "p", Some("html"), FR_PAGE, None, Some("#p1")),
        n(
            "quote-3",
            "blockquote",
            Some("html"),
            EN_QUOTE,
            None,
            Some("#quote-3"),
        ),
    ]));
    let resources = NoopResourceFetcher;
    let capture = page(&dom, Some("fr"), &resources);

    let proofs = LangPlugin::with_filter(Box::new(DropAll))
        .analyze(&capture, &PluginOptions::default())
        .await
        .expect("proofs");
    assert!(proofs_for(&proofs, "8.7.1").is_empty(), "{proofs:?}");
    assert!(proofs_for(&proofs, "8.4.1")[0].mesures["blocs_divergents"]
        .as_array()
        .expect("blocs_divergents")
        .is_empty());
}

#[tokio::test]
async fn proofs_never_carry_a_status_or_a_verdict() {
    let dom = DomSnapshot::new(snapshot(vec![n(
        "html",
        "html",
        None,
        "",
        Some("fr"),
        None,
    )]));
    let resources = NoopResourceFetcher;
    let capture = page(&dom, Some("fr"), &resources);
    let proofs = LangPlugin::new()
        .analyze(&capture, &PluginOptions::default())
        .await
        .expect("proofs");
    let text = serde_json::to_string(&proofs).expect("serialize");
    assert!(!text.contains("\"status\""), "{text}");
    assert!(!text.contains("\"verdict\""), "{text}");
}
