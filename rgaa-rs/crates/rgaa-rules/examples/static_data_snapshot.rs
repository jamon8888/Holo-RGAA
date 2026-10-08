//! Snapshot of every static lookup the audit path reads, dumped as text so two
//! builds can be diffed byte for byte (#43).
//!
//! The pipeline's own end-to-end run needs a browser and an LLM API key, so this
//! harness stands in for it: it exercises the catalog, the criterion lists, the
//! axe rule-to-criteria map, the gap-fix snippets and the applicability table —
//! the tables #43 moves behind `OnceLock` — and prints their contents in a
//! fixed order. `cargo run -p rgaa-rules --example static_data_snapshot` before
//! and after the change must produce identical output.

use rgaa_core::{na_detection, RgaaCatalog, RgaaCriteria};
use rgaa_rules::{AxeMapper, GapFixRules};
use std::collections::BTreeMap;

fn main() {
    section("criteria.all");
    for c in RgaaCriteria::all() {
        println!(
            "{}|{:?}|{}|{}",
            c.id, c.classification, c.wcag_refs, c.title
        );
    }
    println!("count={}", RgaaCriteria::count());

    section("criteria.deterministe");
    for c in RgaaCriteria::deterministe() {
        println!(
            "{}|{:?}|{}|{}",
            c.id, c.classification, c.wcag_refs, c.title
        );
    }

    section("criteria.ia_assiste");
    for c in RgaaCriteria::ia_assiste() {
        println!(
            "{}|{:?}|{}|{}",
            c.id, c.classification, c.wcag_refs, c.title
        );
    }

    section("criteria.partiellement_automatique");
    for c in RgaaCriteria::partiellement_automatique() {
        println!(
            "{}|{:?}|{}|{}",
            c.id, c.classification, c.wcag_refs, c.title
        );
    }

    section("criteria.classification_for");
    for id in probe_ids() {
        println!("{id}|{:?}", RgaaCriteria::classification_for(&id));
    }

    section("catalog.by_id");
    for id in probe_ids() {
        let entry = RgaaCatalog::by_id(&id).map(|(theme, c)| {
            format!(
                "{theme}|{}|{}|{}|{:?}|{:?}|{:?}|{}/{}",
                c.number,
                c.title,
                c.test_count(),
                c.automatable,
                c.axe_coverage,
                c.axe_rules,
                c.test_accounting.automatable,
                c.test_accounting.total
            )
        });
        println!("{id}|{entry:?}");
    }

    section("axe_mapper.map");
    for (name, json) in axe_fixtures() {
        println!("-- {name}");
        match AxeMapper::map(json) {
            // Insertion order is part of the output: #43 must not reorder it.
            Ok(results) => {
                for (id, r) in &results {
                    println!("{id}|{r:?}");
                }
            }
            Err(e) => println!("error|{e}"),
        }
    }

    section("gap_fix.snippets");
    let snippets: BTreeMap<_, _> = GapFixRules::snippets().iter().collect();
    for (id, js) in snippets {
        println!("{id}|{}|{js}", js.len());
    }

    section("gap_fix.covers_whole_criterion");
    for id in probe_ids() {
        println!("{id}|{}", GapFixRules::covers_whole_criterion(&id));
    }

    section("gap_fix.parse_results");
    let js_results: std::collections::HashMap<String, serde_json::Value> = [
        (
            "1.1",
            serde_json::json!({"pass": false, "details": "2 images without alt", "nodes": 2}),
        ),
        (
            "1.2",
            serde_json::json!({"pass": true, "details": "ok", "nodes": 0}),
        ),
        (
            "8.5",
            serde_json::json!({"pass": true, "details": "ok", "nodes": 0}),
        ),
        (
            "10.11",
            serde_json::json!({"pass": true, "details": "ok", "nodes": 0}),
        ),
        (
            "11.4",
            serde_json::json!({"pass": false, "details": "1 unlabelled field", "nodes": 1}),
        ),
        ("3.2", serde_json::json!({"malformed": true})),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect();
    let parsed: BTreeMap<_, _> = GapFixRules::parse_results(&js_results)
        .into_iter()
        .collect();
    for (id, r) in parsed {
        println!("{id}|{r:?}");
    }

    section("na_detection.detect_na");
    for (name, context) in na_fixtures() {
        println!("-- {name}");
        let table: BTreeMap<_, _> = na_detection::detect_na(&context).into_iter().collect();
        println!("len={}", table.len());
        for (id, applicable) in table {
            println!("{id}|{applicable}");
        }
    }
}

fn section(name: &str) {
    println!("===== {name}");
}

/// Every catalog id, plus ids that must keep resolving to nothing.
fn probe_ids() -> Vec<String> {
    let mut ids: Vec<String> = RgaaCriteria::all()
        .iter()
        .map(|c| c.id.to_string())
        .collect();
    ids.extend(
        ["01.1", "1.01", "99.99", "", "1", "1.1.1", "abc"]
            .iter()
            .map(|s| (*s).to_string()),
    );
    ids
}

fn axe_fixtures() -> Vec<(&'static str, &'static str)> {
    vec![
        ("empty", "[]"),
        ("malformed", "not json"),
        (
            "single",
            r#"[{"id":"color-contrast","impact":"serious","description":"Low contrast","nodes":[{"html":"<p>a</p>"}]}]"#,
        ),
        (
            "mixture",
            r#"[
                {"id":"image-alt","impact":"critical","description":"Images must have alternate text","nodes":[{"html":"<img>"},{"html":"<img src=a>"}]},
                {"id":"color-contrast","impact":"serious","description":"Low contrast","nodes":[{"html":"<p>a</p>"}]},
                {"id":"color-contrast","impact":"critical","description":"Low contrast again","nodes":[{"html":"<p>b</p>"}]},
                {"id":"meta-refresh","impact":"critical","description":"Timed refresh must not exist","nodes":[{"html":"<meta>"}]},
                {"id":"frame-title","impact":"serious","description":"Frames must have an accessible name","nodes":[{"html":"<iframe>"}]},
                {"id":"label","impact":"critical","description":"Form elements must have labels","nodes":[{"html":"<input>"}]},
                {"id":"unknown-rule-xyz","impact":"minor","description":"Unknown","nodes":[]}
            ]"#,
        ),
    ]
}

fn na_fixtures() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        ("empty-object", serde_json::json!({})),
        (
            "nothing-present",
            serde_json::json!({"images":[],"forms":[],"iframes":[],"media":[],"landmarks":[]}),
        ),
        (
            "everything-present",
            serde_json::json!({
                "images":[{"src":"a.png"}],
                "forms":[{"id":"f1"}],
                "iframes":[{"src":"i.html"}],
                "media":[{"media_type":"video"}],
                "landmarks":[{"tag":"main","role":"main"},{"tag":"div","role":"table","label":"Data"}]
            }),
        ),
    ]
}
