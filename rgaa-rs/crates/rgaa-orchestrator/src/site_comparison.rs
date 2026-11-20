//! Conservative comparisons for RGAA criteria whose scope is a set of pages.
//!
//! This module consumes observations from the same crawl. It never treats a
//! missing page, missing observation, or uniform heuristic as proof of Pass.

use std::collections::BTreeSet;

use rgaa_core::CriterionStatus;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub(crate) struct PageObservation {
    #[serde(default)]
    pub navigation_systems: Vec<String>,
    #[serde(default)]
    pub navigation_positions: Vec<String>,
    #[serde(default)]
    pub sitemap_access: Vec<String>,
    #[serde(default)]
    pub search_access: Vec<String>,
    /// The page holds an `input[type=password]` (authentication page).
    #[serde(default)]
    pub has_password_input: bool,
    /// Absolute http(s) links of the page (capped by the extraction script).
    #[serde(default)]
    pub page_links: Vec<String>,
}

/// A page left out of the compared set, with the reason that goes into the evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PageExclusion {
    pub url: String,
    pub reason: &'static str,
}

pub(crate) const EXCLUDED_HOME: &str = "page d'accueil (profondeur 0)";
pub(crate) const EXCLUDED_PASSWORD: &str = "page avec input[type=password]";
pub(crate) const TOO_FEW_PAGES: &str = "moins de deux pages comparables";

/// The "ensemble de pages" compared by 12.2 and 12.5 (spec §6.4): every sampled page
/// except the home page and pages holding a password field. Each exclusion is named so
/// the evidence says why a page was not compared. A page whose observation is missing
/// is neither compared nor excluded: the caller counts it as failed.
pub(crate) fn select_comparable_pages(
    sampled: &[(String, Option<PageObservation>)],
    home_url: &str,
) -> (Vec<PageObservation>, Vec<PageExclusion>) {
    let mut comparable = Vec::new();
    let mut exclusions = Vec::new();
    for (url, observation) in sampled {
        let Some(observation) = observation else {
            continue;
        };
        if url == home_url {
            exclusions.push(PageExclusion {
                url: url.clone(),
                reason: EXCLUDED_HOME,
            });
        } else if observation.has_password_input {
            exclusions.push(PageExclusion {
                url: url.clone(),
                reason: EXCLUDED_PASSWORD,
            });
        } else {
            comparable.push(observation.clone());
        }
    }
    (comparable, exclusions)
}

/// Keeps the comparison criteria (12.2, 12.5) at `review` when fewer than two pages can
/// be compared, and names every exclusion in the details of all results.
pub(crate) fn apply_comparable_set(
    results: &mut [SiteCriterionObservation],
    comparable: usize,
    exclusions: &[PageExclusion],
) {
    let excluded = if exclusions.is_empty() {
        String::new()
    } else {
        let list: Vec<String> = exclusions
            .iter()
            .map(|e| format!("{} ({})", e.url, e.reason))
            .collect();
        format!("; pages exclues: {}", list.join(", "))
    };
    for result in results.iter_mut() {
        if comparable < 2 && matches!(result.criterion_id, "12.2" | "12.5") {
            result.status = CriterionStatus::NeedsReview;
            result.details = format!("{TOO_FEW_PAGES} ({comparable}){}", excluded);
        } else {
            result.details.push_str(&excluded);
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SiteCriterionObservation {
    pub criterion_id: &'static str,
    pub status: CriterionStatus,
    pub details: String,
    pub sampled_pages: usize,
    pub failed_pages: usize,
    pub sample_complete: bool,
}

pub(crate) fn compare_site(
    pages: &[PageObservation],
    sampled_pages: usize,
    failed_pages: usize,
    sample_complete: bool,
) -> Vec<SiteCriterionObservation> {
    let enough_data = !pages.is_empty() && pages.len() == sampled_pages && failed_pages == 0;
    let mut results = Vec::with_capacity(4);

    let systems = pages.iter().map(|page| {
        page.navigation_systems
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
    });
    let system_counts: Vec<_> = systems.collect();
    let missing_systems = system_counts.iter().filter(|count| **count < 2).count();
    let no_system_pages = system_counts.iter().filter(|count| **count == 0).count();
    results.push(SiteCriterionObservation {
        criterion_id: "12.1",
        status: if system_counts.is_empty() {
            CriterionStatus::NeedsReview
        } else if no_system_pages > 0 {
            CriterionStatus::Fail
        } else {
            CriterionStatus::NeedsReview
        },
        details: format!(
            "two navigation systems inferred on {} of {} observed pages; one-system cases and RGAA exceptions remain for review",
            system_counts.len().saturating_sub(missing_systems),
            system_counts.len()
        ),
        sampled_pages,
        failed_pages,
        sample_complete,
    });

    let nav_signatures = pages
        .iter()
        // DOM order does not identify the primary menu. Compare the observed
        // regions without treating additional contextual menus as movement.
        .map(|page| normalized_set(&page.navigation_positions))
        .collect::<Vec<_>>();
    results.push(site_result(
        "12.2",
        &nav_signatures,
        "a navigation region is shared across pages; primary-menu identity and visual placement still need confirmation",
        pages.len(),
        sampled_pages,
        failed_pages,
        sample_complete,
    ));

    let sitemap_signatures = pages
        .iter()
        .map(|page| normalized_set(&page.sitemap_access))
        .collect::<Vec<_>>();
    results.push(site_result(
        "12.4",
        &sitemap_signatures,
        "sitemap access signatures are consistent; target relevance and actual reachability need confirmation",
        pages.len(),
        sampled_pages,
        failed_pages,
        sample_complete,
    ));

    let search_signatures = pages
        .iter()
        .map(|page| normalized_set(&page.search_access))
        .collect::<Vec<_>>();
    results.push(site_result(
        "12.5",
        &search_signatures,
        "search access signatures are consistent; keyboard reachability and result quality need confirmation",
        pages.len(),
        sampled_pages,
        failed_pages,
        sample_complete,
    ));

    // Incomplete crawls cannot establish the whole-set criteria. A directly
    // observed contradiction remains a failure; otherwise the result is review.
    if !enough_data || !sample_complete {
        for result in &mut results {
            if result.status != CriterionStatus::Fail {
                result.status = CriterionStatus::NeedsReview;
            }
            result
                .details
                .push_str("; crawl sample incomplete, no Pass issued");
        }
    }
    results
}

fn normalized_set(values: &[String]) -> BTreeSet<String> {
    values
        .iter()
        .map(|value| {
            value
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase()
        })
        .filter(|value| !value.is_empty())
        .collect()
}

fn all_same_nonempty(values: &[BTreeSet<String>]) -> bool {
    values
        .first()
        .is_some_and(|first| !first.is_empty() && values.iter().all(|value| value == first))
}

fn site_result(
    criterion_id: &'static str,
    signatures: &[BTreeSet<String>],
    evidence: &str,
    observed_pages: usize,
    sampled_pages: usize,
    failed_pages: usize,
    sample_complete: bool,
) -> SiteCriterionObservation {
    let consistent = if criterion_id == "12.2" {
        signatures.first().is_some_and(|first| {
            first
                .iter()
                .any(|region| signatures.iter().all(|set| set.contains(region)))
        })
    } else {
        all_same_nonempty(signatures)
    };
    let missing_signal = signatures.iter().any(BTreeSet::is_empty);
    SiteCriterionObservation {
        criterion_id,
        status: if observed_pages == 0 || consistent || missing_signal {
            CriterionStatus::NeedsReview
        } else {
            CriterionStatus::Fail
        },
        details: format!(
            "{}; observations available for {observed_pages} of {sampled_pages} sampled pages",
            if missing_signal {
                "no matching system was inferred on at least one page; applicability and alternate markup require review"
            } else if consistent {
                evidence
            } else {
                "access or primary-position signatures differ across pages; repeated-system identity and exceptions require confirmation"
            }
        ),
        sampled_pages,
        failed_pages,
        sample_complete,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_access_signals_require_review_instead_of_proving_failure() {
        let results = compare_site(
            &[
                page(&["nav"], &["top"], &[], &[]),
                page(&["nav"], &["top"], &[], &[]),
            ],
            2,
            0,
            true,
        );
        assert!(results
            .iter()
            .all(|r| r.status == CriterionStatus::NeedsReview));
        assert!(results[2].details.contains("no matching system"));
    }

    #[test]
    fn extra_contextual_menu_does_not_prove_primary_navigation_moved() {
        let results = compare_site(
            &[
                page(&["nav"], &["top"], &[], &[]),
                page(&["nav"], &["top", "middle"], &[], &[]),
            ],
            2,
            0,
            true,
        );
        assert_eq!(results[1].status, CriterionStatus::NeedsReview);
    }

    #[test]
    fn prepended_contextual_menu_does_not_prove_primary_navigation_moved() {
        let results = compare_site(
            &[
                page(&["nav"], &["top"], &[], &[]),
                page(&["nav"], &["middle", "top"], &[], &[]),
            ],
            2,
            0,
            true,
        );
        assert_eq!(results[1].status, CriterionStatus::NeedsReview);
    }

    fn page(
        nav: &[&str],
        positions: &[&str],
        sitemap: &[&str],
        search: &[&str],
    ) -> PageObservation {
        PageObservation {
            navigation_systems: nav.iter().map(|s| (*s).into()).collect(),
            navigation_positions: positions.iter().map(|s| (*s).into()).collect(),
            sitemap_access: sitemap.iter().map(|s| (*s).into()).collect(),
            search_access: search.iter().map(|s| (*s).into()).collect(),
            has_password_input: false,
            page_links: Vec::new(),
        }
    }

    #[test]
    fn consistent_site_signals_remain_review_instead_of_pass() {
        let results = compare_site(
            &[
                page(
                    &["nav:main", "search"],
                    &["top"],
                    &["footer:sitemap"],
                    &["header:search"],
                ),
                page(
                    &["nav:main", "search"],
                    &["top"],
                    &["footer:sitemap"],
                    &["header:search"],
                ),
            ],
            2,
            0,
            true,
        );
        assert!(results
            .iter()
            .all(|result| result.status == CriterionStatus::NeedsReview));
    }

    #[test]
    fn observed_differences_fail_but_missing_signals_remain_review() {
        let results = compare_site(
            &[
                page(
                    &["nav:main"],
                    &["top"],
                    &["footer:sitemap"],
                    &["header:search"],
                ),
                page(
                    &["nav:main", "search"],
                    &["bottom"],
                    &[],
                    &["sidebar:search"],
                ),
            ],
            2,
            0,
            true,
        );
        assert_eq!(results[0].status, CriterionStatus::NeedsReview);
        assert_eq!(results[1].status, CriterionStatus::Fail);
        assert_eq!(results[2].status, CriterionStatus::NeedsReview);
        assert_eq!(results[3].status, CriterionStatus::Fail);
    }

    #[test]
    fn missing_page_blocks_pass_for_every_site_comparison() {
        let results = compare_site(
            &[page(
                &["nav:main", "search"],
                &["top"],
                &["footer:sitemap"],
                &["header:search"],
            )],
            2,
            1,
            false,
        );
        assert!(results
            .iter()
            .all(|result| result.status != CriterionStatus::Pass));
        assert!(results.iter().all(|result| !result.sample_complete));
    }

    fn obs(password: bool) -> Option<PageObservation> {
        Some(PageObservation {
            has_password_input: password,
            ..PageObservation::default()
        })
    }

    #[test]
    fn home_and_password_pages_are_excluded_by_name() {
        let sampled = vec![
            ("https://x.test/".to_string(), obs(false)),
            ("https://x.test/a".to_string(), obs(false)),
            ("https://x.test/login".to_string(), obs(true)),
            ("https://x.test/b".to_string(), obs(false)),
            ("https://x.test/failed".to_string(), None),
        ];
        let (comparable, exclusions) = select_comparable_pages(&sampled, "https://x.test/");
        assert_eq!(comparable.len(), 2);
        assert_eq!(
            exclusions,
            vec![
                PageExclusion {
                    url: "https://x.test/".into(),
                    reason: EXCLUDED_HOME
                },
                PageExclusion {
                    url: "https://x.test/login".into(),
                    reason: EXCLUDED_PASSWORD
                },
            ]
        );
    }

    #[test]
    fn fewer_than_two_comparable_pages_keeps_comparison_criteria_at_review() {
        let mut results = compare_site(&[PageObservation::default()], 1, 0, true);
        apply_comparable_set(
            &mut results,
            1,
            &[PageExclusion {
                url: "https://x.test/".into(),
                reason: EXCLUDED_HOME,
            }],
        );
        for id in ["12.2", "12.5"] {
            let r = results.iter().find(|r| r.criterion_id == id).expect(id);
            assert_eq!(r.status, CriterionStatus::NeedsReview, "{id}");
            assert!(r.details.starts_with(TOO_FEW_PAGES), "{id}: {}", r.details);
            assert!(r.details.contains("page d'accueil"), "{id}");
        }
    }
}
