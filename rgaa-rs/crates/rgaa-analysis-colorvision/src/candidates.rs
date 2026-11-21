//! Finding the colour-only elements of a page (P-02 §4, `Portee::Mesuree`).
//!
//! `PageCapture::dom_snapshot` is a serialised CDP snapshot. The orchestrator
//! flattens its computed styles into the small, stable subset below before
//! calling the plugin; both `elements` and CDP-shaped `documents[].nodes` are
//! accepted. Unknown fields are ignored, so a fuller snapshot never breaks
//! detection — it simply yields no candidate when the required styles are
//! absent.

use rgaa_analysis::error::{analysis_error, Result};
use rgaa_analysis::types::Rect;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::color::{contrast_ratio, parse_color, Rgb8};

/// The four kinds of colour-only information listed in P-02 §4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum CandidateKind {
    LienSansSoulignement,
    ChampErreur,
    SerieGraphique,
    Pastille,
}

impl CandidateKind {
    /// Value of `mesures.candidat`, as specified in P-02 §5.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LienSansSoulignement => "lien-sans-soulignement",
            Self::ChampErreur => "champ-erreur",
            Self::SerieGraphique => "serie-graphique",
            Self::Pastille => "pastille",
        }
    }

    /// RGAA 4.1.2 test served by this kind of candidate (catalogue `3.1.x`).
    pub fn test(self) -> &'static str {
        match self {
            Self::LienSansSoulignement => "3.1.1",
            Self::ChampErreur => "3.1.2",
            Self::SerieGraphique => "3.1.3",
            Self::Pastille => "3.1.4",
        }
    }
}

/// One series of a chart, distinguished from its neighbours only by colour.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Series {
    pub color: Rgb8,
    pub pattern: bool,
}

/// A candidate element, with the two colours whose simulated loss is measured.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DomCandidate {
    pub kind: CandidateKind,
    pub selecteur: String,
    pub xpath: String,
    pub bbox: Rect,
    /// Colour carrying the information.
    pub foreground: Rgb8,
    /// Colour of the neighbouring text/background it must differ from.
    pub neighbor: Rgb8,
    /// WCAG ratio between the two, when a link in a text block.
    pub ratio: f64,
    pub underlined: bool,
    pub icon: bool,
    /// Text of the error message, when the candidate is an error field.
    pub error_text: String,
    /// Chart series, when the candidate is a chart.
    pub series: Vec<Series>,
    /// RGAA test from the catalogue.
    pub test: &'static str,
}

fn field<'a>(value: &'a Value, names: &[&str]) -> Option<&'a Value> {
    names.iter().find_map(|name| value.get(*name))
}

fn text(value: &Value, names: &[&str]) -> String {
    field(value, names)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn flag(value: &Value, names: &[&str], default: bool) -> bool {
    field(value, names)
        .and_then(Value::as_bool)
        .unwrap_or(default)
}

fn colour(value: &Value, names: &[&str]) -> Option<Rgb8> {
    field(value, names)
        .and_then(Value::as_str)
        .and_then(parse_color)
}

fn rect(value: &Value) -> Rect {
    let bbox = field(value, &["bbox", "bounds"]);
    let number = |name: &str| -> f64 {
        bbox.and_then(|b| b.get(name))
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    };
    Rect {
        x: number("x"),
        y: number("y"),
        width: number("width"),
        height: number("height"),
    }
}

fn styles(value: &Value) -> Value {
    field(value, &["styles", "style", "computed_styles"])
        .cloned()
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()))
}

fn is_red(colour: Rgb8) -> bool {
    let [r, g, b] = colour;
    r >= 120 && r > g.saturating_mul(3) / 2 && r > b.saturating_mul(3) / 2
}

fn is_colourful(colour: Rgb8) -> bool {
    let max = colour.iter().copied().max().unwrap_or(0);
    let min = colour.iter().copied().min().unwrap_or(255);
    max.saturating_sub(min) > 24
}

fn link_candidate(element: &Value) -> Option<DomCandidate> {
    let style = styles(element);
    let foreground = colour(&style, &["color", "colour"])?;
    let parent = field(element, &["parent_styles", "parentStyle"]);
    let neighbor = parent
        .and_then(|p| colour(p, &["color", "colour"]))
        .or_else(|| colour(element, &["surrounding_color", "parent_color"]))?;
    // F73 is about links inside a block of text, not standalone links.
    let in_text = flag(element, &["in_text_block"], false)
        || !text(element, &["parent_text", "surrounding_text"])
            .trim()
            .is_empty();
    if !in_text {
        return None;
    }
    let ratio = contrast_ratio(foreground, neighbor);
    if ratio >= 3.0 {
        return None;
    }
    let decoration = text(
        &style,
        &[
            "text-decoration-line",
            "textDecorationLine",
            "text-decoration",
        ],
    );
    let underlined = decoration.contains("underline") || flag(element, &["underlined"], false);
    if underlined {
        return None;
    }
    if flag(element, &["icon"], false)
        || !text(&style, &["background-image"]).is_empty()
            && !text(&style, &["background-image"]).contains("none")
    {
        return None;
    }
    let parent_weight =
        parent.and_then(|p| text(p, &["font-weight", "fontWeight"]).parse::<u16>().ok());
    let parent_size = parent.and_then(|p| text(p, &["font-size", "fontSize"]).parse::<f64>().ok());
    let weight = text(&style, &["font-weight", "fontWeight"])
        .parse::<u16>()
        .ok();
    let size = text(&style, &["font-size", "fontSize"]).parse::<f64>().ok();
    if parent_weight.is_some() && parent_weight != weight {
        return None;
    }
    if parent_size.is_some() && parent_size != size {
        return None;
    }
    Some(DomCandidate {
        kind: CandidateKind::LienSansSoulignement,
        selecteur: text(element, &["selecteur", "selector"]),
        xpath: text(element, &["xpath"]),
        bbox: rect(element),
        foreground,
        neighbor,
        ratio,
        underlined: false,
        icon: false,
        error_text: String::new(),
        series: Vec::new(),
        test: CandidateKind::LienSansSoulignement.test(),
    })
}

fn error_field_candidate(element: &Value) -> Option<DomCandidate> {
    let style = styles(element);
    let border = colour(
        &style,
        &[
            "border-color",
            "borderColor",
            "border-top-color",
            "border_bottom_color",
        ],
    )
    .or_else(|| colour(element, &["border_color"]))?;
    if !is_red(border) {
        return None;
    }
    Some(DomCandidate {
        kind: CandidateKind::ChampErreur,
        selecteur: text(element, &["selecteur", "selector"]),
        xpath: text(element, &["xpath"]),
        bbox: rect(element),
        foreground: border,
        neighbor: colour(&style, &["background-color", "backgroundColor"])
            .unwrap_or([255, 255, 255]),
        ratio: 0.0,
        underlined: false,
        icon: flag(element, &["icon"], false),
        error_text: text(element, &["error_message", "error_text", "message"]),
        series: Vec::new(),
        test: CandidateKind::ChampErreur.test(),
    })
}

fn series_candidate(element: &Value) -> Option<DomCandidate> {
    let raw = field(element, &["series"])?.as_array()?;
    let mut series = Vec::with_capacity(raw.len());
    for item in raw {
        let color = colour(item, &["fill", "stroke", "color"])?;
        series.push(Series {
            color,
            pattern: flag(item, &["pattern", "motif"], false),
        });
    }
    if series.len() < 2 || series.iter().any(|s| s.pattern) {
        return None;
    }
    if series.iter().all(|s| s.color == series[0].color) {
        return None;
    }
    Some(DomCandidate {
        kind: CandidateKind::SerieGraphique,
        selecteur: text(element, &["selecteur", "selector"]),
        xpath: text(element, &["xpath"]),
        bbox: rect(element),
        foreground: series[0].color,
        neighbor: series[1].color,
        ratio: 0.0,
        underlined: false,
        icon: false,
        error_text: String::new(),
        series,
        test: CandidateKind::SerieGraphique.test(),
    })
}

fn pastille_candidate(element: &Value) -> Option<DomCandidate> {
    if !text(element, &["text"]).trim().is_empty() {
        return None;
    }
    let style = styles(element);
    let foreground = colour(&style, &["color", "colour"])
        .or_else(|| colour(&style, &["background-color", "backgroundColor"]))?;
    if !is_colourful(foreground) {
        return None;
    }
    Some(DomCandidate {
        kind: CandidateKind::Pastille,
        selecteur: text(element, &["selecteur", "selector"]),
        xpath: text(element, &["xpath"]),
        bbox: rect(element),
        foreground,
        neighbor: [255, 255, 255],
        ratio: 0.0,
        underlined: false,
        icon: false,
        error_text: String::new(),
        series: Vec::new(),
        test: CandidateKind::Pastille.test(),
    })
}

/// Detects candidates from the normalised DOM snapshot JSON.
///
/// # Errors
///
/// Returns [`rgaa_analysis::error::RgaaError::Analysis`] when the snapshot is
/// not valid JSON. A valid snapshot without candidates yields an empty vector.
pub fn detect_candidates(dom_json: &str) -> Result<Vec<DomCandidate>> {
    let document: Value = serde_json::from_str(dom_json).map_err(|error| {
        analysis_error("colorvision", format!("DOM snapshot invalide : {error}"))
    })?;
    let mut elements: Vec<&Value> = Vec::new();
    if let Some(list) = document.get("elements").and_then(Value::as_array) {
        elements.extend(list);
    }
    if let Some(documents) = document.get("documents").and_then(Value::as_array) {
        for node_list in documents
            .iter()
            .filter_map(|doc| doc.get("nodes").and_then(Value::as_array))
        {
            elements.extend(node_list);
        }
    }
    let mut candidates = Vec::with_capacity(elements.len());
    for element in elements {
        let tag = text(element, &["tag", "nodeName", "tag_name"]).to_ascii_lowercase();
        let role = text(element, &["role"]).to_ascii_lowercase();
        let candidate = if tag == "a" || role == "link" {
            link_candidate(element)
        } else if matches!(tag.as_str(), "input" | "select" | "textarea") {
            error_field_candidate(element)
        } else if tag == "svg" || tag == "canvas" {
            series_candidate(element)
        } else if tag == "span" || role == "status" {
            pastille_candidate(element)
        } else {
            None
        };
        if let Some(candidate) = candidate {
            candidates.push(candidate);
        }
    }
    Ok(candidates)
}
