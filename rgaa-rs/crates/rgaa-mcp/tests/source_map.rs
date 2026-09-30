//! Behaviour of the `source_map` tool (ticket #165).
//!
//! The fixtures under `tests/fixtures/source_map/` are deliberately small
//! *templates*, not rendered pages: the findings below carry the markup a
//! browser would report, which is not byte-identical to the source, and the
//! assertions pin down both what the matcher resolves and what it refuses to.

use rgaa_mcp::{
    map_findings, MatchConfidence, SourceFlavorDto, SourceMapFindingInput, SourceMapResponse,
    ToolServer, UnmappableReason,
};

fn fixture(name: &str) -> String {
    format!(
        "{}/tests/fixtures/source_map/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn finding(id: &str, html: &str) -> SourceMapFindingInput {
    SourceMapFindingInput {
        id: id.into(),
        selector: None,
        html: Some(html.into()),
    }
}

fn run(root: &str, findings: Vec<SourceMapFindingInput>) -> SourceMapResponse {
    map_findings(&fixture(root), &findings).expect("source_map should scan the fixture tree")
}

#[test]
fn the_tool_is_registered_on_the_macro_router() {
    let names: Vec<String> = ToolServer::tool_router()
        .list_all()
        .into_iter()
        .map(|tool| tool.name.to_string())
        .collect();
    assert!(
        names.iter().any(|name| name == "source_map"),
        "source_map missing from the stdio tool router: {names:?}"
    );
}

#[test]
fn a_react_alt_attribute_is_traced_to_its_jsx_line() {
    let response = run(
        "react-app",
        vec![finding(
            "r1",
            r#"<img class="kpi-chart" src="/static/charts/revenue-2024.png" alt="Chiffre d'affaires par trimestre">"#,
        )],
    );
    assert!(response.unmappable.is_empty(), "{:?}", response.unmappable);
    let hit = &response.mapped[0];
    assert_eq!(hit.finding_id, "r1");
    assert_eq!(hit.source_location.file, "src/components/Dashboard.jsx");
    // The attribute sits three lines below the `<img` that owns it, which is
    // exactly the multi-line formatting the lookback window exists for.
    assert_eq!(hit.source_location.line, 11);
    assert_eq!(hit.source_location.column, 14);
    assert_eq!(hit.confidence, MatchConfidence::High);
    assert_eq!(hit.framework, SourceFlavorDto::ReactJsx);
    assert_eq!(hit.matched_on, "alt=\"Chiffre d'affaires par trimestre\"");
}

#[test]
fn a_react_id_wins_over_the_weaker_literals_on_the_same_element() {
    let response = run(
        "react-app",
        vec![finding(
            "r2",
            r#"<button class="btn btn-primary" id="export-csv">Exporter</button>"#,
        )],
    );
    let hit = &response.mapped[0];
    assert_eq!(hit.source_location.line, 13);
    // `btn-primary` and the text also occur in Card.jsx; picking the id keeps
    // this unambiguous instead of falling into AmbiguousMatch.
    assert_eq!(hit.matched_on, "id=\"export-csv\"");
    assert_eq!(hit.confidence, MatchConfidence::High);
}

#[test]
fn a_vanilla_html_alt_and_asset_are_traced_to_their_lines() {
    let response = run(
        "vanilla-site",
        vec![
            finding(
                "h1",
                r#"<img src="/img/logo-mairie.svg" alt="Mairie de Villeneuve">"#,
            ),
            finding("h2", r#"<img src="/img/plan-acces.png">"#),
        ],
    );
    assert!(response.unmappable.is_empty(), "{:?}", response.unmappable);
    assert_eq!(response.mapped[0].source_location.file, "index.html");
    assert_eq!(response.mapped[0].source_location.line, 9);
    assert_eq!(response.mapped[0].framework, SourceFlavorDto::Html);
    assert_eq!(response.mapped[1].source_location.line, 15);
    assert_eq!(response.mapped[1].matched_on, "src=\"…/plan-acces.png\"");
}

#[test]
fn a_vue_single_file_component_is_reported_as_vue() {
    let response = run(
        "vue-app",
        vec![finding(
            "v1",
            r#"<img class="avatar" src="/img/avatar-default.png" alt="Photo de profil">"#,
        )],
    );
    let hit = &response.mapped[0];
    assert_eq!(hit.source_location.file, "src/ProfileCard.vue");
    assert_eq!(hit.source_location.line, 3);
    assert_eq!(hit.framework, SourceFlavorDto::VueSfc);
}

#[test]
fn an_angular_component_template_is_reported_as_angular() {
    let response = run(
        "angular-app",
        vec![finding(
            "a1",
            r#"<button class="menu-toggle" aria-label="Ouvrir le menu principal"></button>"#,
        )],
    );
    let hit = &response.mapped[0];
    assert_eq!(hit.source_location.file, "src/app/menu.component.html");
    assert_eq!(hit.source_location.line, 2);
    assert_eq!(hit.framework, SourceFlavorDto::Angular);
}

#[test]
fn an_element_repeated_across_pages_is_refused_rather_than_guessed() {
    // The same link exists in index.html and contact.html. Every literal it
    // carries — href, text, class — resolves to both, and there is nothing in
    // the rendered node that says which page it came from.
    let response = run(
        "vanilla-site",
        vec![finding(
            "h3",
            r#"<a href="/demarches" class="cta">En savoir plus</a>"#,
        )],
    );
    assert!(response.mapped.is_empty(), "{:?}", response.mapped);
    let miss = &response.unmappable[0];
    assert_eq!(miss.reason, UnmappableReason::AmbiguousMatch);
    assert!(
        miss.detail.contains("index.html:13") && miss.detail.contains("contact.html:6"),
        "the reason should name the candidates: {}",
        miss.detail
    );
}

#[test]
fn a_runtime_interpolated_attribute_is_reported_as_not_found() {
    // `alt={item.label}` in the template renders to a value that exists
    // nowhere in the sources, so there is no literal to search for.
    let response = run(
        "react-app",
        vec![finding(
            "r3",
            r#"<img src="/media/42.png" alt="Bilan annuel 2024">"#,
        )],
    );
    assert!(response.mapped.is_empty(), "{:?}", response.mapped);
    assert_eq!(
        response.unmappable[0].reason,
        UnmappableReason::NotFoundInSource
    );
}

#[test]
fn a_purely_structural_selector_is_reported_as_having_no_literal() {
    let response = run(
        "vanilla-site",
        vec![SourceMapFindingInput {
            id: "h4".into(),
            selector: Some("body > main > div:nth-child(2) > span".into()),
            html: None,
        }],
    );
    assert_eq!(
        response.unmappable[0].reason,
        UnmappableReason::NoDistinguishingLiteral
    );
}

#[test]
fn a_literal_found_under_a_different_tag_is_not_accepted() {
    // `icon-bars` exists in the Angular template, but on a <span>. Reporting
    // that line for a <div> finding would send the developer to the wrong
    // element, so the hit is rejected instead of downgraded.
    let response = run(
        "angular-app",
        vec![finding("a2", r#"<div class="icon-bars"></div>"#)],
    );
    assert!(response.mapped.is_empty(), "{:?}", response.mapped);
    assert_eq!(
        response.unmappable[0].reason,
        UnmappableReason::ElementNotCorroborated
    );
}

#[test]
fn a_missing_source_root_is_an_invalid_argument_not_an_empty_result() {
    let error = map_findings(
        &fixture("does-not-exist"),
        &[finding("x", r#"<img alt="x">"#)],
    )
    .expect_err("a non-existent root must not look like 'nothing matched'");
    assert_eq!(error.code(), "INVALID_INPUT");
}

#[cfg(unix)]
#[test]
fn a_symlink_out_of_the_source_root_is_skipped_not_followed() {
    // Without this guard a link committed inside the project would let the
    // scan read, and report line numbers from, files the caller never offered.
    let outside = std::env::temp_dir().join(format!("rgaa-src-map-outside-{}", std::process::id()));
    let root = std::env::temp_dir().join(format!("rgaa-src-map-root-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&outside);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&outside).expect("outside dir");
    std::fs::create_dir_all(&root).expect("root dir");
    std::fs::write(
        outside.join("secret.html"),
        "<img alt=\"Marqueur hors racine\" />\n",
    )
    .expect("write outside file");
    std::os::unix::fs::symlink(&outside, root.join("linked")).expect("symlink");

    let response = map_findings(
        root.to_str().expect("utf-8 path"),
        &[finding("s1", r#"<img alt="Marqueur hors racine">"#)],
    )
    .expect("scan");

    assert!(response.mapped.is_empty(), "{:?}", response.mapped);
    assert_eq!(
        response.unmappable[0].reason,
        UnmappableReason::NotFoundInSource
    );
    assert!(
        response
            .skipped_files
            .iter()
            .any(|f| f.file == "linked" && f.reason == "symlink not followed"),
        "the skipped symlink should be reported: {:?}",
        response.skipped_files
    );

    let _ = std::fs::remove_dir_all(&outside);
    let _ = std::fs::remove_dir_all(&root);
}
