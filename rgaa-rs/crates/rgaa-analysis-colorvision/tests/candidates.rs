//! Detection rules from P-02 §4, expressed with the normalised DOM snapshot
//! subset documented in `candidates.rs`.

use rgaa_analysis_colorvision::candidates::{detect_candidates, CandidateKind};

fn link(underlined: bool) -> String {
    format!(
        r##"{{
          "elements": [
            {{
              "tag": "a",
              "selecteur": "p a.more",
              "xpath": "/html/body/p/a",
              "bbox": {{ "x": 10, "y": 20, "width": 80, "height": 18 }},
              "styles": {{ "color": "#0000ee", "text-decoration-line": "{}",
                           "font-weight": "400", "font-size": "16px" }},
              "text": "En savoir plus",
              "parent_text": "Lisez cet article. ",
              "parent_styles": {{ "color": "#000000", "font-weight": "400", "font-size": "16px" }},
              "icon": false
            }}
          ]
        }}"##,
        if underlined { "underline" } else { "none" }
    )
}

#[test]
fn an_ununderlined_low_contrast_link_in_text_is_an_f73_candidate() {
    let candidates = detect_candidates(&link(false)).expect("parse");
    assert_eq!(candidates.len(), 1);
    let candidate = &candidates[0];
    assert_eq!(candidate.kind, CandidateKind::LienSansSoulignement);
    assert_eq!(candidate.test, "3.1.1");
    assert_eq!(candidate.foreground, [0, 0, 238]);
    assert_eq!(candidate.neighbor, [0, 0, 0]);
    assert!(candidate.ratio < 3.0, "ratio was {}", candidate.ratio);
    assert_eq!(candidate.bbox.width, 80.0);
}

#[test]
fn the_same_link_when_underlined_is_not_a_candidate() {
    assert!(detect_candidates(&link(true)).expect("parse").is_empty());
}

#[test]
fn a_link_with_an_icon_or_different_weight_is_not_a_candidate() {
    let icon = link(false).replace("\"icon\": false", "\"icon\": true");
    assert!(detect_candidates(&icon).expect("parse").is_empty());

    let bold = link(false).replacen("\"font-weight\": \"400\"", "\"font-weight\": \"700\"", 1);
    // Only the link's weight changed: it is no longer identical to its parent.
    assert!(detect_candidates(&bold).expect("parse").is_empty());
}

#[test]
fn a_red_border_error_field_is_a_candidate() {
    let dom = r##"{
      "elements": [{
        "tag": "input", "selecteur": "#email", "xpath": "/html/body/form/input[1]",
        "bbox": {"x": 0, "y": 0, "width": 200, "height": 30},
        "styles": {"border-color": "#ff0000", "color": "#000000"}
      }]
    }"##;
    let candidates = detect_candidates(dom).expect("parse");
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].kind, CandidateKind::ChampErreur);
    assert_eq!(candidates[0].test, "3.1.2");
    assert!(candidates[0].error_text.is_empty());
}

#[test]
fn a_chart_whose_series_differ_only_by_fill_is_a_candidate() {
    let dom = r##"{
      "elements": [{
        "tag": "svg", "selecteur": "svg.chart", "xpath": "/html/body/svg",
        "bbox": {"x": 0, "y": 0, "width": 300, "height": 200},
        "styles": {"color": "#000000"},
        "series": [
          {"fill": "#ff0000", "stroke": "none", "pattern": false},
          {"fill": "#00ff00", "stroke": "none", "pattern": false}
        ]
      }]
    }"##;
    let candidates = detect_candidates(dom).expect("parse");
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].kind, CandidateKind::SerieGraphique);
    assert_eq!(candidates[0].series.len(), 2);
    assert_eq!(candidates[0].series[0].color, [255, 0, 0]);
}

#[test]
fn a_coloured_status_dot_without_text_is_a_candidate() {
    let dom = r##"{
      "elements": [{
        "tag": "span", "selecteur": "span.dot", "xpath": "/html/body/span",
        "bbox": {"x": 5, "y": 5, "width": 10, "height": 10},
        "styles": {"background-color": "#00ff00", "color": "#00ff00"},
        "text": ""
      }]
    }"##;
    let candidates = detect_candidates(dom).expect("parse");
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].kind, CandidateKind::Pastille);
}

#[test]
fn a_malformed_dom_is_an_error_not_a_panic() {
    assert!(detect_candidates("{not json").is_err());
}
