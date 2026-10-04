//! The four rule families #166 asks for: `alt`, form `label`, button name and
//! link name.
//!
//! Each rule is written to the same contract: it fires only when the defect is
//! provable from the text of one file. Anything the scanner cannot decide — a
//! JSX spread, a bound attribute, a component child, an `id` computed at
//! runtime — makes the rule stand down. That asymmetry is deliberate. A static
//! linter that reports a false `missing alt` on `<img alt={caption} />` trains
//! its users to skip its output, and the real missing `alt` two files over goes
//! with it.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::config::LintConfig;
use crate::profile::{Profile, Reference};
use crate::scan::{Document, Element};

/// A rule family.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum RuleId {
    /// `<img>` with no `alt` attribute at all.
    ImgAlt,
    /// A form control with no label and no accessible-name attribute.
    FormLabel,
    /// A `<button>` with no accessible name.
    ButtonName,
    /// An `<a href>` with no accessible name.
    LinkName,
}

impl RuleId {
    /// Every rule, in report order.
    pub const ALL: [RuleId; 4] = [
        RuleId::ImgAlt,
        RuleId::FormLabel,
        RuleId::ButtonName,
        RuleId::LinkName,
    ];

    /// The identifier used in `lint-rules.toml` and in findings.
    ///
    /// It matches the axe-core rule id for the same defect wherever one exists,
    /// so a finding from this linter and a finding from the browser-based audit
    /// can be deduplicated by name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ImgAlt => "img-alt",
            Self::FormLabel => "form-label",
            Self::ButtonName => "button-name",
            Self::LinkName => "link-name",
        }
    }

    /// The RGAA 4.1.2 criterion this rule decides against.
    ///
    /// These are catalog ids, not invented ones: a test resolves each against
    /// `RgaaCriteria::all()` so a renumbering cannot leave a rule citing a
    /// criterion that no longer exists. The pairing follows the repository's own
    /// axe → RGAA mapping (`crates/rgaa-core/data/rgaa-4.1.2/mechanisms.toml`),
    /// where `image-alt` → 1.1, `link-name` → 6.1 and both `label` and
    /// `button-name` → 11.1.
    pub const fn rgaa_criterion(self) -> &'static str {
        match self {
            Self::ImgAlt => "1.1",
            Self::FormLabel | Self::ButtonName => "11.1",
            Self::LinkName => "6.1",
        }
    }

    /// Severity when no configuration says otherwise.
    ///
    /// All four are errors: each one leaves a control or an image with no name
    /// at all for assistive technology, which is a failure under every profile
    /// the crate supports, not a matter of taste.
    pub const fn default_severity(self) -> Severity {
        Severity::Error
    }

    /// One sentence on what the rule checks, for tool discovery.
    pub const fn description(self) -> &'static str {
        match self {
            Self::ImgAlt => {
                "<img> must carry an alt attribute; alt=\"\" is the correct value for a \
                 decorative image"
            }
            Self::FormLabel => {
                "every input, select and textarea must have a <label for>, an enclosing \
                 <label>, aria-label, aria-labelledby or title"
            }
            Self::ButtonName => {
                "<button> must have text content or an aria-label, aria-labelledby or title"
            }
            Self::LinkName => {
                "<a href> must have text content, an image with alt text, or an aria-label, \
                 aria-labelledby or title"
            }
        }
    }
}

/// How loudly a rule reports.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Reported and counted against the run.
    Error,
    /// Reported, not counted as a failure.
    Warning,
    /// Not evaluated at all.
    Off,
}

impl Severity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Off => "off",
        }
    }
}

/// What to do about a finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct FixHint {
    /// The action, in the imperative.
    pub action: String,
    /// A concrete attribute or element to add, ready to paste and fill in.
    pub suggestion: String,
}

/// One defect, at one place in one file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Finding {
    pub rule: RuleId,
    pub severity: Severity,
    /// Path as the caller gave it, or the real path when the file was read from
    /// disk. Never a temporary name.
    pub file: String,
    /// 1-based line of the offending opening tag.
    pub line: usize,
    /// 1-based column, counted in characters.
    pub column: usize,
    /// Byte offset of the opening tag, for editors that prefer offsets.
    pub offset: usize,
    /// What is wrong, phrased so it can be read without the rule id.
    pub message: String,
    /// The opening tag as written, whitespace-collapsed and clipped.
    pub snippet: String,
    pub fix_hint: FixHint,
    /// Framework citations, chosen by the active [`Profile`].
    pub references: Vec<Reference>,
}

/// Runs every enabled rule over one scanned document.
pub fn lint_document(
    document: &Document<'_>,
    file: &str,
    config: &LintConfig,
    profile: Profile,
) -> Vec<Finding> {
    let label_targets = collect_label_targets(document);
    let mut findings = Vec::new();

    for (index, element) in document.elements.iter().enumerate() {
        // A JSX component is opaque: `<Button />` may render anything. Only
        // intrinsic (lowercase) tags are ours to judge.
        if !is_intrinsic(element) || is_hidden(element) {
            continue;
        }
        if element.has_spread {
            continue;
        }
        if let Some(finding) = check_img_alt(document, element, config, profile, file) {
            findings.push(finding);
        }
        if let Some(finding) = check_form_label(
            document,
            index,
            element,
            &label_targets,
            config,
            profile,
            file,
        ) {
            findings.push(finding);
        }
        if let Some(finding) = check_button_name(document, index, element, config, profile, file) {
            findings.push(finding);
        }
        if let Some(finding) = check_link_name(document, index, element, config, profile, file) {
            findings.push(finding);
        }
    }
    findings
}

fn is_intrinsic(element: &Element) -> bool {
    element
        .raw_name
        .chars()
        .next()
        .is_some_and(|c| c.is_lowercase() || !c.is_alphabetic())
}

/// Elements removed from the accessibility tree are out of scope: naming
/// something no assistive technology will reach is not a fix, it is noise.
fn is_hidden(element: &Element) -> bool {
    element.literal("aria-hidden") == Some("true")
        || element.attr("hidden").is_some()
        || matches!(element.literal("role"), Some("presentation" | "none"))
}

fn check_img_alt(
    document: &Document<'_>,
    element: &Element,
    config: &LintConfig,
    profile: Profile,
    file: &str,
) -> Option<Finding> {
    let severity = enabled(config, RuleId::ImgAlt)?;
    if element.name != "img" {
        return None;
    }
    // Present-but-empty is correct for a decorative image, so only a wholly
    // absent attribute is a defect. An element named some other way (aria-label)
    // still needs alt in HTML, but flagging it would duplicate what the
    // browser-based audit already reports, so it is left alone.
    if element.attr("alt").is_some() || element.attr_supplies_text("aria-labelledby") {
        return None;
    }
    Some(finding(
        document,
        element,
        RuleId::ImgAlt,
        severity,
        profile,
        file,
        "this <img> has no alt attribute, so assistive technology announces its file name \
         or nothing at all"
            .to_string(),
        FixHint {
            action: "add an alt attribute: a description if the image carries information, \
                     or an empty one if it is decorative"
                .into(),
            suggestion: "alt=\"…\"   (decorative: alt=\"\")".into(),
        },
    ))
}

/// Control types that are not labelled by a `<label>`: hidden ones are not in
/// the accessibility tree, and the button-like ones take their name from
/// `value` or their content instead.
const UNLABELLED_INPUT_TYPES: &[&str] = &["hidden", "submit", "reset", "button"];

fn check_form_label(
    document: &Document<'_>,
    index: usize,
    element: &Element,
    label_targets: &[String],
    config: &LintConfig,
    profile: Profile,
    file: &str,
) -> Option<Finding> {
    let severity = enabled(config, RuleId::FormLabel)?;
    if !matches!(element.name.as_str(), "input" | "select" | "textarea") {
        return None;
    }
    if element.name == "input" {
        match element.attr("type") {
            // An unresolvable type may be `hidden`; claiming a missing label
            // would be a guess.
            Some(value) if value.is_dynamic() => return None,
            Some(crate::scan::AttrValue::Literal(kind))
                if UNLABELLED_INPUT_TYPES.contains(&kind.to_ascii_lowercase().as_str()) =>
            {
                return None
            }
            _ => {}
        }
    }
    if has_name_attribute(element) {
        return None;
    }
    match element.attr("id") {
        Some(value) if value.is_dynamic() => return None,
        Some(crate::scan::AttrValue::Literal(id)) if label_targets.iter().any(|t| t == id) => {
            return None
        }
        _ => {}
    }
    if has_ancestor(document, index, "label") {
        return None;
    }
    let suggestion = match element.literal("id") {
        Some(id) if !id.trim().is_empty() => format!("<label for=\"{id}\">…</label>"),
        _ => "<label for=\"field-id\">…</label>  (and id=\"field-id\" on the control)".to_string(),
    };
    Some(finding(
        document,
        element,
        RuleId::FormLabel,
        severity,
        profile,
        file,
        format!(
            "this <{}> has no label: no <label for>, no enclosing <label>, and no aria-label, \
             aria-labelledby or title",
            element.name
        ),
        FixHint {
            action: "give the control a visible label, or an aria-label when no visible one \
                     is possible"
                .into(),
            suggestion,
        },
    ))
}

fn check_button_name(
    document: &Document<'_>,
    index: usize,
    element: &Element,
    config: &LintConfig,
    profile: Profile,
    file: &str,
) -> Option<Finding> {
    let severity = enabled(config, RuleId::ButtonName)?;
    if element.name != "button" {
        return None;
    }
    if has_name_attribute(element) || content_naming(document, index) != Naming::Empty {
        return None;
    }
    Some(finding(
        document,
        element,
        RuleId::ButtonName,
        severity,
        profile,
        file,
        "this <button> has no accessible name: no text content and no aria-label, \
         aria-labelledby or title"
            .to_string(),
        FixHint {
            action: "add visible text, or an aria-label when the button shows only an icon".into(),
            suggestion: "aria-label=\"…\"   (icon buttons), or visible text between the tags"
                .into(),
        },
    ))
}

fn check_link_name(
    document: &Document<'_>,
    index: usize,
    element: &Element,
    config: &LintConfig,
    profile: Profile,
    file: &str,
) -> Option<Finding> {
    let severity = enabled(config, RuleId::LinkName)?;
    // An `<a>` without href is not a link and exposes no name requirement.
    if element.name != "a" || element.attr("href").is_none() {
        return None;
    }
    if has_name_attribute(element) || content_naming(document, index) != Naming::Empty {
        return None;
    }
    Some(finding(
        document,
        element,
        RuleId::LinkName,
        severity,
        profile,
        file,
        "this link has no accessible name: no text content, no image with alt text, and no \
         aria-label, aria-labelledby or title"
            .to_string(),
        FixHint {
            action: "add link text that says where the link goes, or an aria-label for an \
                     icon-only link"
                .into(),
            suggestion: "aria-label=\"…\"   (icon links), or visible text between the tags".into(),
        },
    ))
}

/// True when the element itself carries an accessible-name attribute.
fn has_name_attribute(element: &Element) -> bool {
    element.attr_supplies_text("aria-label")
        || element.attr_supplies_text("aria-labelledby")
        || element.attr_supplies_text("title")
}

/// Every `for` target declared by a `<label>` in this file.
///
/// Cross-file labels exist and are not detectable here; that is one of the
/// documented limits of static linting, and the reason an unresolvable `id`
/// makes the rule stand down rather than fire.
fn collect_label_targets(document: &Document<'_>) -> Vec<String> {
    document
        .elements
        .iter()
        .filter(|element| element.name == "label")
        .filter_map(|element| element.literal("for").map(str::to_string))
        .collect()
}

fn has_ancestor(document: &Document<'_>, index: usize, name: &str) -> bool {
    let mut current = document.elements[index].parent;
    while let Some(parent) = current {
        if document.elements[parent].name == name {
            return true;
        }
        current = document.elements[parent].parent;
    }
    false
}

/// What the children of an element contribute to its accessible name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Naming {
    /// Text the scanner can read.
    Static,
    /// Something only the runtime knows: an interpolation or a component.
    Dynamic,
    /// Provably nothing.
    Empty,
}

/// Decides what an element's content names it.
///
/// Both [`Naming::Static`] and [`Naming::Dynamic`] suppress a finding; they are
/// kept apart because the distinction is the one that would matter if this ever
/// grew a strict mode.
fn content_naming(document: &Document<'_>, index: usize) -> Naming {
    let element = &document.elements[index];
    let Some((start, end)) = element.children else {
        return Naming::Empty;
    };
    let text = strip_tags(&document.source[start..end]);
    // `{label}` in JSX and `{{ label }}` in Vue are both names decided at
    // runtime.
    if text.contains('{') {
        return Naming::Dynamic;
    }
    if !text
        .replace("&nbsp;", " ")
        .replace('\u{a0}', " ")
        .trim()
        .is_empty()
    {
        return Naming::Static;
    }
    // No text of its own: a nested image or icon may still name it.
    for descendant in document.elements[index + 1..]
        .iter()
        .take_while(|child| child.start < end)
    {
        if !is_intrinsic(descendant) {
            // An opaque component may render text.
            return Naming::Dynamic;
        }
        if is_hidden(descendant) {
            continue;
        }
        if has_name_attribute(descendant) {
            return Naming::Static;
        }
        if descendant.name == "img" && descendant.attr_supplies_text("alt") {
            return Naming::Static;
        }
        if descendant.name == "title" {
            // `<svg><title>Close</title></svg>` names its parent.
            return Naming::Static;
        }
    }
    Naming::Empty
}

/// Removes tag markup so only character data remains.
///
/// Quotes are tracked because an attribute value may contain `>`, and losing
/// that would spill markup into what the caller thinks is text.
fn strip_tags(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut in_tag = false;
    let mut quote: Option<char> = None;
    for ch in source.chars() {
        match (in_tag, quote) {
            (false, _) => {
                if ch == '<' {
                    in_tag = true;
                } else {
                    out.push(ch);
                }
            }
            (true, Some(open)) => {
                if ch == open {
                    quote = None;
                }
            }
            (true, None) => match ch {
                '"' | '\'' => quote = Some(ch),
                '>' => in_tag = false,
                _ => {}
            },
        }
    }
    out
}

/// The configured severity, or `None` when the rule is switched off.
fn enabled(config: &LintConfig, rule: RuleId) -> Option<Severity> {
    match config.severity(rule) {
        Severity::Off => None,
        severity => Some(severity),
    }
}

#[allow(clippy::too_many_arguments)]
fn finding(
    document: &Document<'_>,
    element: &Element,
    rule: RuleId,
    severity: Severity,
    profile: Profile,
    file: &str,
    message: String,
    fix_hint: FixHint,
) -> Finding {
    let (line, column) = document.position(element.start);
    Finding {
        rule,
        severity,
        file: file.to_string(),
        line,
        column,
        offset: element.start,
        message,
        snippet: document.opening_tag(element),
        fix_hint,
        references: profile.references(rule),
    }
}
