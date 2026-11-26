//! Conservative comparisons for RGAA criteria whose scope is a set of pages.
//!
//! This module consumes observations from the same crawl. It never treats a
//! missing page, missing observation, or uniform heuristic as proof of Pass.
#![allow(dead_code)]

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
    pub sitemap_urls: Vec<String>,
    #[serde(default)]
    pub search_access: Vec<String>,
    /// The page holds an `input[type=password]` (authentication page).
    /// Page URL, filled in by [`select_comparable_pages`] so evidence can name pages.
    #[serde(default, skip)]
    pub url: String,
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
        if same_url(url, home_url) {
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
            let mut observation = observation.clone();
            observation.url = url.clone();
            comparable.push(observation);
        }
    }
    (comparable, exclusions)
}

fn same_url(left: &str, right: &str) -> bool {
    match (reqwest::Url::parse(left), reqwest::Url::parse(right)) {
        (Ok(mut left), Ok(mut right)) => {
            left.set_fragment(None);
            right.set_fragment(None);
            left == right
        }
        _ => left == right,
    }
}

pub(crate) fn sitemap_url(
    sampled: &[(String, Option<PageObservation>)],
    home_url: &str,
) -> Option<String> {
    let base = reqwest::Url::parse(home_url).ok()?;
    sampled
        .iter()
        .filter_map(|(_, observation)| observation.as_ref())
        .flat_map(|observation| &observation.sitemap_urls)
        .find_map(|raw| {
            let url = reqwest::Url::parse(raw).ok()?;
            (matches!(url.scheme(), "http" | "https") && url.host_str() == base.host_str())
                .then(|| url.to_string())
        })
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

    let urls: Vec<String> = pages.iter().map(|page| page.url.clone()).collect();
    let sample = SampleCounts {
        observed_pages: pages.len(),
        sampled_pages,
        failed_pages,
        sample_complete,
    };
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
        &urls,
        sample,
    ));

    let sitemap_signatures = pages
        .iter()
        .map(|page| normalized_set(&page.sitemap_access))
        .collect::<Vec<_>>();
    results.push(site_result(
        "12.4",
        &sitemap_signatures,
        "sitemap access signatures are consistent; target relevance and actual reachability need confirmation",
        &urls,
        sample,
    ));

    let search_signatures = pages
        .iter()
        .map(|page| normalized_set(&page.search_access))
        .collect::<Vec<_>>();
    results.push(site_result(
        "12.5",
        &search_signatures,
        "search access signatures are consistent; keyboard reachability and result quality need confirmation",
        &urls,
        sample,
    ));

    if let Some(deviants) = navigation_order_deviants(pages) {
        if let Some(nav) = results.iter_mut().find(|r| r.criterion_id == "12.2") {
            nav.status = CriterionStatus::Fail;
            nav.details.push_str(&format!(
                "; ordre des régions de navigation dans le source différent sur: {}",
                deviants.join(", ")
            ));
        }
    }

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

/// Relative source order of the navigation regions shared by every page (#270, 12.2). The
/// extraction lists regions in document order; a page that lists the shared regions in a
/// different sequence moves the navigation relative to the rest of the page.
/// Returns the pages whose order deviates from the most common one, `None` when all agree
/// (or fewer than two regions are shared, so no order exists to compare).
fn navigation_order_deviants(pages: &[PageObservation]) -> Option<Vec<String>> {
    if pages.len() < 2 {
        return None;
    }
    let normalized: Vec<Vec<String>> = pages
        .iter()
        .map(|p| {
            p.navigation_positions
                .iter()
                .map(|v| {
                    v.split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .to_lowercase()
                })
                .filter(|v| !v.is_empty())
                .collect()
        })
        .collect();
    let shared: BTreeSet<&String> = normalized
        .iter()
        .map(|seq| seq.iter().collect::<BTreeSet<_>>())
        .reduce(|a, b| a.intersection(&b).copied().collect())?;
    if shared.len() < 2 {
        return None;
    }
    let sequences: Vec<Vec<&String>> = normalized
        .iter()
        .map(|seq| {
            let mut seen = BTreeSet::new();
            seq.iter()
                .filter(|r| shared.contains(r) && seen.insert(*r))
                .collect()
        })
        .collect();
    let mut counts: Vec<(&Vec<&String>, usize)> = Vec::new();
    for sequence in &sequences {
        match counts.iter_mut().find(|(known, _)| *known == sequence) {
            Some((_, n)) => *n += 1,
            None => counts.push((sequence, 1)),
        }
    }
    if counts.len() < 2 {
        return None;
    }
    let (majority, _) = counts.iter().max_by_key(|(_, n)| *n)?;
    Some(
        pages
            .iter()
            .zip(&sequences)
            .filter(|(_, seq)| seq != majority)
            .map(|(p, _)| p.url.clone())
            .collect(),
    )
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

/// Names the compared pages and, when signatures differ, the pages that deviate from the
/// most common signature (#270). For 12.5, a sample with no search engine at all carries
/// an applicability hint: the mechanism never emits `not_applicable` itself.
fn evidence_suffix(
    criterion_id: &str,
    signatures: &[BTreeSet<String>],
    urls: &[String],
    consistent: bool,
) -> String {
    let mut suffix = String::new();
    if !urls.is_empty() {
        suffix.push_str(&format!("; pages comparées: {}", urls.join(", ")));
    }
    if criterion_id == "12.5" && !signatures.is_empty() && signatures.iter().all(BTreeSet::is_empty)
    {
        suffix.push_str(
            "; indice d'applicabilité: aucun moteur de recherche détecté sur tout l'échantillon (à confirmer, review)",
        );
    }
    if !consistent && urls.len() == signatures.len() && signatures.iter().all(|s| !s.is_empty()) {
        let mut counts: Vec<(&BTreeSet<String>, usize)> = Vec::new();
        for signature in signatures {
            match counts.iter_mut().find(|(known, _)| *known == signature) {
                Some((_, count)) => *count += 1,
                None => counts.push((signature, 1)),
            }
        }
        if let Some((majority, _)) = counts.iter().max_by_key(|(_, count)| *count) {
            let deviating: Vec<&str> = urls
                .iter()
                .zip(signatures)
                .filter(|(_, signature)| signature != majority)
                .map(|(url, _)| url.as_str())
                .collect();
            if !deviating.is_empty() {
                suffix.push_str(&format!("; écart relevé sur: {}", deviating.join(", ")));
            }
        }
    }
    suffix
}

/// How much of the crawl sample the comparison actually saw.
#[derive(Clone, Copy)]
struct SampleCounts {
    observed_pages: usize,
    sampled_pages: usize,
    failed_pages: usize,
    sample_complete: bool,
}

fn site_result(
    criterion_id: &'static str,
    signatures: &[BTreeSet<String>],
    evidence: &str,
    urls: &[String],
    sample: SampleCounts,
) -> SiteCriterionObservation {
    let SampleCounts {
        observed_pages,
        sampled_pages,
        failed_pages,
        sample_complete,
    } = sample;
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
            "{}; observations available for {observed_pages} of {sampled_pages} sampled pages{}",
            if missing_signal {
                "no matching system was inferred on at least one page; applicability and alternate markup require review"
            } else if consistent {
                evidence
            } else {
                "access or primary-position signatures differ across pages; repeated-system identity and exceptions require confirmation"
            },
            evidence_suffix(criterion_id, signatures, urls, consistent)
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
            sitemap_urls: Vec::new(),
            search_access: search.iter().map(|s| (*s).into()).collect(),
            has_password_input: false,
            page_links: Vec::new(),
            url: String::new(),
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
            ("https://x.test".to_string(), obs(false)),
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
                    url: "https://x.test".into(),
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
    fn sitemap_selection_keeps_query_and_rejects_other_hosts() {
        let sampled = vec![(
            "https://x.test/".into(),
            Some(PageObservation {
                sitemap_urls: vec![
                    "https://outside.test/plan?view=full".into(),
                    "https://x.test/index.php?page=plan-du-site".into(),
                ],
                ..PageObservation::default()
            }),
        )];
        assert_eq!(
            sitemap_url(&sampled, "https://x.test/").as_deref(),
            Some("https://x.test/index.php?page=plan-du-site")
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

    #[test]
    fn evidence_names_compared_and_deviating_pages() {
        let mut a = page(&["nav"], &["header"], &[], &[]);
        let mut b = page(&["nav"], &["header"], &[], &[]);
        let mut c = page(&["nav"], &["footer"], &[], &[]);
        a.url = "https://x.test/a".into();
        b.url = "https://x.test/b".into();
        c.url = "https://x.test/c".into();
        let results = compare_site(&[a, b, c], 3, 0, true);
        let nav = results.iter().find(|r| r.criterion_id == "12.2").unwrap();
        assert_eq!(nav.status, CriterionStatus::Fail);
        assert!(nav.details.contains("pages comparées: https://x.test/a"));
        assert!(nav.details.contains("écart relevé sur: https://x.test/c"));
    }

    #[test]
    fn a_site_without_search_gets_an_applicability_hint_not_a_verdict() {
        let results = compare_site(
            &[
                page(&["nav"], &["header"], &[], &[]),
                page(&["nav"], &["header"], &[], &[]),
            ],
            2,
            0,
            true,
        );
        let search = results.iter().find(|r| r.criterion_id == "12.5").unwrap();
        assert_eq!(search.status, CriterionStatus::NeedsReview);
        assert!(search.details.contains("indice d'applicabilité"));
    }

    #[test]
    fn a_page_listing_navigation_regions_in_another_source_order_fails_12_2() {
        let mut a = page(&["nav"], &["header", "footer"], &[], &[]);
        let mut b = page(&["nav"], &["header", "footer"], &[], &[]);
        let mut c = page(&["nav"], &["footer", "header"], &[], &[]);
        a.url = "https://x.test/a".into();
        b.url = "https://x.test/b".into();
        c.url = "https://x.test/c".into();
        let results = compare_site(&[a, b, c], 3, 0, true);
        let nav = results.iter().find(|r| r.criterion_id == "12.2").unwrap();
        assert_eq!(nav.status, CriterionStatus::Fail);
        assert!(nav.details.contains("ordre des régions"), "{}", nav.details);
        assert!(nav.details.contains("https://x.test/c"));
    }

    #[test]
    fn same_order_or_a_single_shared_region_never_fails_on_order() {
        let a = page(&["nav"], &["header", "footer"], &[], &[]);
        let b = page(&["nav"], &["header", "footer"], &[], &[]);
        let nav = |r: &[SiteCriterionObservation]| {
            r.iter()
                .find(|x| x.criterion_id == "12.2")
                .unwrap()
                .status
                .clone()
        };
        assert_ne!(
            nav(&compare_site(&[a.clone(), b], 2, 0, true)),
            CriterionStatus::Fail
        );
        let one = page(&["nav"], &["header"], &[], &[]);
        assert_ne!(
            nav(&compare_site(&[a, one], 2, 0, true)),
            CriterionStatus::Fail
        );
    }
}
