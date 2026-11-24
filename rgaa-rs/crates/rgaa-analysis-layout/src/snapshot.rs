//! Shape the plugin expects in `PageCapture::dom_snapshot.json` (P-07 §2).
//!
//! P-00 keeps that field opaque: the orchestrator adapts the CDP
//! `DOMSnapshot.captureSnapshot` result (computed styles + DOM rects), the
//! `Accessibility.getFullAXTree` result and the injected `MutationObserver`
//! records into the arrays below. Field names mirror the CDP computed-style
//! collector (`fontSize`, `fontWeight`, `backgroundColor`, `target`), so one DOM
//! walk can feed the contrast and layout plugins.
//!
//! Everything is `#[serde(default)]`: a missing optional field is never an
//! error, and an incomplete capture yields fewer proofs rather than a panic.

use crate::actions::FORBIDDEN_ACTION_WORDS;
use rgaa_analysis::Rect;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One capture: DOM nodes, tables, AX exposure and simulated actions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Snapshot {
    /// Visible DOM nodes, in document order (the "DOM view").
    #[serde(default)]
    pub nodes: Vec<Node>,
    /// Tables, with their cells referring to indices in [`Snapshot::nodes`].
    #[serde(default)]
    pub tables: Vec<Table>,
    /// Accessibility-tree details, referring to indices in [`Snapshot::nodes`].
    #[serde(default)]
    pub ax: Vec<AxInfo>,
    /// Simulated actions (7.1 / 7.5), referring to indices in [`Snapshot::nodes`].
    #[serde(default)]
    pub actions: Vec<Action>,
    /// Declared or P-06-detected language (`fr` / `en`); no detection here.
    #[serde(default)]
    pub lang: Option<String>,
    /// `dir` of the document: `ltr` (default) or `rtl`.
    #[serde(default)]
    pub dir: Option<String>,
}

/// A DOM element with the computed styles needed by the layout analyses.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Node {
    /// CSS selector, called `target` by the computed-style collector.
    #[serde(default, alias = "target")]
    pub selecteur: String,
    #[serde(default)]
    pub xpath: String,
    /// Lowercase tag name (`div`, `h2`, `td`, …).
    #[serde(default)]
    pub tag: String,
    /// Text content of the element (or of its direct text nodes).
    #[serde(default)]
    pub text: String,
    /// Bounding box in CSS pixels, from `includeDOMRects`.
    #[serde(default)]
    pub bbox: Option<Rect>,
    #[serde(default, alias = "fontSize")]
    pub font_size: Option<String>,
    #[serde(default, alias = "fontWeight")]
    pub font_weight: Option<String>,
    #[serde(default, alias = "backgroundColor")]
    pub background_color: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub display: Option<String>,
    #[serde(default)]
    pub position: Option<String>,
    #[serde(default)]
    pub visibility: Option<String>,
    #[serde(default)]
    pub cursor: Option<String>,
    /// CSS `order`, when the element is a flex/grid item.
    #[serde(default)]
    pub order: Option<i32>,
    /// CSS `grid-area`, when the element is a grid item.
    #[serde(default, alias = "gridArea")]
    pub grid_area: Option<String>,
    /// Number of pages on which the same visual zone was observed (9.2).
    #[serde(default, alias = "repeatCount")]
    pub repeat_count: usize,
    /// Explicit WAI-ARIA role (`button`, `status`, `banner`, …).
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default, alias = "ariaLive")]
    pub aria_live: Option<String>,
    #[serde(default, alias = "ariaExpanded")]
    pub aria_expanded: Option<bool>,
    #[serde(default, alias = "ariaPressed")]
    pub aria_pressed: Option<bool>,
    #[serde(default, alias = "ariaChecked")]
    pub aria_checked: Option<bool>,
    #[serde(default, alias = "ariaLevel")]
    pub aria_level: Option<u8>,
    #[serde(default, alias = "ariaLabel")]
    pub aria_label: Option<String>,
    /// `scope` of a `th` cell.
    #[serde(default)]
    pub scope: Option<String>,
    /// Element can receive keyboard focus (7.5).
    #[serde(default)]
    pub focusable: bool,
    /// Event handlers attached to the element (`click`, …).
    #[serde(default)]
    pub handlers: Vec<String>,
    /// Other attributes, e.g. `type = submit` or `data-action = delete`.
    #[serde(default)]
    pub attrs: BTreeMap<String, String>,
}

impl Node {
    /// Whether the element is rendered (CSS visibility only).
    #[must_use]
    pub fn is_visible(&self) -> bool {
        !matches!(self.display.as_deref(), Some("none") | Some("contents"))
            && !matches!(
                self.visibility.as_deref(),
                Some("hidden") | Some("collapse")
            )
    }

    /// Short, non-empty text.
    #[must_use]
    pub fn has_text(&self) -> bool {
        !self.text.trim().is_empty()
    }

    /// Accessible name: `aria-label` first, then the text content.
    #[must_use]
    pub fn label(&self) -> &str {
        self.aria_label.as_deref().unwrap_or(self.text.trim())
    }

    /// Font weight from the computed style, `400` when absent.
    #[must_use]
    pub fn weight(&self) -> u16 {
        self.font_weight
            .as_deref()
            .map(rgaa_analysis::color::parse_weight)
            .unwrap_or(400)
    }

    /// Font size in CSS pixels, when the computed style is parseable.
    #[must_use]
    pub fn size_px(&self) -> Option<f64> {
        self.font_size
            .as_deref()
            .and_then(rgaa_analysis::color::parse_px)
    }

    /// `h1`–`h6` or `role=heading`.
    #[must_use]
    pub fn is_heading(&self) -> bool {
        self.heading_level().is_some()
    }

    /// Heading level from `aria-level` or the `hN` tag.
    #[must_use]
    pub fn heading_level(&self) -> Option<u8> {
        if let Some(level) = self.aria_level {
            return (1..=6).contains(&level).then_some(level);
        }
        let tag = self.tag.trim().to_ascii_lowercase();
        match tag.as_str() {
            "h1" => Some(1),
            "h2" => Some(2),
            "h3" => Some(3),
            "h4" => Some(4),
            "h5" => Some(5),
            "h6" => Some(6),
            _ => None,
        }
    }

    /// AX landmark role (9.2).
    #[must_use]
    pub fn is_landmark(&self) -> bool {
        matches!(
            self.role.as_deref(),
            Some("banner") | Some("navigation") | Some("main") | Some("contentinfo")
        )
    }

    /// Custom widget: `div`/`span` with `cursor:pointer` or a click handler and
    /// no native interactive role (7.1).
    #[must_use]
    pub fn is_custom_widget(&self) -> bool {
        let native = matches!(
            self.tag.as_str(),
            "a" | "button" | "input" | "select" | "textarea" | "summary"
        ) || self.role.is_some();
        let clickable = self.cursor.as_deref() == Some("pointer")
            || self.handlers.iter().any(|handler| handler == "click");
        matches!(self.tag.as_str(), "div" | "span") && clickable && !native
    }

    /// Whether acting on this element is forbidden by the shared F-SEC list.
    #[must_use]
    pub fn is_forbidden_action(&self) -> bool {
        let mut haystack = self.label().to_lowercase();
        for value in self.attrs.values() {
            haystack.push(' ');
            haystack.push_str(&value.to_lowercase());
        }
        if let Some(role) = &self.role {
            haystack.push(' ');
            haystack.push_str(&role.to_lowercase());
        }
        FORBIDDEN_ACTION_WORDS
            .iter()
            .any(|word| haystack.contains(word))
    }
}

/// A table and its cells, in DOM order (row then column).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Table {
    #[serde(default)]
    pub selecteur: String,
    /// `role=presentation` layout table (5.3).
    #[serde(default)]
    pub presentation: bool,
    /// Background of the table body, for the ΔE comparison of 5.6.
    #[serde(default, alias = "bodyBackground")]
    pub body_background: Option<String>,
    #[serde(default)]
    pub rows: Vec<Vec<TableCell>>,
}

/// One cell of a [`Table`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableCell {
    /// Index in [`Snapshot::nodes`].
    #[serde(default)]
    pub node: usize,
    /// `td` or `th`.
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub scope: Option<String>,
}

/// Accessibility-tree details for a DOM node (CDP `getFullAXTree`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AxInfo {
    /// Index in [`Snapshot::nodes`].
    #[serde(default)]
    pub node: usize,
    /// AX role: `heading`, `button`, `banner`, `generic`, …
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub level: Option<u8>,
    /// `expanded`, `pressed`, `checked`, `focused`, …
    #[serde(default)]
    pub states: Vec<String>,
    /// Effective (inherited) `aria-live` value of the message's region.
    #[serde(default)]
    pub live: Option<String>,
}

impl AxInfo {
    /// Genuinely exposed, not a generic AX node (7.1).
    #[must_use]
    pub fn is_generic(&self) -> bool {
        self.role.is_empty()
            || matches!(
                self.role.as_str(),
                "generic" | "none" | "presentation" | "InlineTextBox"
            )
    }

    /// Whether an AX state is present before and after an action.
    #[must_use]
    pub fn has_state(&self, state: &str) -> bool {
        self.states.iter().any(|value| value == state)
    }
}

/// An action replayed by the orchestrator, with its observed before/after state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Action {
    /// Index of the clicked / activated element in [`Snapshot::nodes`].
    #[serde(default)]
    pub target: usize,
    /// Indices of text nodes that appeared after the action (7.5).
    #[serde(default)]
    pub produced: Vec<usize>,
    #[serde(default, alias = "axBefore")]
    pub ax_before: Option<AxInfo>,
    #[serde(default, alias = "axAfter")]
    pub ax_after: Option<AxInfo>,
    /// Fraction of the widget's bounding box whose pixels changed (0.0–1.0),
    /// computed by the orchestrator from the two CDP screenshots.
    #[serde(default, alias = "pixelsChanged")]
    pub pixels_changed: f64,
    /// Index of the element that received focus after the action, if any.
    #[serde(default, alias = "focusAfter")]
    pub focus_after: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_cdp_camel_case_fields_and_defaults_the_rest() {
        let snapshot: Snapshot = serde_json::from_str(
            r##"{
                "nodes":[{"target":"div.big","tag":"div","fontSize":"28px",
                          "fontWeight":"700","backgroundColor":"#dddddd",
                          "ariaExpanded":false,"gridArea":"head"}],
                "actions":[{"target":0,"pixelsChanged":0.1,"focusAfter":0}]
            }"##,
        )
        .expect("snapshot");
        assert_eq!(snapshot.nodes[0].selecteur, "div.big");
        assert_eq!(snapshot.nodes[0].font_size.as_deref(), Some("28px"));
        assert_eq!(snapshot.nodes[0].aria_expanded, Some(false));
        assert_eq!(snapshot.actions[0].pixels_changed, 0.1);
        assert_eq!(snapshot.actions[0].focus_after, Some(0));
        assert!(snapshot.tables.is_empty());
    }

    #[test]
    fn a_submit_button_is_forbidden_but_a_menu_widget_is_not() {
        let submit = Node {
            text: "Soumettre".to_owned(),
            ..Node::default()
        };
        let menu = Node {
            text: "Ouvrir le menu".to_owned(),
            cursor: Some("pointer".to_owned()),
            ..Node::default()
        };
        assert!(submit.is_forbidden_action());
        assert!(!menu.is_forbidden_action());
    }
}
