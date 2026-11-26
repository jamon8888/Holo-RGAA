//! TDD for ticket #429 — éléments transverses évalués une fois pour l'échantillon.
//!
//! The expected hashes below were computed independently with a reference
//! FNV-1a 64 implementation in Python, not from the Rust code under test.
//! The status expectations come from Ara's documented rule ("NC if NC on one
//! page, C if C or NA everywhere with at least one C"), not from the code.

use rgaa_core::{AuditType, CriterionStatus};
use rgaa_core::{Classification, CriterionResult, PageResult, Violation};
use rgaa_orchestrator::assemble_site_audit;
use rgaa_orchestrator::transverse::{
    apply_transverse_pages, detect_transverse_landmarks, normalized_subtree_hash,
    transverse_page_from, LandmarkNode, LandmarkSubtree, PageLandmarks, TransverseSignature,
    TRANSVERSE_PAGE_URL, TRANSVERSE_THRESHOLD,
};

fn node(tag: &str, role: &str, label: &str, text: &str) -> LandmarkNode {
    LandmarkNode {
        tag: tag.into(),
        role: role.into(),
        label: label.into(),
        text: text.into(),
    }
}

fn banner() -> LandmarkSubtree {
    LandmarkSubtree {
        kind: "banner".into(),
        nodes: vec![
            node("header", "banner", "", ""),
            node("a", "", "", "accueil"),
            node("a", "", "", "contact"),
        ],
    }
}

fn navigation(last: &str) -> LandmarkSubtree {
    LandmarkSubtree {
        kind: "navigation".into(),
        nodes: vec![
            node("nav", "navigation", "menu principal", ""),
            node("ul", "", "", ""),
            node("li", "", "", last),
        ],
    }
}

fn footer() -> LandmarkSubtree {
    LandmarkSubtree {
        kind: "contentinfo".into(),
        nodes: vec![
            node("footer", "contentinfo", "", ""),
            node("p", "", "", "mentions légales"),
        ],
    }
}

fn sample_pages(count: usize, last_nav_items: &[&str]) -> Vec<PageLandmarks> {
    (0..count)
        .map(|index| PageLandmarks {
            url: format!("https://example.test/page-{index}"),
            landmarks: vec![
                banner(),
                navigation(last_nav_items[index % last_nav_items.len()]),
                footer(),
            ],
        })
        .collect()
}

#[test]
fn normalized_hash_matches_an_independent_fnv1a_reference() {
    assert_eq!(
        normalized_subtree_hash(&banner()),
        "fnv1a64:0affa16b380d5a8d"
    );
    assert_eq!(
        normalized_subtree_hash(&navigation("accueil")),
        "fnv1a64:53305c5e9a4f308e"
    );
    assert_eq!(
        normalized_subtree_hash(&footer()),
        "fnv1a64:34d835ee80dcff8c"
    );
    // A different text node must change the hash: the signature is not just a
    // landmark-kind label that would collide on every site.
    assert_ne!(
        normalized_subtree_hash(&navigation("accueil")),
        normalized_subtree_hash(&navigation("contact"))
    );
}

#[test]
fn threshold_is_strictly_greater_than_eighty_percent() {
    assert!((TRANSVERSE_THRESHOLD - 0.8).abs() < f64::EPSILON);
    // 4/5 = exactly 80 % is NOT "more than 80 %": the fifth page has a
    // different header, menu and footer, so no signature is repeated enough.
    let mut pages = sample_pages(5, &["accueil"]);
    pages[4] = PageLandmarks {
        url: "https://example.test/page-4".into(),
        landmarks: vec![
            LandmarkSubtree {
                kind: "banner".into(),
                nodes: vec![node("header", "banner", "", "unique")],
            },
            navigation("unique"),
            footer_with_text("unique"),
        ],
    };
    assert!(detect_transverse_landmarks(&pages).is_empty());
    // 5/6 ≈ 83 % is.
    let found = detect_transverse_landmarks(&sample_pages(6, &["accueil"]));
    assert_eq!(found.len(), 3);
}

#[test]
fn detection_keeps_only_repeated_landmarks_in_first_seen_order() {
    let mut pages = sample_pages(6, &["accueil"]);
    // A footer that changes on each page must not be treated as transverse,
    // while the stable banner and navigation are.
    for (index, page) in pages.iter_mut().enumerate() {
        page.landmarks[2] = footer_with_text(&format!("page {index}"));
    }
    let found = detect_transverse_landmarks(&pages);
    assert_eq!(
        found,
        vec![
            TransverseSignature::new("banner", normalized_subtree_hash(&banner())),
            TransverseSignature::new(
                "navigation",
                normalized_subtree_hash(&navigation("accueil"))
            ),
        ]
    );
}

#[test]
fn detection_requires_at_least_two_pages_and_known_landmarks() {
    assert!(detect_transverse_landmarks(&sample_pages(1, &["accueil"])).is_empty());
    let pages = vec![PageLandmarks {
        url: "https://example.test/a".into(),
        landmarks: vec![LandmarkSubtree {
            kind: "complementary".into(),
            nodes: vec![node("aside", "complementary", "", "")],
        }],
    }];
    assert!(detect_transverse_landmarks(&pages).is_empty());
}

fn footer_with_text(text: &str) -> LandmarkSubtree {
    LandmarkSubtree {
        kind: "contentinfo".into(),
        nodes: vec![
            node("footer", "contentinfo", "", ""),
            node("p", "", "", text),
        ],
    }
}

fn page_with_fail(url: &str, rule: &str) -> PageResult {
    PageResult {
        url: url.into(),
        title: Some("Page".into()),
        criteria: vec![CriterionResult {
            criterion_id: "1.1".into(),
            title: "Images".into(),
            classification: Classification::Deterministe,
            status: CriterionStatus::Fail,
            violations: vec![Violation {
                rule_id: rule.into(),
                impact: "critical".into(),
                description: "Missing alt".into(),
                nodes_affected: 1,
                nodes: vec![],
            }],
            confidence: None,
            raw_confidence: None,
            justification: Some("axe".into()),
            source: "axe-core".into(),
            citations: vec![],
            considered_sources: vec![],
            tests: vec![],
            automated_verdict: Some(rgaa_core::AutomatedVerdict::Fail),
            verdict_basis: vec![rgaa_core::VerdictBasis::Axe],
            evidence: vec![],
            confidence_calibration_version: None,
            review_required: false,
            review_reason: None,
            verified_status: None,
            review_events: vec![],
        }],
        compliance_rate: 0.0,
        crawl_depth: 0,
    }
}

#[test]
fn repeated_rule_findings_stay_page_local_without_target_identity() {
    let mut pages = vec![
        page_with_fail("https://example.test/a", "image-alt"),
        page_with_fail("https://example.test/b", "image-alt"),
        page_with_fail("https://example.test/c", "image-alt"),
    ];
    let signatures = detect_transverse_landmarks(&sample_pages(3, &["accueil"]));
    assert_eq!(signatures.len(), 3);

    apply_transverse_pages(&mut pages, &signatures, AuditType::Complet);

    assert_eq!(pages.len(), 3);
    assert!(pages.iter().all(|page| {
        page.criteria[0].violations.len() == 1
            && page.criteria[0].violations[0].rule_id == "image-alt"
    }));
}

#[test]
fn the_transverse_page_takes_its_finding_from_any_page_not_only_the_first() {
    let mut pages = vec![
        page_with_fail("https://example.test/a", "local-only"),
        page_with_fail("https://example.test/b", "image-alt"),
        page_with_fail("https://example.test/c", "image-alt"),
        page_with_fail("https://example.test/d", "image-alt"),
        page_with_fail("https://example.test/e", "image-alt"),
        page_with_fail("https://example.test/f", "image-alt"),
    ];
    let signatures = detect_transverse_landmarks(&sample_pages(6, &["accueil"]));
    apply_transverse_pages(&mut pages, &signatures, AuditType::Complet);

    assert_eq!(pages.len(), 6);
    // Both page-local and repeated-rule findings stay where they were found.
    assert_eq!(pages[0].criteria[0].violations[0].rule_id, "local-only");
    assert!(pages[1..]
        .iter()
        .all(|page| { page.criteria[0].violations[0].rule_id == "image-alt" }));
}

#[test]
fn a_finding_below_the_threshold_stays_on_every_page() {
    let mut pages = vec![
        page_with_fail("https://example.test/a", "image-alt"),
        page_with_fail("https://example.test/b", "image-alt"),
        page_with_fail("https://example.test/c", "local-only"),
    ];
    let signatures = detect_transverse_landmarks(&sample_pages(3, &["accueil"]));
    apply_transverse_pages(&mut pages, &signatures, AuditType::Complet);

    assert!(pages.iter().all(|page| page.url != TRANSVERSE_PAGE_URL));
    assert!(pages
        .iter()
        .all(|page| page.criteria[0].violations.len() == 1));
    assert!(pages
        .iter()
        .all(|page| page.criteria[0].source == "axe-core"));
}

#[test]
fn no_transverse_landmark_means_no_transverse_page() {
    let mut pages = vec![
        page_with_fail("https://example.test/a", "image-alt"),
        page_with_fail("https://example.test/b", "image-alt"),
    ];
    apply_transverse_pages(&mut pages, &[], AuditType::Complet);
    assert_eq!(pages.len(), 2);
}

#[test]
fn a_transverse_page_can_be_built_without_mutating_the_sample() {
    let pages = vec![
        page_with_fail("https://example.test/a", "image-alt"),
        page_with_fail("https://example.test/b", "image-alt"),
    ];
    let transverse = transverse_page_from(&pages, "1.1").expect("a transverse page is built");
    assert_eq!(transverse.url, TRANSVERSE_PAGE_URL);
    assert_eq!(transverse.criteria.len(), 1);
    assert_eq!(pages.len(), 2, "the caller's sample is untouched");
}

/// Ara's rule, via the site aggregation that consumes the transverse page.
/// Expected values come from Ara's DOCUMENTATION.md, not from the code.
#[test]
fn site_rate_includes_the_transverse_page_and_accepts_pass_plus_na() {
    fn criterion(id: &str, status: CriterionStatus, source: &str) -> CriterionResult {
        CriterionResult {
            criterion_id: id.into(),
            title: "t".into(),
            classification: Classification::Deterministe,
            status: status.clone(),
            violations: vec![],
            confidence: None,
            raw_confidence: None,
            justification: None,
            source: source.into(),
            citations: vec![],
            considered_sources: vec![],
            tests: vec![],
            automated_verdict: None,
            verdict_basis: vec![],
            evidence: vec![],
            confidence_calibration_version: None,
            review_required: false,
            review_reason: None,
            verified_status: None,
            review_events: vec![],
        }
    }

    // Criterion 1.1 passes on both real pages but fails on the transverses:
    // it must be counted non-conforming for the whole site.
    let pages = vec![
        PageResult {
            url: "a".into(),
            title: None,
            criteria: vec![criterion("1.1", CriterionStatus::Pass, "axe-core")],
            compliance_rate: 100.0,
            crawl_depth: 0,
        },
        PageResult {
            url: "b".into(),
            title: None,
            criteria: vec![criterion("1.1", CriterionStatus::Pass, "axe-core")],
            compliance_rate: 100.0,
            crawl_depth: 0,
        },
        PageResult {
            url: TRANSVERSE_PAGE_URL.into(),
            title: None,
            criteria: vec![criterion("1.1", CriterionStatus::Fail, "axe-core")],
            compliance_rate: 0.0,
            crawl_depth: 0,
        },
    ];
    let (taux, _, _) = rgaa_orchestrator::aggregate_site_compliance(&pages);
    assert_eq!(
        taux, 0.0,
        "one NC on the transverses makes the criterion NC"
    );

    // C or NA everywhere with at least one C is C (Ara rule).
    let pages = vec![
        PageResult {
            url: "a".into(),
            title: None,
            criteria: vec![criterion("1.1", CriterionStatus::Pass, "axe-core")],
            compliance_rate: 100.0,
            crawl_depth: 0,
        },
        PageResult {
            url: "b".into(),
            title: None,
            criteria: vec![criterion("1.1", CriterionStatus::NotApplicable, "axe-core")],
            compliance_rate: 0.0,
            crawl_depth: 0,
        },
    ];
    let (taux, _, _) = rgaa_orchestrator::aggregate_site_compliance(&pages);
    assert_eq!(taux, 100.0, "C on one page and NA on another is C");

    // All NA is excluded from the rate.
    let pages = vec![PageResult {
        url: "a".into(),
        title: None,
        criteria: vec![criterion("1.1", CriterionStatus::NotApplicable, "axe-core")],
        compliance_rate: 0.0,
        crawl_depth: 0,
    }];
    let (taux, _, _) = rgaa_orchestrator::aggregate_site_compliance(&pages);
    assert_eq!(taux, 0.0);
}

#[test]
fn the_transverse_page_is_a_regular_page_in_the_audit_result() {
    let pages = vec![
        page_with_fail("a", "image-alt"),
        page_with_fail("b", "image-alt"),
    ];
    let audit = assemble_site_audit("https://example.test".into(), pages, 0, AuditType::Complet);
    assert_eq!(
        audit.pages.len(),
        2,
        "assembly alone does not invent a page"
    );
}
