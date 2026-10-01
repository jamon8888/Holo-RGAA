//! Static accessibility linting of source files, without a browser.
//!
//! The rest of this workspace audits *rendered pages*: it drives a browser,
//! runs axe-core and reasons about a DOM. That is the only way to be right, and
//! it is far too slow to sit in an editor loop. This crate covers the other
//! half of the problem — the defects that are already visible in the source a
//! developer is typing — and it is built to a different budget: no browser, no
//! network, no JavaScript runtime, one pass over the text, well under a
//! millisecond for a file of ordinary size.
//!
//! # Scope, stated plainly
//!
//! Four rule families are implemented: missing `alt`, unlabelled form controls,
//! nameless `<button>`, nameless `<a href>`. They run over HTML, JSX/TSX and
//! Vue single-file components. There is no JavaScript parser behind this (see
//! [`scan`] for why), so the linter cannot follow a component boundary, cannot
//! evaluate an expression and cannot see a label that lives in another file.
//! Everywhere it cannot decide, it reports nothing. It is a fast first pass,
//! not a replacement for the browser-based audit, and it must not be quoted as
//! a conformance verdict.
//!
//! # Entry points
//!
//! - [`lint_sources`] lints content the caller already holds.
//! - [`lint_paths`] reads files from disk and maps findings onto their real
//!   paths — the "source mapping" half of the API.
//!
//! Both take a [`LintOptions`] carrying the [`Profile`] and the resolved
//! [`LintConfig`].

pub mod config;
pub mod profile;
pub mod rules;
pub mod scan;

use std::path::{Path, PathBuf};
use std::time::Instant;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use config::{ConfigError, LintConfig, CONFIG_ENV_VAR, CONFIG_FILE_NAME};
pub use profile::{Profile, Reference};
pub use rules::{Finding, FixHint, RuleId, Severity};
pub use scan::Language;

/// Largest file the linter will read from disk, in bytes.
///
/// A generated bundle is not a source file, and linting one would blow the
/// per-file time budget this crate exists to keep. Refusing loudly is better
/// than a run that mysteriously takes a second per "file".
pub const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

/// Why a lint run could not be performed.
///
/// Note what is *not* here: there is no error for "the file had defects".
/// Findings are the output, not a failure.
#[derive(Debug, thiserror::Error)]
pub enum LintError {
    /// The configuration could not be honoured.
    #[error(transparent)]
    Config(#[from] ConfigError),
    /// A path in `source_files` does not exist.
    #[error("source file not found: {0}")]
    SourceNotFound(PathBuf),
    /// A path could not be read.
    #[error("source file {path} could not be read: {source}")]
    SourceUnreadable {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// A path is larger than [`MAX_FILE_BYTES`].
    #[error("source file {path} is {size} bytes, above the {MAX_FILE_BYTES} byte limit")]
    SourceTooLarge { path: PathBuf, size: u64 },
    /// The extension names no dialect the scanner implements.
    #[error(
        "cannot lint {path}: unsupported extension (supported: {})",
        Language::SUPPORTED_EXTENSIONS
    )]
    UnsupportedLanguage { path: PathBuf },
    /// The caller asked for a run with nothing in it.
    #[error("no sources to lint: provide sources or source_files")]
    NothingToLint,
}

/// One unit of input the caller already holds in memory.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Source {
    /// Path used to pick the dialect and to label findings. It need not exist
    /// on disk, but it must carry a meaningful extension.
    pub path: String,
    /// The file's text.
    pub content: String,
}

/// Settings shared by every file in a run.
#[derive(Debug, Clone)]
pub struct LintOptions {
    pub profile: Profile,
    pub config: LintConfig,
    /// Where `config` came from, carried into the report.
    pub config_source: String,
}

impl LintOptions {
    /// Resolves configuration from the user profile and applies an optional
    /// caller-supplied profile override.
    ///
    /// The caller's profile wins over the config file's: an agent asking for a
    /// Section 508 report must get one regardless of what the developer's own
    /// default is.
    pub fn resolve(
        profile: Option<Profile>,
        config_path: Option<&Path>,
    ) -> Result<Self, LintError> {
        let (config, config_source) = LintConfig::resolve(config_path)?;
        let profile = profile.unwrap_or(config.profile);
        Ok(Self {
            profile,
            config,
            config_source,
        })
    }
}

/// What one file cost and produced.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FileReport {
    pub path: String,
    pub language: Language,
    /// Bytes scanned.
    pub bytes: usize,
    /// Wall-clock microseconds for scanning plus rule evaluation.
    ///
    /// Reported per file rather than asserted, because the <100ms budget in
    /// #166 is a property of a machine as much as of this code. Measuring it
    /// here lets any caller — CI, an agent, a developer — see the real number
    /// instead of trusting a claim.
    pub duration_us: u128,
    pub finding_count: usize,
}

/// The result of a lint run.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LintReport {
    pub profile: Profile,
    /// Path of the config used, or `"built-in defaults"`.
    pub config_source: String,
    /// True when findings were mapped onto files read from disk rather than
    /// onto caller-supplied labels.
    pub source_mapped: bool,
    pub files: Vec<FileReport>,
    pub findings: Vec<Finding>,
    /// Findings whose severity is `error`.
    pub error_count: usize,
    /// Findings whose severity is `warning`.
    pub warning_count: usize,
    /// Wall-clock microseconds for the whole run.
    pub duration_us: u128,
}

impl LintReport {
    fn assemble(
        profile: Profile,
        config_source: String,
        source_mapped: bool,
        files: Vec<FileReport>,
        findings: Vec<Finding>,
        duration_us: u128,
    ) -> Self {
        let error_count = findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .count();
        let warning_count = findings
            .iter()
            .filter(|f| f.severity == Severity::Warning)
            .count();
        Self {
            profile,
            config_source,
            source_mapped,
            files,
            findings,
            error_count,
            warning_count,
            duration_us,
        }
    }
}

/// Lints sources the caller already holds.
pub fn lint_sources(sources: &[Source], options: &LintOptions) -> Result<LintReport, LintError> {
    if sources.is_empty() {
        return Err(LintError::NothingToLint);
    }
    let started = Instant::now();
    let mut files = Vec::with_capacity(sources.len());
    let mut findings = Vec::new();
    for source in sources {
        let language =
            Language::from_path(&source.path).ok_or_else(|| LintError::UnsupportedLanguage {
                path: PathBuf::from(&source.path),
            })?;
        let (report, mut file_findings) =
            lint_one(&source.path, &source.content, language, options);
        files.push(report);
        findings.append(&mut file_findings);
    }
    Ok(LintReport::assemble(
        options.profile,
        options.config_source.clone(),
        false,
        files,
        findings,
        started.elapsed().as_micros(),
    ))
}

/// Reads files from disk and lints them, mapping findings onto their real paths.
///
/// This is what `source_files` means in the MCP tool: the caller names files, we
/// read exactly those files, and every finding's `file`, `line` and `column`
/// refer to what is on disk — so an agent can apply a fix without a second
/// round-trip to ask where the code actually lives.
///
/// Every failure is loud. A path that does not exist, cannot be read, is too
/// large, or has no dialect aborts the run instead of quietly producing a
/// report with fewer files than the caller listed — a short report reads as
/// "clean", which is the one wrong answer an accessibility linter must not give.
pub fn lint_paths<P: AsRef<Path>>(
    paths: &[P],
    options: &LintOptions,
) -> Result<LintReport, LintError> {
    if paths.is_empty() {
        return Err(LintError::NothingToLint);
    }
    let started = Instant::now();
    let mut files = Vec::with_capacity(paths.len());
    let mut findings = Vec::new();

    for path in paths {
        let path = path.as_ref();
        let display = path.display().to_string();
        let language =
            Language::from_path(&display).ok_or_else(|| LintError::UnsupportedLanguage {
                path: path.to_path_buf(),
            })?;
        let metadata = std::fs::metadata(path).map_err(|source| {
            if source.kind() == std::io::ErrorKind::NotFound {
                LintError::SourceNotFound(path.to_path_buf())
            } else {
                LintError::SourceUnreadable {
                    path: path.to_path_buf(),
                    source,
                }
            }
        })?;
        if metadata.len() > MAX_FILE_BYTES {
            return Err(LintError::SourceTooLarge {
                path: path.to_path_buf(),
                size: metadata.len(),
            });
        }
        let content =
            std::fs::read_to_string(path).map_err(|source| LintError::SourceUnreadable {
                path: path.to_path_buf(),
                source,
            })?;
        let (report, mut file_findings) = lint_one(&display, &content, language, options);
        files.push(report);
        findings.append(&mut file_findings);
    }
    Ok(LintReport::assemble(
        options.profile,
        options.config_source.clone(),
        true,
        files,
        findings,
        started.elapsed().as_micros(),
    ))
}

/// Scans and evaluates one file, timing only the work itself.
fn lint_one(
    path: &str,
    content: &str,
    language: Language,
    options: &LintOptions,
) -> (FileReport, Vec<Finding>) {
    let started = Instant::now();
    let document = scan::scan(content, language);
    let findings = rules::lint_document(&document, path, &options.config, options.profile);
    let duration_us = started.elapsed().as_micros();
    (
        FileReport {
            path: path.to_string(),
            language,
            bytes: content.len(),
            duration_us,
            finding_count: findings.len(),
        },
        findings,
    )
}
