//! Relocate browser findings back to the template that produced them.
//!
//! A finding carries a CSS selector and an HTML snippet taken from the
//! *rendered* DOM. The repository contains *templates*. Between the two sit a
//! bundler, a component tree, loops, conditionals and interpolation, so there
//! is no general inverse function from a DOM node to a file and line — a real
//! source map exists for the emitted JS, not for the DOM the JS later builds.
//!
//! This matcher therefore works the only way that stays honest without a
//! per-framework compiler: it lifts *literal* strings out of the finding (an
//! id, an `alt`, an `aria-label`, an asset name, visible text, a class token),
//! looks for them verbatim in the source tree, and keeps a hit only when the
//! element's own tag appears close enough above it to corroborate that the
//! literal really belongs to that element. Anything that stays ambiguous is
//! returned as unmappable with the reason, because a confidently wrong
//! `file:line` sends a developer to edit the wrong element, which is worse
//! than admitting the tool cannot tell.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Files above this size are skipped rather than scanned. A minified bundle or
/// a checked-in data blob can be tens of megabytes and would both stall the
/// scan and produce matches that are useless as edit targets.
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

/// Upper bound on files visited in one call, so pointing the tool at a tree
/// that still contains `node_modules` degrades into a truncated scan instead
/// of an unbounded one.
const MAX_FILES: usize = 4_000;

/// Directory nesting limit; cheap insurance against a pathological tree.
const MAX_DEPTH: usize = 32;

/// How many lines above a literal may still carry the element's opening tag.
/// Formatters routinely break one JSX/Vue element across several lines, so the
/// tag and its `alt` are often not on the same line.
const TAG_LOOKBACK_LINES: usize = 6;

/// Directory names never worth scanning; they hold build output or
/// dependencies, never the template a developer would edit.
const SKIPPED_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "dist",
    "build",
    "target",
    ".next",
    ".nuxt",
    "coverage",
    "vendor",
];

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SourceMapRequest {
    /// Root directory of the project sources. The scan never leaves it.
    pub source_root: String,
    pub findings: Vec<SourceMapFindingInput>,
}

impl SourceMapRequest {
    /// Bounded so one call cannot turn into a full-tree scan per finding.
    pub fn validate_finding_count(count: usize) -> Result<(), crate::server::McpFailure> {
        (1..=200).contains(&count).then_some(()).ok_or_else(|| {
            crate::server::McpFailure::invalid("findings must contain between 1 and 200 items")
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SourceMapFindingInput {
    pub id: String,
    /// CSS selector of the offending node, as reported by the browser.
    #[serde(default)]
    pub selector: Option<String>,
    /// `outerHTML` of the offending node. Far more useful than the selector:
    /// the selector is often purely structural (`div > p:nth-child(3)`) and
    /// carries no literal to search for.
    #[serde(default)]
    pub html: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SourceMapResponse {
    pub mapped: Vec<MappedFinding>,
    pub unmappable: Vec<UnmappableFinding>,
    pub scanned_files: usize,
    /// Files deliberately left out of the scan, with the reason. Reported so a
    /// "not found in source" answer can be told apart from "the file holding
    /// it was never read".
    pub skipped_files: Vec<SkippedFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct MappedFinding {
    pub finding_id: String,
    pub source_location: SourceLocationDto,
    pub confidence: MatchConfidence,
    /// The literal that produced the match, e.g. `alt="Chiffre d'affaires"`.
    /// Shown so a reviewer can judge the match instead of trusting it.
    pub matched_on: String,
    pub framework: SourceFlavorDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SourceLocationDto {
    /// Path relative to `source_root`.
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub snippet: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MatchConfidence {
    /// A single corroborated hit on an identifying literal (id, label, asset).
    High,
    /// A single corroborated hit on a weaker literal (text, class token),
    /// which a sibling element could plausibly share.
    Medium,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceFlavorDto {
    ReactJsx,
    VueSfc,
    Angular,
    Html,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct UnmappableFinding {
    pub finding_id: String,
    pub reason: UnmappableReason,
    /// Human-readable detail: which literal was tried, how many candidates
    /// came back, which files they were in.
    pub detail: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnmappableReason {
    /// Nothing in the finding is a literal string: a purely structural
    /// selector and no distinguishing attribute or text.
    NoDistinguishingLiteral,
    /// Literals were extracted but appear nowhere in the scanned sources.
    /// Usually means the value is interpolated at runtime (i18n, props,
    /// template expression) or lives outside `source_root`.
    NotFoundInSource,
    /// The literal was found, but the element's own tag is not within
    /// `TAG_LOOKBACK_LINES` above it, so the hit is probably a different
    /// construct that happens to contain the same string.
    ElementNotCorroborated,
    /// Several corroborated candidates. Refusing rather than picking one.
    AmbiguousMatch,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SkippedFile {
    pub file: String,
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Source tree
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceFlavor {
    ReactJsx,
    VueSfc,
    Angular,
    Html,
}

impl From<SourceFlavor> for SourceFlavorDto {
    fn from(flavor: SourceFlavor) -> Self {
        match flavor {
            SourceFlavor::ReactJsx => Self::ReactJsx,
            SourceFlavor::VueSfc => Self::VueSfc,
            SourceFlavor::Angular => Self::Angular,
            SourceFlavor::Html => Self::Html,
        }
    }
}

/// Classify by filename. The flavor is reported, not used to change the
/// search: the matcher works on attribute *values*, which survive the
/// `class`/`className`/`[class]` spelling differences between the frameworks.
fn flavor_for(name: &str) -> Option<SourceFlavor> {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".component.html") || lower.ends_with(".component.ts") {
        return Some(SourceFlavor::Angular);
    }
    if lower.ends_with(".vue") {
        return Some(SourceFlavor::VueSfc);
    }
    if lower.ends_with(".jsx") || lower.ends_with(".tsx") {
        return Some(SourceFlavor::ReactJsx);
    }
    if lower.ends_with(".html") || lower.ends_with(".htm") {
        return Some(SourceFlavor::Html);
    }
    // Plain .js/.ts are included because JSX and inline Angular templates
    // routinely live in them.
    if lower.ends_with(".js") || lower.ends_with(".ts") {
        return Some(SourceFlavor::ReactJsx);
    }
    None
}

struct SourceFile {
    relative: String,
    flavor: SourceFlavor,
    text: String,
}

struct SourceTree {
    files: Vec<SourceFile>,
    skipped: Vec<SkippedFile>,
}

/// Collect the candidate templates under `root`.
///
/// Symlinks are skipped outright rather than resolved: a link inside the
/// project pointing at `/etc` or at a sibling checkout would otherwise let the
/// scan read, and report line numbers from, files the caller never offered.
fn collect_sources(root: &Path) -> std::io::Result<SourceTree> {
    let mut files = Vec::new();
    let mut skipped = Vec::new();
    let mut stack = vec![(root.to_path_buf(), 0usize)];

    while let Some((dir, depth)) = stack.pop() {
        if depth > MAX_DEPTH {
            skipped.push(SkippedFile {
                file: relative_to(root, &dir),
                reason: format!("directory nesting exceeds {MAX_DEPTH} levels"),
            });
            continue;
        }
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(error) => {
                skipped.push(SkippedFile {
                    file: relative_to(root, &dir),
                    reason: format!("unreadable directory: {error}"),
                });
                continue;
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let meta = match std::fs::symlink_metadata(&path) {
                Ok(meta) => meta,
                Err(error) => {
                    skipped.push(SkippedFile {
                        file: relative_to(root, &path),
                        reason: format!("unreadable: {error}"),
                    });
                    continue;
                }
            };
            if meta.file_type().is_symlink() {
                skipped.push(SkippedFile {
                    file: relative_to(root, &path),
                    reason: "symlink not followed".into(),
                });
                continue;
            }
            if meta.is_dir() {
                if !SKIPPED_DIRS.contains(&name.as_str()) && !name.starts_with('.') {
                    stack.push((path, depth + 1));
                }
                continue;
            }
            let Some(flavor) = flavor_for(&name) else {
                continue;
            };
            if files.len() >= MAX_FILES {
                skipped.push(SkippedFile {
                    file: relative_to(root, &path),
                    reason: format!("scan limit of {MAX_FILES} files reached"),
                });
                continue;
            }
            if meta.len() > MAX_FILE_BYTES {
                skipped.push(SkippedFile {
                    file: relative_to(root, &path),
                    reason: format!("larger than {MAX_FILE_BYTES} bytes"),
                });
                continue;
            }
            match std::fs::read_to_string(&path) {
                Ok(text) => files.push(SourceFile {
                    relative: relative_to(root, &path),
                    flavor,
                    text,
                }),
                // Non-UTF-8 lands here; there is nothing to match line-wise.
                Err(error) => skipped.push(SkippedFile {
                    file: relative_to(root, &path),
                    reason: format!("not readable as UTF-8 text: {error}"),
                }),
            }
        }
    }

    files.sort_by(|a, b| a.relative.cmp(&b.relative));
    skipped.sort_by(|a, b| a.file.cmp(&b.file));
    Ok(SourceTree { files, skipped })
}

fn relative_to(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

// ---------------------------------------------------------------------------
// Finding → searchable literals
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum LiteralKind {
    /// `id` — unique by definition in a valid document.
    Id,
    /// Accessible name and friends; these are exactly the strings an RGAA
    /// finding is usually about, and they are normally written in the template.
    Label,
    /// Asset reference; the basename survives bundler hashing less often than
    /// the full URL, so only the basename is searched.
    Resource,
    /// Visible text content.
    Text,
    /// A class token; weakest, since utility classes repeat everywhere.
    Class,
}

impl LiteralKind {
    fn confidence(self) -> MatchConfidence {
        match self {
            Self::Id | Self::Label | Self::Resource => MatchConfidence::High,
            Self::Text | Self::Class => MatchConfidence::Medium,
        }
    }

    fn min_len(self) -> usize {
        match self {
            // A two-character id is still a deliberate identifier; a
            // two-character class or text fragment is noise.
            Self::Id | Self::Label | Self::Resource => 2,
            Self::Text | Self::Class => 4,
        }
    }
}

#[derive(Debug, Clone)]
struct Literal {
    kind: LiteralKind,
    /// How the match is described back to the caller.
    label: String,
    value: String,
}

#[derive(Debug, Default)]
struct ElementQuery {
    tag: Option<String>,
    literals: Vec<Literal>,
}

const LABEL_ATTRS: &[&str] = &[
    "alt",
    "aria-label",
    "title",
    "placeholder",
    "aria-labelledby",
    "aria-describedby",
    "for",
    "name",
    "value",
];

/// True for values that are template syntax rather than a rendered literal.
/// They reach us when a caller passes template markup instead of rendered DOM;
/// searching for `{item.alt}` would match the template but tells us nothing
/// about which *instance* of the loop the finding came from.
fn is_template_expression(value: &str) -> bool {
    let trimmed = value.trim();
    (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with("{{") && trimmed.contains("}}"))
}

impl ElementQuery {
    fn from_finding(finding: &SourceMapFindingInput) -> Self {
        let mut query = Self::default();
        if let Some(html) = finding.html.as_deref() {
            query.absorb_html(html);
        }
        if let Some(selector) = finding.selector.as_deref() {
            query.absorb_selector(selector);
        }
        query.literals.sort_by_key(|l| l.kind);
        query.literals.dedup_by(|a, b| a.value == b.value);
        query
    }

    fn push(&mut self, kind: LiteralKind, label: String, value: &str) {
        let value = value.trim();
        if value.len() < kind.min_len() || is_template_expression(value) {
            return;
        }
        if self.literals.iter().any(|l| l.value == value) {
            return;
        }
        self.literals.push(Literal {
            kind,
            label,
            value: value.to_string(),
        });
    }

    fn absorb_attribute(&mut self, name: &str, value: &str) {
        let lower = name.to_ascii_lowercase();
        match lower.as_str() {
            "id" => self.push(LiteralKind::Id, format!("id=\"{value}\""), value),
            "src" | "href" | "poster" | "data-src" => {
                // The bundler rewrites the directory and may hash the name, but
                // the authored basename is what appears in the template.
                let base = value.rsplit(['/', '\\']).next().unwrap_or(value);
                let base = base.split(['?', '#']).next().unwrap_or(base);
                self.push(LiteralKind::Resource, format!("{lower}=\"…/{base}\""), base);
            }
            "class" | "classname" => {
                for token in value.split_whitespace() {
                    self.push(
                        LiteralKind::Class,
                        format!("class token \"{token}\""),
                        token,
                    );
                }
            }
            _ if LABEL_ATTRS.contains(&lower.as_str()) => {
                self.push(LiteralKind::Label, format!("{lower}=\"{value}\""), value)
            }
            _ => {}
        }
    }

    fn absorb_html(&mut self, html: &str) {
        let Some((tag, attributes, text)) = parse_open_tag(html) else {
            return;
        };
        self.tag.get_or_insert(tag);
        for (name, value) in attributes {
            self.absorb_attribute(&name, &value);
        }
        if let Some(text) = text {
            let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
            self.push(
                LiteralKind::Text,
                format!("text content \"{collapsed}\""),
                &collapsed,
            );
        }
    }

    /// Only the right-most compound of the selector describes the offending
    /// element; the ancestors in `main > ul li#x` belong to other elements and
    /// matching on them would point at the wrong line.
    fn absorb_selector(&mut self, selector: &str) {
        let compound = selector
            .rsplit([' ', '>', '+', '~'])
            .find(|part| !part.trim().is_empty())
            .unwrap_or(selector)
            .trim();
        let mut rest = compound;
        // Leading element name, if any.
        let tag_len = rest
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '-')
            .unwrap_or(rest.len());
        if tag_len > 0 {
            self.tag.get_or_insert(rest[..tag_len].to_ascii_lowercase());
        }
        rest = &rest[tag_len..];
        while !rest.is_empty() {
            let byte = rest.as_bytes()[0];
            rest = &rest[1..];
            match byte {
                b'#' | b'.' => {
                    let end = rest.find(['#', '.', '[', ':']).unwrap_or(rest.len());
                    let (token, tail) = rest.split_at(end);
                    if byte == b'#' {
                        self.push(LiteralKind::Id, format!("id=\"{token}\""), token);
                    } else {
                        self.push(
                            LiteralKind::Class,
                            format!("class token \"{token}\""),
                            token,
                        );
                    }
                    rest = tail;
                }
                b'[' => {
                    let end = rest.find(']').unwrap_or(rest.len());
                    let (inner, tail) = rest.split_at(end);
                    if let Some((name, value)) = inner.split_once('=') {
                        let value = value.trim_matches(|c| c == '"' || c == '\'');
                        self.absorb_attribute(name.trim(), value);
                    }
                    rest = tail.strip_prefix(']').unwrap_or(tail);
                }
                // `:nth-child(2)`, `::before` and the like are structural and
                // give the matcher nothing to search for.
                b':' => {
                    let end = rest.find(['#', '.', '[']).unwrap_or(rest.len());
                    rest = &rest[end..];
                }
                _ => {}
            }
        }
    }
}

/// Minimal scanner for the first element in an HTML fragment: tag name,
/// attributes, and the text directly inside it. A real parser is not worth a
/// dependency here — the input is one `outerHTML` snippet, and anything this
/// scanner cannot read simply yields fewer literals, which surfaces as an
/// honest `no_distinguishing_literal` rather than a wrong answer.
type OpenTag = (String, Vec<(String, String)>, Option<String>);

fn parse_open_tag(html: &str) -> Option<OpenTag> {
    let bytes = html.as_bytes();
    let start = (0..bytes.len())
        .find(|&i| bytes[i] == b'<' && bytes.get(i + 1).is_some_and(|c| c.is_ascii_alphabetic()))?;
    let mut cursor = start + 1;
    let tag_start = cursor;
    while cursor < bytes.len() && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'-') {
        cursor += 1;
    }
    let tag = html[tag_start..cursor].to_ascii_lowercase();

    let mut attributes = Vec::new();
    while cursor < bytes.len() {
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] == b'>' || bytes[cursor] == b'/' {
            break;
        }
        let name_start = cursor;
        while cursor < bytes.len()
            && !bytes[cursor].is_ascii_whitespace()
            && !matches!(bytes[cursor], b'=' | b'>' | b'/')
        {
            cursor += 1;
        }
        let name = html[name_start..cursor].to_string();
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor < bytes.len() && bytes[cursor] == b'=' {
            cursor += 1;
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            let value = if cursor < bytes.len() && matches!(bytes[cursor], b'"' | b'\'') {
                let quote = bytes[cursor];
                cursor += 1;
                let value_start = cursor;
                while cursor < bytes.len() && bytes[cursor] != quote {
                    cursor += 1;
                }
                let value = html[value_start..cursor].to_string();
                cursor = (cursor + 1).min(bytes.len());
                value
            } else {
                let value_start = cursor;
                while cursor < bytes.len()
                    && !bytes[cursor].is_ascii_whitespace()
                    && bytes[cursor] != b'>'
                {
                    cursor += 1;
                }
                html[value_start..cursor].to_string()
            };
            attributes.push((name, value));
        } else {
            attributes.push((name, String::new()));
        }
    }

    let text = html[cursor.min(html.len())..]
        .find('>')
        .map(|offset| cursor + offset + 1)
        .and_then(|body_start| {
            let body = html.get(body_start..)?;
            let end = body.find('<').unwrap_or(body.len());
            let text = body[..end].trim();
            (!text.is_empty() && !is_template_expression(text)).then(|| text.to_string())
        });

    Some((tag, attributes, text))
}

// ---------------------------------------------------------------------------
// Matching
// ---------------------------------------------------------------------------

struct Candidate {
    file_index: usize,
    line: usize,
    column_byte: usize,
}

/// Every occurrence of `value`, corroborated by the element's tag appearing on
/// the same line or just above it.
fn find_candidates(tree: &SourceTree, value: &str, tag: Option<&str>) -> (usize, Vec<Candidate>) {
    let mut raw = 0usize;
    let mut corroborated = Vec::new();
    for (file_index, file) in tree.files.iter().enumerate() {
        let lines: Vec<&str> = file.text.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            let mut from = 0usize;
            while let Some(offset) = line[from..].find(value) {
                let column_byte = from + offset;
                raw += 1;
                if tag_is_nearby(&lines, index, tag) {
                    corroborated.push(Candidate {
                        file_index,
                        line: index,
                        column_byte,
                    });
                }
                from = column_byte + value.len().max(1);
            }
        }
    }
    (raw, corroborated)
}

fn tag_is_nearby(lines: &[&str], line: usize, tag: Option<&str>) -> bool {
    // Without a tag there is nothing to corroborate against; the literal alone
    // has to carry the match, which only the strong literal kinds do.
    let Some(tag) = tag else {
        return true;
    };
    let needle = format!("<{tag}");
    let start = line.saturating_sub(TAG_LOOKBACK_LINES);
    lines[start..=line]
        .iter()
        .any(|candidate| contains_ignore_ascii_case(candidate, &needle))
}

fn contains_ignore_ascii_case(haystack: &str, needle: &str) -> bool {
    if needle.len() > haystack.len() {
        return false;
    }
    let haystack = haystack.to_ascii_lowercase();
    haystack.contains(&needle.to_ascii_lowercase())
}

fn map_one(
    tree: &SourceTree,
    finding: &SourceMapFindingInput,
) -> Result<MappedFinding, UnmappableFinding> {
    let query = ElementQuery::from_finding(finding);
    if query.literals.is_empty() {
        return Err(UnmappableFinding {
            finding_id: finding.id.clone(),
            reason: UnmappableReason::NoDistinguishingLiteral,
            detail: "the finding carries no id, label, asset name, text or class to search for; \
                     a purely structural selector cannot be traced to a template"
                .into(),
        });
    }

    let mut best_failure: Option<UnmappableFinding> = None;
    // Literals are ordered strongest first, so the first one that resolves
    // unambiguously wins and weaker literals are never consulted.
    for literal in &query.literals {
        let (raw, candidates) = find_candidates(tree, &literal.value, query.tag.as_deref());
        let failure = match candidates.len() {
            1 => {
                let candidate = &candidates[0];
                let file = &tree.files[candidate.file_index];
                let text = file.text.lines().nth(candidate.line).unwrap_or_default();
                return Ok(MappedFinding {
                    finding_id: finding.id.clone(),
                    source_location: SourceLocationDto {
                        file: file.relative.clone(),
                        line: candidate.line as u32 + 1,
                        column: text[..candidate.column_byte].chars().count() as u32 + 1,
                        snippet: text.trim().to_string(),
                    },
                    confidence: literal.kind.confidence(),
                    matched_on: literal.label.clone(),
                    framework: file.flavor.into(),
                });
            }
            0 if raw == 0 => UnmappableFinding {
                finding_id: finding.id.clone(),
                reason: UnmappableReason::NotFoundInSource,
                detail: format!(
                    "{} does not occur in the scanned sources; it is probably produced at \
                     runtime (interpolation, i18n, props) or lives outside source_root",
                    literal.label
                ),
            },
            0 => UnmappableFinding {
                finding_id: finding.id.clone(),
                reason: UnmappableReason::ElementNotCorroborated,
                detail: format!(
                    "{} occurs {raw} time(s) in the sources but never within {TAG_LOOKBACK_LINES} \
                     lines below a <{}> tag, so the occurrences are a different construct",
                    literal.label,
                    query.tag.as_deref().unwrap_or("?")
                ),
            },
            n => {
                let mut where_ = candidates
                    .iter()
                    .take(3)
                    .map(|c| format!("{}:{}", tree.files[c.file_index].relative, c.line + 1))
                    .collect::<Vec<_>>()
                    .join(", ");
                if n > 3 {
                    where_.push_str(", …");
                }
                UnmappableFinding {
                    finding_id: finding.id.clone(),
                    reason: UnmappableReason::AmbiguousMatch,
                    detail: format!(
                        "{} matches {n} locations ({where_}); refusing to guess which one the \
                         rendered element came from",
                        literal.label
                    ),
                }
            }
        };
        // Keep the most informative failure: an ambiguous or uncorroborated
        // hit tells the caller far more than "not found".
        let keep = best_failure
            .as_ref()
            .is_none_or(|current| current.reason == UnmappableReason::NotFoundInSource);
        if keep {
            best_failure = Some(failure);
        }
    }

    Err(best_failure.expect("literals is non-empty, so the loop produced a failure"))
}

/// Entry point used by the `source_map` tool.
pub fn map_findings(
    source_root: &str,
    findings: &[SourceMapFindingInput],
) -> Result<SourceMapResponse, crate::server::McpFailure> {
    let root = PathBuf::from(source_root);
    let root = root.canonicalize().map_err(|error| {
        crate::server::McpFailure::invalid(format!("source_root {source_root:?}: {error}"))
    })?;
    if !root.is_dir() {
        return Err(crate::server::McpFailure::invalid(format!(
            "source_root {source_root:?} is not a directory"
        )));
    }
    let tree = collect_sources(&root).map_err(|error| {
        crate::server::McpFailure::execution(format!("scanning {source_root:?}: {error}"))
    })?;

    let mut mapped = Vec::new();
    let mut unmappable = Vec::new();
    for finding in findings {
        match map_one(&tree, finding) {
            Ok(hit) => mapped.push(hit),
            Err(miss) => unmappable.push(miss),
        }
    }
    Ok(SourceMapResponse {
        mapped,
        unmappable,
        scanned_files: tree.files.len(),
        skipped_files: tree.skipped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(id: &str, html: Option<&str>, selector: Option<&str>) -> SourceMapFindingInput {
        SourceMapFindingInput {
            id: id.into(),
            selector: selector.map(Into::into),
            html: html.map(Into::into),
        }
    }

    #[test]
    fn the_strongest_literal_is_tried_first() {
        let query = ElementQuery::from_finding(&finding(
            "f1",
            Some(r#"<img class="thumb rounded" src="/assets/logo.png" id="hero">"#),
            None,
        ));
        assert_eq!(query.tag.as_deref(), Some("img"));
        let kinds: Vec<_> = query.literals.iter().map(|l| l.kind).collect();
        assert_eq!(
            kinds,
            vec![
                LiteralKind::Id,
                LiteralKind::Resource,
                LiteralKind::Class,
                LiteralKind::Class
            ]
        );
    }

    #[test]
    fn only_the_rightmost_selector_compound_describes_the_element() {
        let query =
            ElementQuery::from_finding(&finding("f2", None, Some("main#page > ul li#target")));
        assert_eq!(query.tag.as_deref(), Some("li"));
        let values: Vec<_> = query.literals.iter().map(|l| l.value.as_str()).collect();
        assert_eq!(values, vec!["target"]);
    }

    #[test]
    fn a_structural_selector_yields_no_literal_to_search_for() {
        let query =
            ElementQuery::from_finding(&finding("f3", None, Some("div > p:nth-child(3) > a")));
        assert!(query.literals.is_empty());
        assert_eq!(query.tag.as_deref(), Some("a"));
    }

    #[test]
    fn a_template_expression_is_not_treated_as_a_literal() {
        let query = ElementQuery::from_finding(&finding(
            "f4",
            Some(r#"<img alt={product.name} src="{{ item.url }}">"#),
            None,
        ));
        assert!(
            query.literals.is_empty(),
            "unexpected literals: {:?}",
            query.literals
        );
    }

    #[test]
    fn an_asset_reference_is_reduced_to_its_basename() {
        let query = ElementQuery::from_finding(&finding(
            "f5",
            Some(r#"<img src="https://cdn.example/static/a1b2/banner.png?v=3">"#),
            None,
        ));
        assert_eq!(query.literals[0].value, "banner.png");
    }
}
