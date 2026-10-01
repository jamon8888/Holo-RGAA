//! Wire types and categorisation for the `verify_fix` tool (ticket #164).
//!
//! The tool is a façade: it re-runs the existing analysis service over the
//! pages touched by a fix and hands the result to the existing baseline diff
//! (`rgaa_remediation::compare`). Nothing here decides whether a finding is
//! fixed — that judgement stays in `rgaa-remediation`; this module only
//! scopes the comparison, names the categories the way the MCP contract does,
//! and re-attaches the citations the diff cannot carry.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::tools::analyze::FindingDto;

/// Upper bound on the per-file re-verification budget.
///
/// A single unresponsive page must not hold the whole MCP call open: callers
/// may lower this, never raise it.
pub const MAX_PER_FILE_TIMEOUT_MS: u64 = 30_000;

/// Largest batch of corrected files accepted in one call, matching the
/// `remediate` batch bound so both tools fail the same way on a runaway
/// agent loop.
pub const MAX_FILES: usize = 25;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VerifyFixRequest {
    /// The audit the fixes were written against, as an `AuditBundle` exactly
    /// as `audit_url` / `get_audit_result` emit one.
    ///
    /// Taken as raw JSON and deserialised into `rgaa_core::AuditBundle`
    /// rather than re-declared as an MCP-side struct: a hand-copied mirror of
    /// the bundle would drift from the real schema the first time a field was
    /// added, and silently drop it.
    pub reference_audit: serde_json::Value,
    /// The corrected files. Each names the page that must be re-analysed to
    /// see whether the fix landed.
    pub files: Vec<CorrectedFileInput>,
    /// Per-file budget, clamped to [`MAX_PER_FILE_TIMEOUT_MS`].
    #[serde(default)]
    pub per_file_timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CorrectedFileInput {
    /// Repository path of the corrected file, echoed back so the caller can
    /// correlate outcomes; it is not read from disk.
    pub path: String,
    /// The page whose re-analysis proves or disproves this file's fix.
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FileVerificationStatus {
    /// The page was re-analysed; its findings took part in the diff.
    Verified,
    /// The per-file budget elapsed first.
    TimedOut,
    /// The analysis service refused or failed.
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FileVerificationDto {
    pub path: String,
    pub url: String,
    pub status: FileVerificationStatus,
    /// Present only for `timed_out` / `failed`.
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CitationDto {
    Referentiel {
        test_id: String,
        referentiel_version: String,
    },
    Crawl {
        url: String,
        captured_at: String,
        evidence_hash: String,
    },
}

impl From<&rgaa_core::Citation> for CitationDto {
    fn from(citation: &rgaa_core::Citation) -> Self {
        match citation {
            rgaa_core::Citation::Referentiel {
                test_id,
                referentiel_version,
            } => Self::Referentiel {
                test_id: test_id.clone(),
                referentiel_version: referentiel_version.clone(),
            },
            rgaa_core::Citation::Crawl {
                url,
                captured_at,
                evidence_hash,
            } => Self::Crawl {
                url: url.clone(),
                captured_at: captured_at.clone(),
                evidence_hash: evidence_hash.clone(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VerifiedFindingDto {
    pub finding: FindingDto,
    /// Sources backing the reference verdict for this finding's criterion,
    /// empty when the verdict was reached without retrieval.
    ///
    /// The diff works on `Finding`, which has no citation field, so a RAG
    /// verdict's evidence would be dropped on the way out unless it is
    /// re-attached here from the reference bundle's criterion results.
    pub citations: Vec<CitationDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VerifyFixResponse {
    /// Open in the reference audit, gone or passing after the fix.
    pub fixed: Vec<VerifiedFindingDto>,
    /// Still reported with the same status after the fix.
    pub remaining: Vec<VerifiedFindingDto>,
    /// Absent from the reference audit, or regressed from `Pass` to `Fail`.
    #[serde(rename = "new")]
    pub new_findings: Vec<VerifiedFindingDto>,
    /// Reference findings on a page that could not be re-analysed.
    ///
    /// They are deliberately kept out of the three categories above: a page
    /// that timed out produces no current findings, and counting its absent
    /// findings as `fixed` would report an unverified page as remediated.
    pub unverified: Vec<VerifiedFindingDto>,
    /// One entry per input file, in input order.
    pub files: Vec<FileVerificationDto>,
}

/// Which pages took part in the comparison, and which could not.
pub struct VerificationScope {
    pub verified_urls: HashSet<String>,
    pub unverified_urls: HashSet<String>,
}

/// Build the categorised response from the reference bundle and the findings
/// the re-analysis produced.
///
/// The reference is narrowed to the pages actually re-analysed before being
/// handed to `rgaa_remediation::compare`: a finding on a page nobody
/// re-scanned has no counterpart in `current`, and the diff would read that
/// absence as a fix.
pub fn categorize(
    reference: &rgaa_core::AuditBundle,
    scope: &VerificationScope,
    current_findings: Vec<rgaa_core::Finding>,
    files: Vec<FileVerificationDto>,
) -> VerifyFixResponse {
    let citations = citations_by_criterion(reference);

    let mut scoped_reference = reference.clone();
    scoped_reference.pages.clear();
    scoped_reference.findings = reference_findings(reference)
        .filter(|finding| scope.verified_urls.contains(&finding.url))
        .cloned()
        .collect();

    let mut current = scoped_reference.clone();
    current.findings = current_findings;

    let diff = rgaa_remediation::compare(&scoped_reference, &current);

    let unverified = reference_findings(reference)
        .filter(|finding| scope.unverified_urls.contains(&finding.url))
        .cloned()
        .collect::<Vec<_>>();

    // `unchanged` also holds findings that stayed `Pass`; those are not
    // outstanding work and would pad `remaining` with non-problems.
    let remaining = diff
        .unchanged
        .into_iter()
        .filter(|finding| finding.status != rgaa_core::CriterionStatus::Pass)
        .collect::<Vec<_>>();

    let mut new_findings = diff.new_findings;
    new_findings.extend(diff.regressions);

    VerifyFixResponse {
        fixed: attach(diff.resolved_findings, &citations),
        remaining: attach(remaining, &citations),
        new_findings: attach(new_findings, &citations),
        unverified: attach(unverified, &citations),
        files,
    }
}

fn reference_findings(
    bundle: &rgaa_core::AuditBundle,
) -> impl Iterator<Item = &rgaa_core::Finding> {
    bundle
        .findings
        .iter()
        .chain(bundle.pages.iter().flat_map(|page| page.findings.iter()))
}

/// Index the reference bundle's per-criterion citations so a finding can be
/// paired with the evidence behind its criterion's verdict.
fn citations_by_criterion(bundle: &rgaa_core::AuditBundle) -> HashMap<&str, Vec<CitationDto>> {
    let mut index: HashMap<&str, Vec<CitationDto>> = HashMap::new();
    for criterion in bundle.pages.iter().flat_map(|page| page.criteria.iter()) {
        if criterion.citations.is_empty() {
            continue;
        }
        index
            .entry(criterion.criterion_id.as_str())
            .or_default()
            .extend(criterion.citations.iter().map(CitationDto::from));
    }
    index
}

fn attach(
    findings: Vec<rgaa_core::Finding>,
    citations: &HashMap<&str, Vec<CitationDto>>,
) -> Vec<VerifiedFindingDto> {
    findings
        .into_iter()
        .map(|finding| {
            let cited = finding
                .criterion_id
                .as_deref()
                .and_then(|id| citations.get(id))
                .cloned()
                .unwrap_or_default();
            VerifiedFindingDto {
                finding: finding.into(),
                citations: cited,
            }
        })
        .collect()
}
