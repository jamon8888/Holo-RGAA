//! P-01 plugin oracle — the expected ratios below are hardcoded WebAIM
//! Contrast Checker values (spec §6). They are never recomputed from the code.

use rgaa_analysis::{
    AnalysisPlugin, DomSnapshot, Indice, NoopResourceFetcher, PageCapture, PluginOptions, Portee,
    Preuve, RgbaImage,
};
use rgaa_analysis_contrast::ContrastPlugin;

async fn analyze(snapshot: &str, screenshot: Option<&RgbaImage>, dpr: f32) -> Vec<Preuve> {
    let dom = DomSnapshot::new(snapshot);
    let resources = NoopResourceFetcher;
    let page = PageCapture {
        url: "https://example.test/page",
        dom_snapshot: &dom,
        ax_tree: None,
        screenshot,
        dpr,
        resources: &resources,
        lang_declared: Some("fr"),
    };
    ContrastPlugin
        .analyze(&page, &PluginOptions::default())
        .await
        .expect("contrast proof")
}

fn make_image(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> RgbaImage {
    let buffer = image::RgbaImage::from_fn(width, height, |x, y| image::Rgba(pixel(x, y)));
    RgbaImage {
        width: buffer.width(),
        height: buffer.height(),
        pixels: buffer.into_raw(),
    }
}

fn grey(value: u8) -> [u8; 4] {
    [value, value, value, 255]
}

#[tokio::test]
async fn step_a_767676_on_white_passes_the_webaim_4_54_oracle() {
    let proofs = analyze(
        r##"{"texts":[{
            "selecteur":"p.pass", "xpath":"/html/body/p[1]",
            "bbox":{"x":0,"y":0,"width":100,"height":20},
            "color":"#767676","layers":["#ffffff"],"issues":[],
            "fontSize":"16px","fontWeight":"400"
        }]}"##,
        None,
        1.0,
    )
    .await;
    assert_eq!(proofs.len(), 1);
    let proof = &proofs[0];
    assert_eq!(proof.plugin, "contrast");
    assert_eq!(proof.tests, vec!["3.2.1".to_owned()]);
    assert_eq!(proof.portee, Portee::Mesuree);
    assert!(
        proof.indice.is_none(),
        "a measured proof carries no review hint"
    );
    assert_eq!(proof.mesures["etape"], "A");
    let ratio = proof.mesures["ratio"].as_f64().expect("ratio");
    assert!((ratio - 4.54).abs() < 0.011, "WebAIM 4.54, got {ratio}");
    assert_eq!(proof.mesures["seuil"], 4.5);
    assert_eq!(proof.mesures["couleur_avant"], "#767676");
    assert_eq!(proof.mesures["couleur_fond"], "#ffffff");
    assert_eq!(proof.mesures["taille_px"], 16.0);
    assert!(!proof.empreinte_entree.is_empty());
}

#[tokio::test]
async fn step_a_777_on_white_is_4_48_and_still_renders_no_status() {
    let proofs = analyze(
        r##"{"texts":[{
            "selecteur":"p.fail", "xpath":"/html/body/p[2]",
            "color":"rgb(119, 119, 119)","layers":["rgb(255, 255, 255)"],
            "fontSize":"16px","fontWeight":"400"
        }]}"##,
        None,
        1.0,
    )
    .await;
    let proof = &proofs[0];
    let ratio = proof.mesures["ratio"].as_f64().expect("ratio");
    assert!((ratio - 4.48).abs() < 0.011, "WebAIM 4.48, got {ratio}");
    assert_eq!(proof.portee, Portee::Mesuree);
    assert!(proof.indice.is_none());
    let json = serde_json::to_string(proof).expect("proof json");
    assert!(!json.contains("\"status\""));
    assert!(!json.contains("\"verdict\""));
}

#[tokio::test]
async fn step_a_large_text_uses_the_3_to_1_threshold() {
    let proofs = analyze(
        r##"{"texts":[{
            "selecteur":"h1", "color":"#777777","layers":["#ffffff"],
            "fontSize":"24px","fontWeight":"400"
        }]}"##,
        None,
        1.0,
    )
    .await;
    let proof = &proofs[0];
    assert_eq!(proof.mesures["etape"], "A");
    assert_eq!(proof.mesures["seuil"], 3.0);
    assert_eq!(proof.mesures["texte_grand"], true);
    assert_eq!(proof.tests, vec!["3.2.3".to_owned()]);
    let ratio = proof.mesures["ratio"].as_f64().expect("ratio");
    assert!(ratio >= 3.0, "WebAIM 4.48 ≥ 3, got {ratio}");
}

#[tokio::test]
async fn step_a_composites_a_translucent_background_before_comparing() {
    let proofs = analyze(
        r##"{"texts":[{
            "selecteur":"p", "color":"#ffffff",
            "layers":["rgba(0, 0, 0, 0.5019608)","#ffffff"],
            "fontSize":"16px","fontWeight":"400"
        }]}"##,
        None,
        1.0,
    )
    .await;
    let ratio = proofs[0].mesures["ratio"].as_f64().expect("ratio");
    assert!(
        (ratio - 4.00).abs() < 0.011,
        "WebAIM 4.00 on #7f7f7f, got {ratio}"
    );
}

#[tokio::test]
async fn step_b_white_text_on_a_light_photo_uses_the_worst_percentile() {
    // Light photograph: every background pixel is 240..250, the text is a
    // small connected patch of pure white. p95 is the lightest background,
    // which gives the least favourable ratio for white text.
    let screenshot = make_image(60, 20, |x, y| {
        if (8..20).contains(&x) && (5..15).contains(&y) {
            grey(255)
        } else {
            let shade = 240 + (x % 11) as u8;
            grey(shade)
        }
    });
    let proofs = analyze(
        r##"{"texts":[{
            "selecteur":"span.photo", "bbox":{"x":0,"y":0,"width":60,"height":20},
            "color":"#ffffff","issues":["background-image"],
            "fontSize":"16px","fontWeight":"400"
        }]}"##,
        Some(&screenshot),
        1.0,
    )
    .await;
    let proof = &proofs[0];
    assert_eq!(proof.mesures["etape"], "B");
    assert_eq!(proof.portee, Portee::Estimee);
    assert!(proof.mesures["fond_p5"].is_string());
    assert!(proof.mesures["fond_p95"].is_string());
    let ratio = proof.mesures["ratio"].as_f64().expect("ratio");
    assert!(
        ratio < 3.0,
        "white on light photo must be Suspect, got {ratio}"
    );
    assert_eq!(
        proof.indice,
        Some(Indice::Suspect {
            raison: "pire zone sous le seuil".to_owned()
        })
    );
    assert_eq!(proof.artefacts.len(), 1);
    assert_eq!(proof.artefacts[0].media_type, "image/png");
    assert!(proof.artefacts[0]
        .chemin_bundle
        .starts_with("analysis/contrast/"));
    assert_eq!(proof.artefacts[0].sha256.len(), 64);
}

#[tokio::test]
async fn step_b_gradient_keeps_the_least_favourable_zone() {
    // Black-to-white gradient behind a connected patch of #777 text. The
    // 5th percentile of the background is dark grey, not pure black, so the
    // least favourable zone is the dark end. WebAIM references for the two
    // extremes: #777 on #000 = 4.69, #777 on #fff = 4.48.
    let screenshot = make_image(100, 20, |x, y| {
        if (45..55).contains(&x) && (8..14).contains(&y) {
            grey(119)
        } else {
            grey((x * 255 / 99) as u8)
        }
    });
    let proofs = analyze(
        r##"{"texts":[{
            "selecteur":"p.gradient", "bbox":{"x":0,"y":0,"width":100,"height":20},
            "color":"#777777","issues":["background-image"],
            "fontSize":"16px","fontWeight":"400"
        }]}"##,
        Some(&screenshot),
        1.0,
    )
    .await;
    let proof = &proofs[0];
    let ratio = proof.mesures["ratio"].as_f64().expect("ratio");
    let p5 = proof.mesures["fond_p5"].as_str().expect("p5");
    let p95 = proof.mesures["fond_p95"].as_str().expect("p95");
    let dark_end = rgaa_analysis_contrast::parse_color(p5).expect("p5 colour");
    let light_end = rgaa_analysis_contrast::parse_color(p95).expect("p95 colour");
    assert!(dark_end.r < 40, "p5 must sit at the dark end, got {p5}");
    assert!(
        light_end.r > 239,
        "p95 must sit at the light end, got {p95}"
    );
    // The retained ratio is the least favourable of the two extremes.
    let text = rgaa_analysis_contrast::parse_color("#777777").expect("text");
    let worst = rgaa_analysis_contrast::contrast_ratio(text, dark_end)
        .min(rgaa_analysis_contrast::contrast_ratio(text, light_end));
    assert!((ratio - worst).abs() < 1e-9);
    // The worst of the two zones is below the white-end WebAIM ratio.
    assert!(
        ratio < 4.48,
        "expected the least favourable zone, got {ratio}"
    );
}

#[tokio::test]
async fn step_c_input_border_ccc_on_white_is_1_61() {
    let screenshot = make_image(60, 30, |x, y| {
        if (10..30).contains(&x) && (10..12).contains(&y) {
            grey(204)
        } else {
            grey(255)
        }
    });
    let proofs = analyze(
        r##"{"components":[{
            "selecteur":"input", "xpath":"/html/body/input[1]",
            "bbox":{"x":10,"y":10,"width":20,"height":2},
            "color":"#cccccc","kind":"border"
        }]}"##,
        Some(&screenshot),
        1.0,
    )
    .await;
    let proof = &proofs[0];
    assert_eq!(proof.mesures["etape"], "C");
    assert_eq!(proof.portee, Portee::Estimee);
    assert_eq!(proof.tests, vec!["3.3.1".to_owned()]);
    assert_eq!(proof.mesures["seuil"], 3.0);
    let ratio = proof.mesures["ratio"].as_f64().expect("ratio");
    assert!((ratio - 1.61).abs() < 0.011, "WebAIM 1.61, got {ratio}");
    assert!(matches!(&proof.indice, Some(Indice::Suspect { .. })));
}

#[tokio::test]
async fn step_d_missing_focus_ring_in_focus_state_is_a_1_to_1_measurement() {
    let proofs = analyze(
        r##"{"etat":"focus","components":[{
            "selecteur":"button", "kind":"focus_ring", "present":false,
            "color":"#cccccc"
        }]}"##,
        None,
        1.0,
    )
    .await;
    let proof = &proofs[0];
    assert_eq!(proof.mesures["etat"], "focus");
    assert_eq!(proof.mesures["etape"], "C");
    assert_eq!(proof.mesures["ratio"], 1.0);
    assert_eq!(proof.mesures["raison"], "contour de focus absent");
    assert_eq!(
        proof.indice,
        Some(Indice::Suspect {
            raison: "contour de focus absent".to_owned()
        })
    );
}

#[tokio::test]
async fn disabled_text_gets_a_neutral_index_and_no_threshold() {
    let proofs = analyze(
        r##"{"texts":[{
            "selecteur":"button", "color":"#dddddd","layers":["#ffffff"],
            "disabled":true,"fontSize":"16px","fontWeight":"400"
        }]}"##,
        None,
        1.0,
    )
    .await;
    let proof = &proofs[0];
    assert_eq!(proof.indice, Some(Indice::Neutre));
    assert_eq!(proof.mesures["raison"], "désactivé");
    assert!(proof.mesures["seuil"].is_null());
}

#[tokio::test]
async fn proofs_are_deterministic_and_respect_dom_order() {
    let snapshot = r##"{
        "texts":[
            {"selecteur":"p.first","color":"#767676","layers":["#fff"],"fontSize":"16px"},
            {"selecteur":"p.second","color":"#777777","layers":["#fff"],"fontSize":"16px"}
        ],
        "components":[
            {"selecteur":"input","color":"#ccc","kind":"border","bbox":{"x":0,"y":0,"width":2,"height":2}}
        ]
    }"##;
    let first = analyze(snapshot, None, 1.0).await;
    let second = analyze(snapshot, None, 1.0).await;
    assert_eq!(
        serde_json::to_string(&first).expect("json"),
        serde_json::to_string(&second).expect("json")
    );
    let subjects: Vec<String> = first
        .iter()
        .map(|proof| match &proof.sujet {
            rgaa_analysis::Subject::Element { selecteur, .. } => selecteur.clone(),
            _ => "other".to_owned(),
        })
        .collect();
    assert_eq!(subjects, vec!["p.first", "p.second", "input"]);
}
