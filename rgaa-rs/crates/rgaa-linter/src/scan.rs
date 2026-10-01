//! A markup scanner for the three shapes this linter reads: HTML, JSX/TSX and
//! Vue single-file components.
//!
//! This is deliberately **not** a parser. A real TSX or Vue parser would pull a
//! JavaScript toolchain into a crate whose whole value is being fast enough to
//! run on every keystroke, and nothing the four supported rules ask requires an
//! AST: they all decide on one opening tag plus the text between it and its
//! close. So the scanner walks the source once and emits opening tags with
//! their attributes and, where it can pair them, the byte range of their
//! children.
//!
//! What that costs is stated rather than hidden. The scanner cannot see through
//! a component boundary (`<Button />` is not `<button>`), it cannot evaluate an
//! expression, and it treats anything dynamic as unknown. Every rule in
//! [`crate::rules`] is written to stay silent on unknowns, because a linter that
//! cries wolf on `alt={caption}` gets switched off and then finds nothing at all.

/// The source dialect, which decides attribute aliasing and which tags are void.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// `.html`, `.htm`.
    Html,
    /// `.jsx`, `.tsx` — and `.js`/`.ts`, which may embed JSX.
    Jsx,
    /// `.vue` single-file components.
    Vue,
}

impl Language {
    /// Infers the dialect from a path's extension.
    ///
    /// Returns `None` for anything else so the caller can refuse the file loudly
    /// instead of silently linting a `.png` as HTML and reporting nothing.
    pub fn from_path(path: &str) -> Option<Self> {
        let extension = path.rsplit('.').next()?.to_ascii_lowercase();
        match extension.as_str() {
            "html" | "htm" => Some(Self::Html),
            "jsx" | "tsx" | "js" | "ts" | "mjs" => Some(Self::Jsx),
            "vue" => Some(Self::Vue),
            _ => None,
        }
    }

    /// Extensions [`Language::from_path`] accepts, for error messages.
    pub const SUPPORTED_EXTENSIONS: &'static str = "html, htm, jsx, tsx, js, ts, mjs, vue";
}

/// What an attribute was given, which is the difference between a defect and an
/// unknown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttrValue {
    /// A bare attribute with no `=`, e.g. `disabled`.
    Bare,
    /// A quoted or unquoted literal the scanner can read.
    Literal(String),
    /// A value the scanner cannot evaluate: JSX `{...}`, a Vue `:bound` or
    /// `v-bind:` attribute. Its content is unknown at lint time.
    Expression,
}

impl AttrValue {
    /// True when the value is known to be a non-blank literal.
    ///
    /// Blank matters: `aria-label="  "` names nothing, so treating "the attribute
    /// is present" as "the element is named" would hide the very defect the rule
    /// exists for.
    pub fn is_non_blank_literal(&self) -> bool {
        matches!(self, Self::Literal(text) if !text.trim().is_empty())
    }

    /// True when the value cannot be decided statically.
    pub fn is_dynamic(&self) -> bool {
        matches!(self, Self::Expression)
    }
}

/// One attribute of an opening tag, with its name already normalized.
#[derive(Debug, Clone)]
pub struct Attribute {
    /// Lowercased and de-aliased: `className` and `:class` both arrive as `class`.
    pub name: String,
    /// The value as the scanner could read it.
    pub value: AttrValue,
}

/// One opening tag the scanner recognised.
#[derive(Debug, Clone)]
pub struct Element {
    /// Lowercased tag name. A capitalised JSX component keeps its own name
    /// lowercased too, which is harmless: no rule matches component names.
    pub name: String,
    /// The original tag name as written, used to tell `<Button>` from `<button>`.
    pub raw_name: String,
    pub attributes: Vec<Attribute>,
    /// Byte offset of the `<`.
    pub start: usize,
    /// Byte offset just past the `>` of the opening tag.
    pub open_end: usize,
    /// Byte range of the children, when a matching close tag was found.
    pub children: Option<(usize, usize)>,
    /// True when the tag carries a JSX spread (`{...props}`), which may supply
    /// any attribute. Rules must not claim an attribute is missing on such a tag.
    pub has_spread: bool,
    /// Index into the element list of the nearest enclosing element, if any.
    pub parent: Option<usize>,
}

impl Element {
    /// Looks up a normalized attribute name.
    pub fn attr(&self, name: &str) -> Option<&AttrValue> {
        self.attributes
            .iter()
            .find(|a| a.name == name)
            .map(|a| &a.value)
    }

    /// True when the attribute is present with a non-blank literal, or dynamic.
    ///
    /// Dynamic counts as satisfied on purpose: the author wrote something there
    /// and the linter cannot read it, so the only honest verdict is "not a defect
    /// I can prove".
    pub fn attr_supplies_text(&self, name: &str) -> bool {
        self.attr(name)
            .is_some_and(|v| v.is_non_blank_literal() || v.is_dynamic())
    }

    /// The literal value of an attribute, if it is a literal.
    pub fn literal(&self, name: &str) -> Option<&str> {
        match self.attr(name) {
            Some(AttrValue::Literal(text)) => Some(text.as_str()),
            _ => None,
        }
    }
}

/// A scanned document: its elements plus the line index needed to place them.
#[derive(Debug)]
pub struct Document<'a> {
    pub source: &'a str,
    pub language: Language,
    pub elements: Vec<Element>,
    line_starts: Vec<usize>,
}

impl<'a> Document<'a> {
    /// Translates a byte offset into a 1-based line and column.
    ///
    /// The column counts characters, not bytes, so an accented word earlier on
    /// the line does not push the reported column past what an editor shows.
    pub fn position(&self, offset: usize) -> (usize, usize) {
        let line_index = match self.line_starts.binary_search(&offset) {
            Ok(index) => index,
            Err(index) => index - 1,
        };
        let line_start = self.line_starts[line_index];
        let column = self.source[line_start..offset.min(self.source.len())]
            .chars()
            .count()
            + 1;
        (line_index + 1, column)
    }

    /// The opening tag's own source, clipped so a finding never carries a whole
    /// minified line into the report.
    pub fn opening_tag(&self, element: &Element) -> String {
        let raw = &self.source[element.start..element.open_end.min(self.source.len())];
        let collapsed = raw.split_whitespace().collect::<Vec<_>>().join(" ");
        clip(&collapsed, 160)
    }

    /// The raw child source of an element, empty when it has no children.
    pub fn children_source(&self, element: &Element) -> &str {
        match element.children {
            Some((start, end)) => &self.source[start..end],
            None => "",
        }
    }
}

/// Truncates on a character boundary with an ellipsis marker.
fn clip(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let head: String = text.chars().take(max_chars).collect();
    format!("{head}…")
}

/// HTML elements that never have children; in JSX they are written self-closing
/// but authors do write `<img>` in `.jsx` too, so the list applies everywhere.
const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// Elements whose content is text, not markup. Scanning into them would make a
/// `<style>` rule body or a string literal in `<script>` look like tags.
const RAW_TEXT_ELEMENTS: &[&str] = &["script", "style"];

/// Scans a source document.
pub fn scan(source: &str, language: Language) -> Document<'_> {
    let mut elements: Vec<Element> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    let bytes = source.as_bytes();
    let mut cursor = 0usize;

    while let Some(offset) = find_byte(bytes, cursor, b'<') {
        cursor = offset + 1;
        if cursor >= bytes.len() {
            break;
        }
        // Comments and doctypes: skipped whole, so `<!-- <img> -->` never becomes
        // a finding the author cannot act on.
        if bytes[cursor..].starts_with(b"!--") {
            cursor = find_slice(bytes, cursor, b"-->").map_or(bytes.len(), |end| end + 3);
            continue;
        }
        if bytes[cursor] == b'!' || bytes[cursor] == b'?' {
            cursor = find_byte(bytes, cursor, b'>').map_or(bytes.len(), |end| end + 1);
            continue;
        }
        if bytes[cursor] == b'/' {
            let name_start = cursor + 1;
            let Some(name_end) = tag_name_end(bytes, name_start) else {
                continue;
            };
            let name = source[name_start..name_end].to_ascii_lowercase();
            let Some(close_end) = find_byte(bytes, name_end, b'>') else {
                break;
            };
            // Close the nearest open element of that name and discard whatever was
            // left unclosed inside it. Markup in the wild is not well-formed and a
            // strict match would abandon the rest of the file.
            if let Some(depth) = stack
                .iter()
                .rposition(|&index| elements[index].name == name)
            {
                let index = stack[depth];
                elements[index].children = Some((elements[index].open_end, offset));
                stack.truncate(depth);
            }
            cursor = close_end + 1;
            continue;
        }
        let Some(name_end) = tag_name_end(bytes, cursor) else {
            continue;
        };
        let raw_name = source[cursor..name_end].to_string();
        let name = raw_name.to_ascii_lowercase();
        let (attributes, has_spread, self_closing, open_end) =
            parse_attributes(source, name_end, language);

        let element = Element {
            name: name.clone(),
            raw_name,
            attributes,
            start: offset,
            open_end,
            children: None,
            has_spread,
            parent: stack.last().copied(),
        };
        elements.push(element);
        let index = elements.len() - 1;
        cursor = open_end;

        if RAW_TEXT_ELEMENTS.contains(&name.as_str()) && !self_closing {
            let closer = format!("</{name}");
            let end = find_slice_ci(bytes, cursor, closer.as_bytes()).unwrap_or(bytes.len());
            elements[index].children = Some((cursor, end));
            cursor = end;
            continue;
        }
        if !self_closing && !VOID_ELEMENTS.contains(&name.as_str()) {
            stack.push(index);
        }
    }

    Document {
        source,
        language,
        elements,
        line_starts: line_starts(source),
    }
}

/// Byte offsets at which each line begins, including a leading 0.
fn line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    starts.extend(
        source
            .bytes()
            .enumerate()
            .filter(|(_, b)| *b == b'\n')
            .map(|(i, _)| i + 1),
    );
    starts
}

fn find_byte(bytes: &[u8], from: usize, needle: u8) -> Option<usize> {
    bytes
        .get(from..)?
        .iter()
        .position(|b| *b == needle)
        .map(|p| p + from)
}

fn find_slice(bytes: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    bytes
        .get(from..)?
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}

fn find_slice_ci(bytes: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    bytes
        .get(from..)?
        .windows(needle.len())
        .position(|w| w.eq_ignore_ascii_case(needle))
        .map(|p| p + from)
}

/// End offset of a tag name starting at `from`, or `None` when `<` was not a tag
/// at all (`a < b` in a script, a stray `<` in text).
fn tag_name_end(bytes: &[u8], from: usize) -> Option<usize> {
    if !bytes.get(from)?.is_ascii_alphabetic() {
        return None;
    }
    let mut index = from;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte.is_ascii_alphanumeric()
            || byte == b'-'
            || byte == b'_'
            || byte == b'.'
            || byte == b':'
        {
            index += 1;
        } else {
            break;
        }
    }
    Some(index)
}

/// Parses attributes from just after the tag name to just past the closing `>`.
///
/// Returns the attributes, whether a JSX spread was seen, whether the tag was
/// self-closing, and the offset past the `>`.
fn parse_attributes(
    source: &str,
    from: usize,
    language: Language,
) -> (Vec<Attribute>, bool, bool, usize) {
    let bytes = source.as_bytes();
    let mut attributes = Vec::new();
    let mut has_spread = false;
    let mut self_closing = false;
    let mut index = from;

    while index < bytes.len() {
        match bytes[index] {
            b if b.is_ascii_whitespace() => index += 1,
            b'>' => {
                index += 1;
                break;
            }
            b'/' => {
                self_closing = true;
                index += 1;
            }
            b'{' => {
                // `<img {...props} />`: the tag may receive any attribute from a
                // value the scanner cannot read.
                has_spread = true;
                index = skip_braces(bytes, index);
            }
            _ => {
                let name_start = index;
                while index < bytes.len()
                    && !bytes[index].is_ascii_whitespace()
                    && bytes[index] != b'='
                    && bytes[index] != b'>'
                    && bytes[index] != b'/'
                {
                    index += 1;
                }
                if index == name_start {
                    index += 1;
                    continue;
                }
                let raw_name = &source[name_start..index];
                let mut lookahead = index;
                while lookahead < bytes.len() && bytes[lookahead].is_ascii_whitespace() {
                    lookahead += 1;
                }
                let (value, next) = if bytes.get(lookahead) == Some(&b'=') {
                    read_value(source, lookahead + 1)
                } else {
                    (AttrValue::Bare, index)
                };
                index = next;
                let (name, forced_dynamic) = normalize_name(raw_name, language);
                let value = if forced_dynamic {
                    AttrValue::Expression
                } else {
                    value
                };
                attributes.push(Attribute { name, value });
            }
        }
    }
    (attributes, has_spread, self_closing, index)
}

/// Normalizes an attribute name and reports whether the binding syntax itself
/// makes the value dynamic.
///
/// `:alt="caption"` reads as a literal `caption` if taken at face value, which
/// would let a Vue template pass a rule it should merely be exempt from. The
/// binding prefix is what makes it unknown, not the quoting.
fn normalize_name(raw: &str, language: Language) -> (String, bool) {
    let lowered = raw.to_ascii_lowercase();
    match language {
        Language::Jsx => {
            let mapped = match lowered.as_str() {
                "classname" => "class",
                "htmlfor" => "for",
                other => other,
            };
            (mapped.to_string(), false)
        }
        Language::Vue => {
            if let Some(rest) = lowered.strip_prefix("v-bind:") {
                (strip_modifiers(rest), true)
            } else if let Some(rest) = lowered.strip_prefix(':') {
                (strip_modifiers(rest), true)
            } else if let Some(rest) = lowered.strip_prefix('@') {
                (format!("on{}", strip_modifiers(rest)), false)
            } else {
                (lowered, false)
            }
        }
        Language::Html => (lowered, false),
    }
}

fn strip_modifiers(name: &str) -> String {
    name.split('.').next().unwrap_or(name).to_string()
}

/// Reads an attribute value starting at `from`, returning it and the next offset.
fn read_value(source: &str, from: usize) -> (AttrValue, usize) {
    let bytes = source.as_bytes();
    let mut index = from;
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    match bytes.get(index) {
        Some(&quote @ (b'"' | b'\'')) => {
            let start = index + 1;
            let end = find_byte(bytes, start, quote).unwrap_or(bytes.len());
            (
                AttrValue::Literal(source[start..end.min(source.len())].to_string()),
                (end + 1).min(bytes.len()),
            )
        }
        Some(b'{') => (AttrValue::Expression, skip_braces(bytes, index)),
        Some(_) => {
            // An unquoted value may legally contain `/` (`href=/contact`), so the
            // slash only ends the value when it is the one closing the tag.
            let start = index;
            while index < bytes.len()
                && !bytes[index].is_ascii_whitespace()
                && bytes[index] != b'>'
                && !(bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'>'))
            {
                index += 1;
            }
            (AttrValue::Literal(source[start..index].to_string()), index)
        }
        None => (AttrValue::Bare, index),
    }
}

/// Skips a balanced `{...}` run, tolerating quoted braces inside it.
fn skip_braces(bytes: &[u8], from: usize) -> usize {
    let mut depth = 0usize;
    let mut index = from;
    let mut quote: Option<u8> = None;
    while index < bytes.len() {
        let byte = bytes[index];
        match quote {
            Some(open) => {
                if byte == b'\\' {
                    index += 1;
                } else if byte == open {
                    quote = None;
                }
            }
            None => match byte {
                b'"' | b'\'' | b'`' => quote = Some(byte),
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return index + 1;
                    }
                }
                _ => {}
            },
        }
        index += 1;
    }
    bytes.len()
}
