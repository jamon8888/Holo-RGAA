//! Baseline evaluation harness — ticket #130.
//!
//! Runs a labeled set of [`BaselineCase`]s through any [`LlmBackend`] —
//! in CI, always a [`rgaa_holo::CassetteBackend`] replaying pre-recorded
//! responses, so a run never touches the network — and reports a
//! per-criterion confusion matrix, hallucination counters, and a cost
//! summary. A [`BaselineCase`] carries a `page_url`: grouping cases under
//! one URL exercises a mono-page audit; grouping them across several URLs
//! exercises the sampled-crawl multi-page shape. [`run_baseline`] treats
//! both identically — nothing here counts or special-cases pages, only
//! cases — so the same harness covers both, per the spec's "couvre l'audit
//! mono-page et l'audit crawl multi-pages".
//!
//! The very first run against a real (live, recorded) cassette becomes the
//! reference baseline #131 locks token/latency envelopes against — see
//! [`BaselineReport::cost`].

use crate::verify::{map_verdict, CONFIDENCE_THRESHOLD};
use rgaa_core::{Citation, CriterionStatus};
use rgaa_holo::LlmBackend;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Instant;

/// One labeled evaluation case fed to the harness.
pub struct BaselineCase {
    /// Which page this case belongs to — see the module docs on
    /// mono-page vs. multi-page coverage.
    pub page_url: String,
    pub criterion_id: String,
    /// The fully rendered prompt (as `PromptBuilder` would build it) to
    /// send to `backend`.
    pub prompt: String,
    /// The expected status from the test corpus's own labels.
    pub expected: CriterionStatus,
    /// Citations that would back this verdict if it proceeds — e.g. from
    /// a prior [`super::router::Router::route`] call. Empty means no RAG
    /// grounding was available for this case, which the harness counts as
    /// a hallucination risk when the case does otherwise produce a verdict
    /// (see [`HallucinationCounters::missing_citations`]).
    pub citations: Vec<Citation>,
}

/// One (expected, actual) pair and how many cases landed there, for one
/// criterion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfusionEntry {
    pub expected: CriterionStatus,
    pub actual: CriterionStatus,
    pub count: usize,
}

/// The confusion matrix for one criterion — the ticket's "matrice de
/// confusion par critère".
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct CriterionConfusion {
    pub criterion_id: String,
    pub entries: Vec<ConfusionEntry>,
}

/// Counters the ticket calls "compteurs d'hallucination".
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct HallucinationCounters {
    pub total_cases: usize,
    /// Cases that produced a non-error, non-"not applicable" verdict with
    /// zero citations — a verdict asserted without a documented source.
    pub missing_citations: usize,
    /// Cases whose confidence fell under [`CONFIDENCE_THRESHOLD`].
    pub below_confidence_threshold: usize,
    /// Invalid tool-call attempts. Always 0 for this harness, which never
    /// gives the backend a tool to call at all — kept as a field so a
    /// future live-agent run (with real tool access) reports into the same
    /// shape rather than a different one.
    pub invalid_tool_calls: usize,
}

/// Aggregate cost across every case run, from the backend's recorded (or,
/// for a live backend, measured) call metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct CostSummary {
    pub call_count: usize,
    pub total_duration_ms: u64,
    pub total_tokens: u64,
}

/// The full output of one [`run_baseline`] call.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BaselineReport {
    pub distinct_page_urls: usize,
    pub confusion: Vec<CriterionConfusion>,
    pub hallucinations: HallucinationCounters,
    pub cost: CostSummary,
}

/// Runs every case in `cases` against `backend` and returns the aggregate
/// [`BaselineReport`]. `backend` is typically a
/// [`rgaa_holo::CassetteBackend`] in CI (deterministic, no network) or a
/// real [`LlmBackend`] for a live baseline-measurement run.
pub async fn run_baseline(backend: &dyn LlmBackend, cases: &[BaselineCase]) -> BaselineReport {
    let mut page_urls: HashSet<&str> = HashSet::new();
    let mut confusion: Vec<CriterionConfusion> = Vec::new();
    let mut hallucinations = HallucinationCounters::default();
    let mut cost = CostSummary::default();

    for case in cases {
        page_urls.insert(case.page_url.as_str());
        hallucinations.total_cases += 1;

        let start = Instant::now();
        let result = backend.evaluate(&case.prompt).await;
        let elapsed_ms = start.elapsed().as_millis() as u64;
        cost.call_count += 1;
        cost.total_duration_ms += elapsed_ms;

        let actual = match &result {
            Ok(response) => {
                if response.confidence < CONFIDENCE_THRESHOLD {
                    hallucinations.below_confidence_threshold += 1;
                }
                map_verdict(response)
            }
            Err(_) => CriterionStatus::Error,
        };

        let verdict_asserted = !matches!(
            actual,
            CriterionStatus::Error | CriterionStatus::NotApplicable | CriterionStatus::NeedsReview
        );
        if verdict_asserted && case.citations.is_empty() {
            hallucinations.missing_citations += 1;
        }

        record_confusion(
            &mut confusion,
            &case.criterion_id,
            case.expected.clone(),
            actual,
        );
    }

    BaselineReport {
        distinct_page_urls: page_urls.len(),
        confusion,
        hallucinations,
        cost,
    }
}

/// As [`run_baseline`], but additionally sums `duration_ms`/`tokens` from
/// each case's recorded [`rgaa_holo::CassetteEntry`] (when `backend` is a
/// [`rgaa_holo::CassetteBackend`]) into [`CostSummary`], instead of only
/// the harness's own wall-clock measurement — the shape #131's locked
/// envelopes are measured against.
pub async fn run_baseline_with_cassette(
    backend: &rgaa_holo::CassetteBackend,
    cases: &[BaselineCase],
) -> BaselineReport {
    let mut report = run_baseline(backend, cases).await;
    report.cost = CostSummary::default();
    for case in cases {
        if let Some(entry) = backend.entry_for(&case.prompt) {
            report.cost.call_count += 1;
            report.cost.total_duration_ms += entry.duration_ms;
            report.cost.total_tokens += u64::from(entry.tokens.unwrap_or(0));
        }
    }
    report
}

fn record_confusion(
    confusion: &mut Vec<CriterionConfusion>,
    criterion_id: &str,
    expected: CriterionStatus,
    actual: CriterionStatus,
) {
    let bucket = match confusion
        .iter_mut()
        .find(|c| c.criterion_id == criterion_id)
    {
        Some(existing) => existing,
        None => {
            confusion.push(CriterionConfusion {
                criterion_id: criterion_id.to_string(),
                entries: Vec::new(),
            });
            confusion.last_mut().expect("just pushed")
        }
    };
    match bucket
        .entries
        .iter_mut()
        .find(|e| e.expected == expected && e.actual == actual)
    {
        Some(entry) => entry.count += 1,
        None => bucket.entries.push(ConfusionEntry {
            expected,
            actual,
            count: 1,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgaa_holo::{Cassette, CassetteBackend, HoloResponse};

    fn response(verdict: &str, confidence: f64) -> HoloResponse {
        HoloResponse {
            verdict: verdict.to_string(),
            confidence,
            justification: "test".to_string(),
        }
    }

    fn cited(test_id: &str) -> Vec<Citation> {
        vec![Citation::referentiel(test_id, "2024.1")]
    }

    #[tokio::test]
    async fn mono_page_report_counts_one_distinct_url() {
        let mut cassette = Cassette::new();
        cassette.record(
            "prompt for 1.1 on page a",
            response("fail", 0.9),
            100,
            Some(50),
        );
        cassette.record(
            "prompt for 1.2 on page a",
            response("pass", 0.9),
            110,
            Some(55),
        );
        let backend = CassetteBackend::new("cassette", "m", cassette);

        let cases = vec![
            BaselineCase {
                page_url: "https://example.test/".into(),
                criterion_id: "1.1".into(),
                prompt: "prompt for 1.1 on page a".into(),
                expected: CriterionStatus::Fail,
                citations: cited("1.1.1"),
            },
            BaselineCase {
                page_url: "https://example.test/".into(),
                criterion_id: "1.2".into(),
                prompt: "prompt for 1.2 on page a".into(),
                expected: CriterionStatus::Pass,
                citations: cited("1.2.1"),
            },
        ];

        let report = run_baseline(&backend, &cases).await;
        assert_eq!(report.distinct_page_urls, 1, "mono-page audit: one URL");
        assert_eq!(report.hallucinations.total_cases, 2);
        assert_eq!(report.hallucinations.missing_citations, 0);
        assert_eq!(report.cost.call_count, 2);
    }

    #[tokio::test]
    async fn sampled_multi_page_report_counts_every_distinct_url() {
        let mut cassette = Cassette::new();
        cassette.record("prompt on home", response("pass", 0.9), 100, Some(20));
        cassette.record("prompt on contact", response("fail", 0.9), 120, Some(30));
        cassette.record("prompt on legal", response("pass", 0.9), 90, Some(15));
        let backend = CassetteBackend::new("cassette", "m", cassette);

        let cases = vec![
            BaselineCase {
                page_url: "https://example.test/".into(),
                criterion_id: "1.1".into(),
                prompt: "prompt on home".into(),
                expected: CriterionStatus::Pass,
                citations: cited("1.1.1"),
            },
            BaselineCase {
                page_url: "https://example.test/contact".into(),
                criterion_id: "1.1".into(),
                prompt: "prompt on contact".into(),
                expected: CriterionStatus::Fail,
                citations: cited("1.1.1"),
            },
            BaselineCase {
                page_url: "https://example.test/legal".into(),
                criterion_id: "1.1".into(),
                prompt: "prompt on legal".into(),
                expected: CriterionStatus::Pass,
                citations: cited("1.1.1"),
            },
        ];

        let report = run_baseline(&backend, &cases).await;
        assert_eq!(
            report.distinct_page_urls, 3,
            "sampled crawl audit: three sampled pages"
        );
        // Same criterion (1.1) across pages folds into one confusion
        // bucket with per-(expected,actual) counts.
        let c11 = report
            .confusion
            .iter()
            .find(|c| c.criterion_id == "1.1")
            .unwrap();
        let total: usize = c11.entries.iter().map(|e| e.count).sum();
        assert_eq!(total, 3);
    }

    #[tokio::test]
    async fn confusion_matrix_is_per_criterion_and_correct() {
        let mut cassette = Cassette::new();
        cassette.record("p1", response("pass", 0.9), 10, None);
        cassette.record("p2", response("pass", 0.9), 10, None); // wrong: expected fail
        cassette.record("p3", response("fail", 0.9), 10, None);
        let backend = CassetteBackend::new("cassette", "m", cassette);

        let cases = vec![
            BaselineCase {
                page_url: "u".into(),
                criterion_id: "1.1".into(),
                prompt: "p1".into(),
                expected: CriterionStatus::Pass,
                citations: cited("1.1.1"),
            },
            BaselineCase {
                page_url: "u".into(),
                criterion_id: "1.1".into(),
                prompt: "p2".into(),
                expected: CriterionStatus::Fail,
                citations: cited("1.1.1"),
            },
            BaselineCase {
                page_url: "u".into(),
                criterion_id: "3.2".into(),
                prompt: "p3".into(),
                expected: CriterionStatus::Fail,
                citations: cited("3.2.1"),
            },
        ];

        let report = run_baseline(&backend, &cases).await;
        assert_eq!(report.confusion.len(), 2, "one bucket per criterion");

        let c11 = report
            .confusion
            .iter()
            .find(|c| c.criterion_id == "1.1")
            .unwrap();
        assert!(c11.entries.contains(&ConfusionEntry {
            expected: CriterionStatus::Pass,
            actual: CriterionStatus::Pass,
            count: 1,
        }));
        assert!(c11.entries.contains(&ConfusionEntry {
            expected: CriterionStatus::Fail,
            actual: CriterionStatus::Pass, // model said pass, expected fail
            count: 1,
        }));

        let c32 = report
            .confusion
            .iter()
            .find(|c| c.criterion_id == "3.2")
            .unwrap();
        assert!(c32.entries.contains(&ConfusionEntry {
            expected: CriterionStatus::Fail,
            actual: CriterionStatus::Fail,
            count: 1,
        }));
    }

    #[tokio::test]
    async fn missing_citations_are_counted_only_for_asserted_verdicts() {
        let mut cassette = Cassette::new();
        cassette.record("uncited", response("fail", 0.9), 10, None);
        cassette.record("na verdict", response("na", 0.9), 10, None);
        let backend = CassetteBackend::new("cassette", "m", cassette);

        let cases = vec![
            BaselineCase {
                page_url: "u".into(),
                criterion_id: "1.1".into(),
                prompt: "uncited".into(),
                expected: CriterionStatus::Fail,
                citations: vec![], // no citation for an asserted Fail verdict
            },
            BaselineCase {
                page_url: "u".into(),
                criterion_id: "5.1".into(),
                prompt: "na verdict".into(),
                expected: CriterionStatus::NotApplicable,
                citations: vec![], // NA verdicts don't need a citation
            },
        ];

        let report = run_baseline(&backend, &cases).await;
        assert_eq!(report.hallucinations.missing_citations, 1);
    }

    #[tokio::test]
    async fn low_confidence_is_counted_as_a_hallucination_signal() {
        let mut cassette = Cassette::new();
        cassette.record("shaky", response("fail", 0.2), 10, None);
        let backend = CassetteBackend::new("cassette", "m", cassette);

        let cases = vec![BaselineCase {
            page_url: "u".into(),
            criterion_id: "1.1".into(),
            prompt: "shaky".into(),
            expected: CriterionStatus::Fail,
            citations: cited("1.1.1"),
        }];

        let report = run_baseline(&backend, &cases).await;
        assert_eq!(report.hallucinations.below_confidence_threshold, 1);
        // Below-threshold responses map to NeedsReview, which the
        // harness doesn't count as an asserted (citable) verdict.
        assert_eq!(report.hallucinations.missing_citations, 0);
    }

    #[tokio::test]
    async fn unrecorded_prompt_counts_as_an_error_not_a_panic() {
        let backend = CassetteBackend::new("cassette", "m", Cassette::new());
        let cases = vec![BaselineCase {
            page_url: "u".into(),
            criterion_id: "1.1".into(),
            prompt: "never recorded".into(),
            expected: CriterionStatus::Pass,
            citations: vec![],
        }];

        let report = run_baseline(&backend, &cases).await;
        let c11 = &report.confusion[0];
        assert_eq!(c11.entries[0].actual, CriterionStatus::Error);
    }

    #[tokio::test]
    async fn run_baseline_with_cassette_sums_recorded_cost_metadata() {
        let mut cassette = Cassette::new();
        cassette.record("p1", response("pass", 0.9), 120, Some(340));
        cassette.record("p2", response("fail", 0.9), 80, Some(210));
        let backend = CassetteBackend::new("cassette", "m", cassette);

        let cases = vec![
            BaselineCase {
                page_url: "u".into(),
                criterion_id: "1.1".into(),
                prompt: "p1".into(),
                expected: CriterionStatus::Pass,
                citations: cited("1.1.1"),
            },
            BaselineCase {
                page_url: "u".into(),
                criterion_id: "1.2".into(),
                prompt: "p2".into(),
                expected: CriterionStatus::Fail,
                citations: cited("1.2.1"),
            },
        ];

        let report = run_baseline_with_cassette(&backend, &cases).await;
        assert_eq!(report.cost.call_count, 2);
        assert_eq!(report.cost.total_duration_ms, 200);
        assert_eq!(report.cost.total_tokens, 550);
    }

    #[tokio::test]
    async fn baseline_report_round_trips_through_json_for_a_locked_fixture() {
        // #131 locks budgets against a checked-in baseline report — this
        // proves BaselineReport is stable JSON, the format such a fixture
        // would use.
        let mut cassette = Cassette::new();
        cassette.record("p1", response("pass", 0.9), 100, Some(50));
        let backend = CassetteBackend::new("cassette", "m", cassette);
        let cases = vec![BaselineCase {
            page_url: "u".into(),
            criterion_id: "1.1".into(),
            prompt: "p1".into(),
            expected: CriterionStatus::Pass,
            citations: cited("1.1.1"),
        }];
        let report = run_baseline_with_cassette(&backend, &cases).await;

        let json = serde_json::to_string(&report).unwrap();
        let reloaded: BaselineReport = serde_json::from_str(&json).unwrap();
        assert_eq!(reloaded, report);
    }
}
