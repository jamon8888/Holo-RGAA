//! P-07 oracle — expected values come from the spec (§5 test table), the RGAA
//! criterion wording and definitional colour facts, never from the code under
//! test. Every case below must be red before the analyses are implemented.

use rgaa_analysis::{
    AnalysisPlugin, AxNode, AxTree, DomSnapshot, Indice, NoopResourceFetcher, PageCapture,
    PluginOptions, Portee, Preuve,
};
use rgaa_analysis_layout::{LayoutPlugin, LAYOUT_TESTS};

async fn analyze(snapshot: &str, lang: Option<&str>, max_actions: usize) -> Vec<Preuve> {
    let dom = DomSnapshot::new(snapshot);
    let resources = NoopResourceFetcher;
    let ax_tree = AxTree {
        nodes: vec![AxNode {
            role: "heading".to_owned(),
            name: "Contact".to_owned(),
        }],
    };
    let page = PageCapture {
        url: "https://example.test/page",
        dom_snapshot: &dom,
        ax_tree: Some(&ax_tree),
        screenshot: None,
        dpr: 1.0,
        resources: &resources,
        lang_declared: lang,
    };
    LayoutPlugin::with_max_actions(max_actions)
        .analyze(&page, &PluginOptions::default())
        .await
        .expect("layout proofs")
}

fn proof<'a>(proofs: &'a [Preuve], analyse: &str) -> &'a Preuve {
    proofs
        .iter()
        .find(|proof| proof.mesures["analyse"] == analyse)
        .unwrap_or_else(|| panic!("no {analyse} proof in {proofs:?}"))
}

fn suspect(proof: &Preuve) -> bool {
    matches!(proof.indice, Some(Indice::Suspect { .. }))
}

#[tokio::test]
async fn case_1_a_bold_28px_div_is_a_visual_heading_not_exposed() {
    let proofs = analyze(
        r##"{
            "nodes":[
              {"selecteur":"p.intro","tag":"p","text":"Une phrase de corps tout a fait ordinaire pour la mediane.",
               "fontSize":"16px","fontWeight":"400","bbox":{"x":0,"y":40,"width":400,"height":20}},
              {"selecteur":"div.big","tag":"div","text":"Nos offres","fontSize":"28px","fontWeight":"700",
               "bbox":{"x":0,"y":0,"width":200,"height":36}}
            ]
        }"##,
        Some("fr"),
        20,
    )
    .await;
    let headings = proof(&proofs, "headings");
    assert_eq!(headings.tests, vec!["9.1.1".to_owned(), "9.1.3".to_owned()]);
    assert_eq!(headings.portee, Portee::Mesuree);
    let visual = &headings.mesures["titres_visuels"];
    assert_eq!(visual[0]["texte"], "Nos offres");
    assert_eq!(visual[0]["taille_px"], 28.0);
    assert_eq!(visual[0]["graisse"], 700);
    assert_eq!(visual[0]["expose"], false);
    assert!(suspect(headings), "an unexposed visual heading is Suspect");
}

#[tokio::test]
async fn case_2_an_h2_smaller_than_an_h3_is_an_inversion() {
    let proofs = analyze(
        r##"{
            "nodes":[
              {"selecteur":"h3","tag":"h3","text":"Nos offres","fontSize":"24px","fontWeight":"700",
               "bbox":{"x":0,"y":0,"width":300,"height":30}},
              {"selecteur":"h2","tag":"h2","text":"Contact","fontSize":"14px","fontWeight":"700",
               "bbox":{"x":0,"y":60,"width":300,"height":18}},
              {"selecteur":"p.a","tag":"p","text":"Premier paragraphe de corps de texte.",
               "fontSize":"16px","fontWeight":"400","bbox":{"x":0,"y":120,"width":400,"height":20}},
              {"selecteur":"p.b","tag":"p","text":"Deuxieme paragraphe de corps de texte.",
               "fontSize":"16px","fontWeight":"400","bbox":{"x":0,"y":150,"width":400,"height":20}}
            ]
        }"##,
        Some("fr"),
        20,
    )
    .await;
    let headings = proof(&proofs, "headings");
    assert_eq!(headings.mesures["inversions"], 1);
    assert_eq!(headings.mesures["sauts_niveau"], serde_json::json!([]));
    assert!(suspect(headings));
}

#[tokio::test]
async fn case_2b_a_skipped_heading_level_is_reported() {
    let proofs = analyze(
        r##"{
            "nodes":[
              {"selecteur":"h1","tag":"h1","text":"Titre","fontSize":"32px","bbox":{"x":0,"y":0,"width":300,"height":40}},
              {"selecteur":"h3","tag":"h3","text":"Sous titre","fontSize":"24px","bbox":{"x":0,"y":60,"width":300,"height":30}},
              {"selecteur":"p","tag":"p","text":"Un paragraphe de corps de texte ici.",
               "fontSize":"16px","bbox":{"x":0,"y":120,"width":400,"height":20}}
            ]
        }"##,
        Some("fr"),
        20,
    )
    .await;
    assert_eq!(
        proof(&proofs, "headings").mesures["sauts_niveau"],
        serde_json::json!(["h1→h3"])
    );
}

#[tokio::test]
async fn case_3_order_minus_one_making_the_footer_appear_first_gives_tau_below_0_8() {
    let proofs = analyze(
        r##"{
            "nodes":[
              {"selecteur":"header","tag":"header","text":"En tete","bbox":{"x":0,"y":100,"width":400,"height":30}},
              {"selecteur":"s1","tag":"section","text":"Section un","bbox":{"x":0,"y":200,"width":400,"height":30}},
              {"selecteur":"s2","tag":"section","text":"Section deux","bbox":{"x":0,"y":300,"width":400,"height":30}},
              {"selecteur":"s3","tag":"section","text":"Section trois","bbox":{"x":0,"y":400,"width":400,"height":30}},
              {"selecteur":"main","tag":"main","text":"Contenu","bbox":{"x":0,"y":500,"width":400,"height":30}},
              {"selecteur":"footer","tag":"footer","text":"Pied de page","order":-1,
               "bbox":{"x":0,"y":0,"width":400,"height":30}}
            ]
        }"##,
        Some("fr"),
        20,
    )
    .await;
    let order = proof(&proofs, "reading_order");
    assert_eq!(order.tests, vec!["10.3.1".to_owned()]);
    let tau = order.mesures["tau"].as_f64().expect("tau");
    assert!(tau < 0.8, "footer moved above everything: tau {tau}");
    assert!(!order.mesures["sauts_positionnes"]
        .as_array()
        .expect("jumps")
        .is_empty());
    assert!(suspect(order));
}

#[tokio::test]
async fn case_4_clickable_div_opening_a_menu_without_aria_expanded_is_suspect() {
    let proofs = analyze(
        r##"{
            "nodes":[
              {"selecteur":"div.widget","tag":"div","text":"Menu","cursor":"pointer","handlers":["click"],
               "bbox":{"x":0,"y":0,"width":80,"height":30}},
              {"selecteur":"div.menu","tag":"div","text":"Options","bbox":{"x":0,"y":40,"width":80,"height":30}}
            ],
            "actions":[{
              "target":0,"produced":[1],"pixelsChanged":0.10,
              "axBefore":{"node":0,"role":"button","name":"Menu","states":[]},
              "axAfter":{"node":0,"role":"button","name":"Menu","states":[]}
            }]
        }"##,
        Some("fr"),
        20,
    )
    .await;
    let widget = proof(&proofs, "widget");
    assert_eq!(
        widget.tests,
        vec!["7.1.1".to_owned(), "7.1.2".to_owned(), "7.1.3".to_owned()]
    );
    let observes = widget.mesures["observes"].as_array().expect("observes");
    assert_eq!(observes.len(), 1);
    assert_eq!(observes[0]["pixels_changed"], 0.1);
    assert!(suspect(widget));
}

#[tokio::test]
async fn case_4b_a_widget_whose_ax_state_follows_the_render_is_not_suspect() {
    let proofs = analyze(
        r##"{
            "nodes":[
              {"selecteur":"div.widget","tag":"div","text":"Menu","cursor":"pointer","handlers":["click"],
               "bbox":{"x":0,"y":0,"width":80,"height":30}}
            ],
            "actions":[{
              "target":0,"pixelsChanged":0.10,
              "axBefore":{"node":0,"role":"button","name":"Menu","states":[]},
              "axAfter":{"node":0,"role":"button","name":"Menu","states":["expanded"]}
            }]
        }"##,
        Some("fr"),
        20,
    )
    .await;
    assert!(!suspect(proof(&proofs, "widget")));
}

#[tokio::test]
async fn case_5_a_status_message_without_aria_live_is_suspect() {
    let proofs = analyze(
        r##"{
            "nodes":[
              {"selecteur":"button.add","tag":"button","text":"Ajouter","bbox":{"x":0,"y":0,"width":80,"height":30}},
              {"selecteur":"div.toast","tag":"div","text":"Ajoute au panier","bbox":{"x":0,"y":40,"width":200,"height":20}}
            ],
            "actions":[{"target":0,"produced":[1],"pixelsChanged":0.05}]
        }"##,
        Some("fr"),
        20,
    )
    .await;
    let status = proof(&proofs, "status");
    assert_eq!(status.tests, vec!["7.5.1".to_owned(), "7.5.2".to_owned()]);
    let messages = status.mesures["messages"].as_array().expect("messages");
    assert_eq!(messages[0]["texte"], "Ajoute au panier");
    assert!(messages[0]["region_live"].is_null());
    assert!(suspect(status));
}

#[tokio::test]
async fn case_5b_a_message_inside_a_live_region_is_not_suspect() {
    let proofs = analyze(
        r##"{
            "nodes":[
              {"selecteur":"button.add","tag":"button","text":"Ajouter","bbox":{"x":0,"y":0,"width":80,"height":30}},
              {"selecteur":"div.toast","tag":"div","text":"Ajoute au panier","ariaLive":"polite",
               "bbox":{"x":0,"y":40,"width":200,"height":20}}
            ],
            "actions":[{"target":0,"produced":[1],"pixelsChanged":0.05}]
        }"##,
        Some("fr"),
        20,
    )
    .await;
    assert!(!suspect(proof(&proofs, "status")));
}

#[tokio::test]
async fn case_6_a_sentence_referring_to_a_green_button_on_the_right_is_a_candidate() {
    let proofs = analyze(
        r##"{
            "nodes":[
              {"selecteur":"p.help","tag":"p","text":"Cliquez sur le bouton vert a droite.",
               "bbox":{"x":0,"y":0,"width":400,"height":20}}
            ]
        }"##,
        Some("fr"),
        20,
    )
    .await;
    let sensory = proof(&proofs, "sensory");
    assert_eq!(
        sensory.tests,
        vec!["10.9.1".to_owned(), "10.10.1".to_owned()]
    );
    let candidates = sensory.mesures["candidats"].as_array().expect("candidats");
    assert_eq!(
        candidates[0]["texte"],
        "Cliquez sur le bouton vert a droite."
    );
    let categories = candidates
        .iter()
        .map(|candidate| candidate["categorie"].as_str().expect("category"))
        .collect::<Vec<_>>();
    assert!(categories.contains(&"couleur"), "{categories:?}");
    assert!(categories.contains(&"position"), "{categories:?}");
    assert!(
        sensory.mesures["avis_modele"].is_null(),
        "extension point only"
    );
    assert!(suspect(sensory));
}

#[tokio::test]
async fn case_7_a_bold_td_on_a_grey_background_is_a_header_candidate() {
    let proofs = analyze(
        r##"{
            "nodes":[
              {"selecteur":"table thead td","tag":"td","text":"Janvier","fontWeight":"700",
               "backgroundColor":"#dddddd","bbox":{"x":0,"y":0,"width":80,"height":20}}
            ],
            "tables":[{
              "selecteur":"table.calendar","bodyBackground":"#ffffff",
              "rows":[[{"node":0,"tag":"td"}]]
            }]
        }"##,
        Some("fr"),
        20,
    )
    .await;
    let headers = proof(&proofs, "table_headers");
    assert_eq!(headers.tests, vec!["5.6.1".to_owned(), "5.6.2".to_owned()]);
    assert_eq!(
        headers.mesures["candidats"]
            .as_array()
            .expect("candidats")
            .len(),
        1
    );
    assert!(suspect(headers));
}

#[tokio::test]
async fn a_black_cell_on_white_is_detected_by_the_delta_e_of_100() {
    // CIEDE2000 is 100 between L*=0 (black) and L*=100 (white): definitional,
    // far above the spec's ΔE > 10 threshold.
    let proofs = analyze(
        r##"{
            "nodes":[
              {"selecteur":"table td.cell","tag":"td","text":"Total","fontWeight":"400",
               "backgroundColor":"#000000","bbox":{"x":0,"y":0,"width":80,"height":20}}
            ],
            "tables":[{
              "selecteur":"table.data","bodyBackground":"#ffffff",
              "rows":[[{"node":0,"tag":"td"}]]
            }]
        }"##,
        Some("fr"),
        20,
    )
    .await;
    let headers = proof(&proofs, "table_headers");
    let delta = headers.mesures["candidats"][0]["delta_e"]
        .as_f64()
        .expect("delta");
    assert!(
        (delta - 100.0).abs() < 1e-6,
        "CIEDE2000 black↔white is 100, got {delta}"
    );
    assert!(suspect(headers));
}

#[tokio::test]
async fn a_repeated_visual_zone_without_a_landmark_is_suspect() {
    let proofs = analyze(
        r##"{
            "nodes":[
              {"selecteur":"header","tag":"header","text":"En tete","role":"banner","repeatCount":3,
               "bbox":{"x":0,"y":0,"width":400,"height":40}},
              {"selecteur":"nav","tag":"nav","text":"Navigation","role":"navigation","repeatCount":2,
               "bbox":{"x":0,"y":40,"width":400,"height":30}},
              {"selecteur":"footer","tag":"footer","text":"Pied de page","repeatCount":3,
               "bbox":{"x":0,"y":600,"width":400,"height":40}},
              {"selecteur":"main","tag":"main","text":"Contenu","role":"main","repeatCount":1,
               "bbox":{"x":0,"y":80,"width":400,"height":400}}
            ]
        }"##,
        Some("fr"),
        20,
    )
    .await;
    let landmarks = proof(&proofs, "landmarks");
    assert_eq!(landmarks.tests, vec!["9.2.1".to_owned()]);
    assert_eq!(
        landmarks.mesures["sans_repere"],
        serde_json::json!(["footer"])
    );
    assert!(suspect(landmarks));
}

#[tokio::test]
async fn a_presentation_table_is_linearised_against_visual_order() {
    let proofs = analyze(
        r##"{
            "nodes":[
              {"selecteur":"td.a","tag":"td","text":"A","bbox":{"x":200,"y":0,"width":50,"height":20}},
              {"selecteur":"td.b","tag":"td","text":"B","bbox":{"x":100,"y":0,"width":50,"height":20}},
              {"selecteur":"td.c","tag":"td","text":"C","bbox":{"x":0,"y":0,"width":50,"height":20}}
            ],
            "tables":[{
              "selecteur":"table.layout","presentation":true,
              "rows":[[{"node":0,"tag":"td"},{"node":1,"tag":"td"},{"node":2,"tag":"td"}]]
            }]
        }"##,
        Some("fr"),
        20,
    )
    .await;
    let linear = proof(&proofs, "linearisation");
    assert_eq!(linear.tests, vec!["5.3.1".to_owned()]);
    let tau = linear.mesures["tau"].as_f64().expect("tau");
    assert!(
        (tau + 1.0).abs() < 1e-9,
        "full reversal gives tau -1, got {tau}"
    );
    assert_eq!(linear.mesures["inversions"], 3);
    assert!(suspect(linear));
}

#[tokio::test]
async fn forbidden_actions_are_never_simulated() {
    let proofs = analyze(
        r##"{
            "nodes":[
              {"selecteur":"button.delete","tag":"button","text":"Supprimer"},
              {"selecteur":"div.pay","tag":"div","text":"Payer","cursor":"pointer","handlers":["click"]},
              {"selecteur":"button.ok","tag":"button","text":"Valider"}
            ],
            "actions":[
              {"target":0,"produced":[],"pixelsChanged":0.9},
              {"target":1,"produced":[],"pixelsChanged":0.9},
              {"target":2,"produced":[0],"pixelsChanged":0.0}
            ]
        }"##,
        Some("fr"),
        20,
    )
    .await;
    assert!(!proofs
        .iter()
        .any(|proof| proof.mesures["analyse"] == "widget"));
    let status = proof(&proofs, "status");
    assert_eq!(status.mesures["actions_ignorees"], 2);
    assert_eq!(
        status.mesures["messages"]
            .as_array()
            .expect("messages")
            .len(),
        1
    );
}

#[tokio::test]
async fn simulated_actions_are_bounded_to_the_configured_maximum() {
    let mut nodes = String::new();
    let mut actions = String::new();
    for index in 0..25 {
        if index > 0 {
            nodes.push(',');
            actions.push(',');
        }
        nodes.push_str(&format!(
            r##"{{"selecteur":"div.w{index}","tag":"div","text":"Widget {index}","cursor":"pointer","handlers":["click"]}}"##
        ));
        actions.push_str(&format!(
            r##"{{"target":{index},"axBefore":{{"node":{index},"role":"button","name":"Widget","states":[]}},"axAfter":{{"node":{index},"role":"button","name":"Widget","states":[]}},"pixelsChanged":0.1}}"##
        ));
    }
    let snapshot = format!(r##"{{"nodes":[{nodes}],"actions":[{actions}]}}"##);
    let proofs = analyze(&snapshot, Some("fr"), 3).await;
    let widget = proof(&proofs, "widget");
    assert_eq!(
        widget.mesures["observes"]
            .as_array()
            .expect("observes")
            .len(),
        3
    );
}

#[tokio::test]
async fn proofs_never_carry_a_status_and_are_deterministic() {
    let snapshot = r##"{
        "nodes":[
          {"selecteur":"div.big","tag":"div","text":"Nos offres","fontSize":"28px","fontWeight":"700"},
          {"selecteur":"p","tag":"p","text":"Un paragraphe de corps avec seize pixels.",
           "fontSize":"16px","fontWeight":"400"}
        ]
    }"##;
    let first = analyze(snapshot, Some("fr"), 20).await;
    let second = analyze(snapshot, Some("fr"), 20).await;
    assert_eq!(
        serde_json::to_string(&first).expect("json"),
        serde_json::to_string(&second).expect("json")
    );
    let json = serde_json::to_string(&first).expect("json");
    assert!(!json.contains("\"status\""), "{json}");
    assert!(!json.contains("\"verdict\""), "{json}");
    assert!(!LAYOUT_TESTS.is_empty());
}
