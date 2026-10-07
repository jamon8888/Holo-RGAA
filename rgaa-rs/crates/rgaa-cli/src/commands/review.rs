use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, SecondsFormat, Utc};
use clap::ValueEnum;
use rgaa_core::{AuditResult, CriterionResult, CriterionStatus, ReviewEvent, RgaaCriteria};

use crate::commands::CommonArgs;
use crate::CliError;

static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// CLI review statuses. Other criterion statuses cannot be accepted as a human decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "snake_case")]
pub enum ReviewStatus {
    Pass,
    Fail,
    NotApplicable,
}

impl From<ReviewStatus> for CriterionStatus {
    fn from(status: ReviewStatus) -> Self {
        match status {
            ReviewStatus::Pass => Self::Pass,
            ReviewStatus::Fail => Self::Fail,
            ReviewStatus::NotApplicable => Self::NotApplicable,
        }
    }
}

/// Arguments for recording a human decision in a saved audit JSON file.
#[derive(Debug, clap::Args)]
pub struct ReviewArgs {
    /// Shared CLI options, including --output and --log-file.
    #[clap(flatten)]
    pub common: CommonArgs,
    /// Path to the audit JSON file to review.
    #[clap(long, value_name = "PATH")]
    pub input: PathBuf,
    /// Canonical RGAA criterion ID, such as 1.1.
    #[clap(long, value_name = "ID")]
    pub criterion: String,
    /// Human reviewed status: pass, fail, or not_applicable.
    #[clap(long, value_enum)]
    pub status: ReviewStatus,
    /// Reviewer name.
    #[clap(long)]
    pub author: String,
    /// Reason for the human decision.
    #[clap(long)]
    pub reason: String,
    /// Exact page URL when the criterion occurs on multiple pages.
    #[clap(long)]
    pub url: Option<String>,
}

/// Errors returned when a review cannot be applied to a criterion result.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ReviewError {
    #[error("review author must not be empty")]
    EmptyAuthor,
    #[error("review reason must not be empty")]
    EmptyReason,
    #[error("only pass, fail, and not_applicable can be reviewed")]
    UnsupportedStatus,
}

/// Adds a human review while leaving the machine prediction and its provenance intact.
pub fn apply_review(
    result: &mut CriterionResult,
    status: CriterionStatus,
    author: &str,
    reviewed_at: DateTime<Utc>,
    reason: &str,
) -> Result<(), ReviewError> {
    if !matches!(
        status,
        CriterionStatus::Pass | CriterionStatus::Fail | CriterionStatus::NotApplicable
    ) {
        return Err(ReviewError::UnsupportedStatus);
    }
    let author = author.trim();
    if author.is_empty() {
        return Err(ReviewError::EmptyAuthor);
    }
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(ReviewError::EmptyReason);
    }

    result.verified_status = Some(status.clone());
    result.status = status.clone();
    result.review_events.push(ReviewEvent {
        status,
        author: author.to_owned(),
        reviewed_at: reviewed_at.to_rfc3339_opts(SecondsFormat::Secs, true),
        reason: reason.to_owned(),
    });
    Ok(())
}

/// Reads an audit, records exactly one criterion review, and writes the updated JSON.
pub fn run(args: ReviewArgs) -> Result<i32, CliError> {
    if RgaaCriteria::find(&args.criterion).is_none() {
        return Err(CliError::invalid_input(format!(
            "unknown RGAA criterion '{}'",
            args.criterion
        )));
    }

    let raw = fs::read_to_string(&args.input).map_err(|error| {
        CliError::execution(format!(
            "failed to read audit '{}': {error}",
            args.input.display()
        ))
    })?;
    let mut audit: AuditResult = serde_json::from_str(&raw)
        .map_err(|error| CliError::invalid_input(format!("invalid audit JSON: {error}")))?;

    let (page_index, criterion_index) =
        locate_criterion(&audit, &args.criterion, args.url.as_deref())?;
    apply_review(
        &mut audit.pages[page_index].criteria[criterion_index],
        args.status.into(),
        &args.author,
        Utc::now(),
        &args.reason,
    )
    .map_err(|error| CliError::invalid_input(error.to_string()))?;

    let serialized = serde_json::to_vec_pretty(&audit).map_err(|error| {
        CliError::execution(format!("failed to serialize reviewed audit: {error}"))
    })?;
    let destination = args.common.output.as_deref().unwrap_or(&args.input);
    write_atomically(destination, &serialized).map_err(|error| {
        CliError::execution(format!(
            "failed to write reviewed audit '{}': {error}",
            destination.display()
        ))
    })?;
    Ok(0)
}

fn locate_criterion(
    audit: &AuditResult,
    criterion_id: &str,
    url: Option<&str>,
) -> Result<(usize, usize), CliError> {
    let mut matches = Vec::new();
    for (page_index, page) in audit.pages.iter().enumerate() {
        if url.is_some_and(|expected_url| page.url != expected_url) {
            continue;
        }
        for (criterion_index, criterion) in page.criteria.iter().enumerate() {
            if criterion.criterion_id == criterion_id {
                matches.push((page_index, criterion_index));
            }
        }
    }

    match matches.as_slice() {
        [] if url.is_some() => Err(CliError::invalid_input(format!(
            "criterion '{}' was not found on page '{}'",
            criterion_id,
            url.unwrap_or_default()
        ))),
        [] => Err(CliError::invalid_input(format!(
            "criterion '{}' was not found in the audit",
            criterion_id
        ))),
        [target] => Ok(*target),
        _ if url.is_none() => Err(CliError::invalid_input(format!(
            "criterion '{}' appears on multiple pages; provide --url",
            criterion_id
        ))),
        _ => Err(CliError::invalid_input(format!(
            "criterion '{}' has duplicate entries on page '{}'",
            criterion_id,
            url.unwrap_or_default()
        ))),
    }
}

fn write_atomically(destination: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let destination_permissions = match fs::metadata(destination) {
        Ok(metadata) => Some(metadata.permissions()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let file_name = destination.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "destination has no file name",
        )
    })?;

    let (temporary_path, mut file) = loop {
        let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let mut temporary_name = file_name.to_os_string();
        temporary_name.push(format!(".review-{}-{sequence}.tmp", std::process::id()));
        let candidate = parent.join(temporary_name);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => break (candidate, file),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    };

    let write_result = (|| {
        file.write_all(bytes)?;
        file.flush()?;
        file.sync_all()?;
        if let Some(permissions) = destination_permissions {
            file.set_permissions(permissions)?;
        }
        drop(file);
        fs::rename(&temporary_path, destination)
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    write_result
}
