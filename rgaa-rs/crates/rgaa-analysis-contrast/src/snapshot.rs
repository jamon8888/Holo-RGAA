//! The shape the plugin expects in `PageCapture::dom_snapshot.json`.
//!
//! P-00 keeps that field as an opaque JSON string: the orchestrator adapts its
//! CDP `DOMSnapshot.captureSnapshot` result into it. The field names below are
//! deliberately aligned with the computed-style collector of PR #456
//! (`rgaa-obscura/src/contrast.rs`), so the same DOM walk feeds both the old
//! Obscura measurement and this plugin: `target`, `color`, `layers`, `issues`,
//! `fontSize`, `fontWeight`, `disabled`.

use crate::mesures::Etat;
use rgaa_analysis::Rect;
use serde::{Deserialize, Serialize};

/// All nodes of one capture. `etat` is set by the orchestrator when it replays
/// a focus or hover state (§3 D); absent means the resting state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Snapshot {
    #[serde(default, alias = "state")]
    pub etat: Option<Etat>,
    #[serde(default)]
    pub texts: Vec<TextNode>,
    #[serde(default)]
    pub components: Vec<ComponentNode>,
}

/// A visible text node of the DOM snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextNode {
    /// CSS selector, called `target` by the #456 collector.
    #[serde(default, alias = "target")]
    pub selecteur: String,
    #[serde(default)]
    pub xpath: String,
    /// Bounding box in CSS pixels; required for step B.
    #[serde(default)]
    pub bbox: Option<Rect>,
    #[serde(default, alias = "color")]
    pub color: Option<String>,
    /// Background colours from the text outwards, up to the first opaque one.
    #[serde(default)]
    pub layers: Vec<String>,
    /// Reasons why a CSS measurement is not reliable (image, opacity, filter…).
    #[serde(default)]
    pub issues: Vec<String>,
    #[serde(default, alias = "fontSize")]
    pub font_size: Option<String>,
    #[serde(default, alias = "fontWeight")]
    pub font_weight: Option<String>,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub logo: bool,
    #[serde(default)]
    pub decorative: bool,
}

/// A non-textual component measured by step C (3.3).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComponentNode {
    #[serde(default, alias = "target")]
    pub selecteur: String,
    #[serde(default)]
    pub xpath: String,
    #[serde(default)]
    pub bbox: Option<Rect>,
    #[serde(default, alias = "color")]
    pub color: Option<String>,
    #[serde(default)]
    pub kind: ComponentKind,
    /// A focus ring explicitly reported as absent yields a 1:1 measurement.
    #[serde(default = "default_true")]
    pub present: bool,
    #[serde(default)]
    pub issues: Vec<String>,
}

fn default_true() -> bool {
    true
}

/// Kinds of non-textual subjects listed in P-01 §3 C.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentKind {
    #[default]
    Border,
    Icon,
    Indicator,
    FocusRing,
}

impl ComponentKind {
    #[must_use]
    pub const fn tests(self) -> &'static [&'static str] {
        match self {
            Self::Border | Self::Indicator | Self::FocusRing => &["3.3.1"],
            Self::Icon => &["3.3.2", "3.3.3"],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_field_names_of_the_456_styles_collector() {
        let snapshot: Snapshot = serde_json::from_str(
            r#"{
                "etat": "focus",
                "texts": [{
                    "target": "body > p", "color": "rgb(119, 119, 119)",
                    "layers": ["rgb(255, 255, 255)"], "issues": [],
                    "fontSize": "24px", "fontWeight": "700", "disabled": false
                }],
                "components": [{"target": "input", "kind": "focus_ring", "present": false}]
            }"#,
        )
        .expect("snapshot");
        assert_eq!(snapshot.etat, Some(Etat::Focus));
        assert_eq!(snapshot.texts[0].selecteur, "body > p");
        assert_eq!(snapshot.texts[0].font_size.as_deref(), Some("24px"));
        assert_eq!(snapshot.components[0].kind, ComponentKind::FocusRing);
        assert!(!snapshot.components[0].present);
    }

    #[test]
    fn missing_optional_fields_default_without_an_error() {
        let snapshot: Snapshot = serde_json::from_str(r#"{}"#).expect("empty snapshot");
        assert!(snapshot.texts.is_empty());
        assert!(snapshot.components.is_empty());
        assert_eq!(snapshot.etat, None);
    }
}
