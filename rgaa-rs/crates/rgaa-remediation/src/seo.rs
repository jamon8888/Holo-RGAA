//! Routes SEO/GEO/AEO rule results through the same findings, dedup, and
//! proposal machinery as RGAA findings.
//!
//! Two entry points: [`seo_findings`] turns `rgaa_rules::seo` results into
//! [`Finding`]s (one per violation, fingerprinted like RGAA findings), and
//! [`merge_with_rgaa`] folds them into the RGAA finding list on the P0-P3
//! criticality ladder. [`HtmlAdapter`] then proposes `<head>`-level patches
//! for the few rules that have a safe deterministic fix; everything that
//! needs written content is returned as `NeedsReview`.

use crate::{
    Framework, FrameworkAdapter, PatchProposal, RemediationError, RemediationIssue, SourceLocation,
};
use indexmap::IndexMap;
use rgaa_core::{CriterionResult, CriterionStatus, EvidenceRef, Finding, FindingFingerprint};
use rgaa_rules::seo::{Criticality, SeoCatalog};

pub use rgaa_rules::seo::SOURCE as SEO_SOURCE;

pub const RGAA_SOURCE: &str = "rgaa";

/// One finding per failed rule violation, fingerprinted with the same scheme
/// as RGAA findings so dedup and baselines treat both alike.
pub fn seo_findings(
    results: &IndexMap<String, CriterionResult>,
    page_url: &str,
    evidence: &[EvidenceRef],
) -> Vec<Finding> {
    let catalog = SeoCatalog::get();
    let mut findings = Vec::new();
    for result in results.values() {
        if result.status != CriterionStatus::Fail {
            continue;
        }
        let Some(rule) = catalog.find(&result.criterion_id) else {
            continue;
        };
        for (index, violation) in result.violations.iter().enumerate() {
            let mut finding = Finding::new("");
            finding.rule = rule.id.clone();
            finding.criterion_id = Some(rule.id.clone());
            finding.url = page_url.to_string();
            finding.target = target_for(&rule.id, &rule.group).to_string();
            finding.component_path = (result.violations.len() > 1).then(|| index.to_string());
            finding.evidence = evidence.to_vec();
            finding.status = CriterionStatus::Fail;
            finding.severity = Some(rule.criticality.as_str().to_string());
            finding.description = Some(violation.description.clone());
            finding.remediation = Some(format!("{} — {}", rule.title, rule.description));
            finding.details = (!rule.rgaa_overlap.is_empty())
                .then(|| format!("rgaa_overlap: {}", rule.rgaa_overlap.join(", ")));
            finding.source = SEO_SOURCE.to_string();
            finding.id = FindingFingerprint::from_finding(&finding);
            findings.push(finding);
        }
    }
    findings
}

fn target_for(rule_id: &str, group: &str) -> &'static str {
    match rule_id {
        "SEO-META-01" | "SEO-META-02" => "head > title",
        "SEO-META-03" | "SEO-META-04" => "head > meta[name=description]",
        "SEO-META-05" => "head > meta[name=robots]",
        "SEO-LANG-01" => "html",
        "SEO-HREF-01" => "head > link[hreflang]",
        "SEO-IMG-01" => "img",
        "GEO-05" => "h1 + p",
        "AEO-04" => "h2 + p",
        _ => match group {
            "canonical" => "head > link[rel=canonical]",
            "headings" => "h1",
            "nap" => "body",
            _ => "script[type=\"application/ld+json\"]",
        },
    }
}

/// Criticality tier of any finding, RGAA or SEO.
///
/// SEO findings carry their tier from the catalog. RGAA findings map axe/RGAA
/// severities onto the ladder, and a RGAA `Fail` is never placed below P1:
/// accessibility is a legal obligation, SEO is not.
pub fn criticality_of(finding: &Finding) -> Criticality {
    if finding.source == SEO_SOURCE {
        return SeoCatalog::get()
            .find(&finding.rule)
            .map_or(Criticality::P3, |r| r.criticality);
    }
    let from_severity = finding
        .severity
        .as_deref()
        .map(|s| s.trim().to_ascii_lowercase());
    let tier = match from_severity.as_deref() {
        Some("p0") | Some("critical") | Some("blocking") | Some("bloquant") => Criticality::P0,
        Some("p1") | Some("serious") | Some("major") | Some("majeur") => Criticality::P1,
        Some("p2") | Some("moderate") | Some("minor") | Some("mineur") => Criticality::P2,
        Some("p3") | Some("info") | Some("minimal") => Criticality::P3,
        _ => match finding.status {
            CriterionStatus::Fail | CriterionStatus::Error => Criticality::P1,
            CriterionStatus::NeedsReview => Criticality::P2,
            _ => Criticality::P3,
        },
    };
    if matches!(
        finding.status,
        CriterionStatus::Fail | CriterionStatus::Error
    ) {
        tier.min(Criticality::P1)
    } else {
        tier
    }
}

/// A finding after RGAA/SEO merge: which passes flagged it and the tier that wins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergedFinding {
    pub finding: Finding,
    pub origins: Vec<&'static str>,
    pub criticality: Criticality,
}

/// Merges SEO findings into the RGAA list. An SEO finding whose rule overlaps
/// an RGAA criterion already failing on the same page is folded into that RGAA
/// finding (both origins, highest tier) rather than reported twice. Output is
/// ordered P0 → P3, RGAA-origin first within a tier.
pub fn merge_with_rgaa(rgaa: &[Finding], seo: &[Finding]) -> Vec<MergedFinding> {
    let catalog = SeoCatalog::get();
    let mut merged: Vec<MergedFinding> = rgaa
        .iter()
        .map(|f| MergedFinding {
            criticality: criticality_of(f),
            origins: vec![RGAA_SOURCE],
            finding: f.clone(),
        })
        .collect();

    for seo_finding in seo {
        let seo_tier = criticality_of(seo_finding);
        let overlap = catalog
            .find(&seo_finding.rule)
            .map(|r| r.rgaa_overlap.as_slice())
            .unwrap_or(&[]);
        let host = merged.iter_mut().find(|m| {
            m.origins.contains(&RGAA_SOURCE)
                && m.finding.url == seo_finding.url
                && matches!(
                    m.finding.status,
                    CriterionStatus::Fail | CriterionStatus::NeedsReview
                )
                && m.finding
                    .criterion_id
                    .as_deref()
                    .is_some_and(|id| overlap.iter().any(|o| o == id))
        });
        match host {
            Some(host) => {
                if !host.origins.contains(&SEO_SOURCE) {
                    host.origins.push(SEO_SOURCE);
                }
                host.criticality = host.criticality.min(seo_tier);
                let note = format!(
                    "also flagged by {} ({})",
                    seo_finding.rule,
                    seo_tier.as_str()
                );
                host.finding.details = Some(match host.finding.details.take() {
                    Some(d) if !d.is_empty() => format!("{d}; {note}"),
                    _ => note,
                });
            }
            None => merged.push(MergedFinding {
                criticality: seo_tier,
                origins: vec![SEO_SOURCE],
                finding: seo_finding.clone(),
            }),
        }
    }

    merged.sort_by_key(|m| (m.criticality, m.origins[0] != RGAA_SOURCE));
    merged
}

/// Builds the remediation input for an SEO finding. `element_html` is the
/// document `<head>` (or the relevant fragment) and `location` the template
/// file that renders it — SEO fixes are page-level, so the caller decides
/// which source file owns the head.
pub fn issue_from_finding(
    finding: &Finding,
    element_html: &str,
    location: SourceLocation,
) -> RemediationIssue {
    let mut criteria = vec![finding.rule.clone()];
    if let Some(rule) = SeoCatalog::get().find(&finding.rule) {
        criteria.extend(rule.rgaa_overlap.iter().map(|id| format!("RGAA-{id}")));
    }
    RemediationIssue {
        id: finding.id.clone(),
        rule: finding.rule.clone(),
        element_html: element_html.to_string(),
        page_url: finding.url.clone(),
        source_locations: vec![location],
        summary: finding.description.clone().unwrap_or_default(),
        remediation: finding.remediation.clone().unwrap_or_default(),
        criteria,
        framework: Some(Framework::Html),
    }
}

/// `<head>`-level adapter for plain HTML. Deterministic fixes only:
/// - `SEO-META-05`: drop `noindex`/`none` from the robots meta.
/// - `SEO-CANON-01`: insert a self-referencing canonical before `</head>`.
/// - `SEO-SCHEMA-02`: add `@context: https://schema.org` to a JSON-LD object.
///
/// Every rule that needs written content (titles, descriptions, alt text,
/// FAQ answers, schema properties) is `NeedsReview` — that content is drafted
/// through an `LlmBackend` proposal, never invented here.
pub struct HtmlAdapter;

impl FrameworkAdapter for HtmlAdapter {
    fn framework(&self) -> Framework {
        Framework::Html
    }

    fn detect(&self, source: &str) -> Option<Framework> {
        (crate::detect_framework(source) == Some(Framework::Html)).then_some(Framework::Html)
    }

    fn locate(&self, _source: &str, issue: &RemediationIssue) -> Vec<SourceLocation> {
        issue.source_locations.clone()
    }

    fn propose(
        &self,
        issue: &RemediationIssue,
        source: &str,
    ) -> Result<PatchProposal, RemediationError> {
        if issue.framework.is_some_and(|f| f != Framework::Html) {
            return Err(RemediationError::UnsupportedFramework {
                issue_id: issue.id.clone(),
            });
        }
        if source.trim().is_empty() {
            return Err(needs_review(issue, "source is empty"));
        }

        let (diff, rationale, effect) = match issue.rule.as_str() {
            "SEO-META-05" => (
                remove_noindex(issue, source)?,
                "remove the noindex directive so the page can be indexed",
                "the page becomes eligible for indexing",
            ),
            "SEO-CANON-01" => (
                insert_canonical(issue, source)?,
                "declare a self-referencing canonical URL",
                "consolidates ranking signals on this URL",
            ),
            "SEO-SCHEMA-02" => (
                add_schema_context(issue, source)?,
                "declare the schema.org @context on the JSON-LD block",
                "the structured data becomes interpretable by search engines",
            ),
            rule if rule.starts_with("SEO-")
                || rule.starts_with("GEO-")
                || rule.starts_with("AEO-") =>
            {
                return Err(needs_review(
                    issue,
                    "rule needs written content or an editorial decision; draft it through the LLM backend proposal flow",
                ));
            }
            _ => {
                return Err(needs_review(
                    issue,
                    "pattern is not high confidence for plain HTML",
                ))
            }
        };

        if diff == source {
            return Err(needs_review(
                issue,
                "remediation would not change the source",
            ));
        }
        let file = issue
            .source_locations
            .first()
            .ok_or_else(|| RemediationError::MissingSourceLocation {
                issue_id: issue.id.clone(),
            })?
            .file
            .clone();
        Ok(PatchProposal::new(
            format!("{}-proposal", issue.id),
            vec![issue.id.clone()],
            diff,
            vec![file],
            rationale,
            vec!["re-run the SEO/GEO/AEO pass to confirm the rule now passes".into()],
            vec!["rgaa audit analyze --rules seo".into()],
            effect,
        ))
    }
}

fn needs_review(issue: &RemediationIssue, reason: &str) -> RemediationError {
    RemediationError::NeedsReview {
        issue_id: issue.id.clone(),
        reason: reason.into(),
    }
}

/// Byte range of the first `<meta ...>` tag whose attributes contain `needle` (case-insensitive).
fn find_meta_tag(source: &str, needle: &str) -> Option<(usize, usize)> {
    let lower = source.to_ascii_lowercase();
    let mut cursor = 0;
    while let Some(rel) = lower[cursor..].find("<meta") {
        let start = cursor + rel;
        let end = lower[start..].find('>')? + start + 1;
        if lower[start..end].contains(needle) {
            return Some((start, end));
        }
        cursor = end;
    }
    None
}

fn attribute_span(tag: &str, name: &str) -> Option<(usize, usize)> {
    let lower = tag.to_ascii_lowercase();
    let key = format!("{name}=\"");
    let start = lower.find(&key)? + key.len();
    let end = lower[start..].find('"')? + start;
    Some((start, end))
}

fn remove_noindex(issue: &RemediationIssue, source: &str) -> Result<String, RemediationError> {
    let (start, end) = find_meta_tag(source, "name=\"robots\"")
        .ok_or_else(|| needs_review(issue, "robots meta tag not found in source"))?;
    let tag = &source[start..end];
    let (cs, ce) = attribute_span(tag, "content")
        .ok_or_else(|| needs_review(issue, "robots meta has no content attribute"))?;
    let kept: Vec<&str> = tag[cs..ce]
        .split(',')
        .map(str::trim)
        .filter(|d| {
            !d.is_empty() && !d.eq_ignore_ascii_case("noindex") && !d.eq_ignore_ascii_case("none")
        })
        .collect();
    let replacement = if kept.is_empty() {
        String::new()
    } else {
        format!("{}{}{}", &tag[..cs], kept.join(", "), &tag[ce..])
    };
    Ok(format!(
        "{}{}{}",
        &source[..start],
        replacement,
        &source[end..]
    ))
}

fn insert_canonical(issue: &RemediationIssue, source: &str) -> Result<String, RemediationError> {
    let lower = source.to_ascii_lowercase();
    if lower.contains("rel=\"canonical\"") {
        return Err(needs_review(issue, "a canonical link is already present"));
    }
    let url = issue.page_url.trim();
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(needs_review(
            issue,
            "page_url is not an absolute http(s) URL",
        ));
    }
    let close = lower
        .find("</head>")
        .ok_or_else(|| needs_review(issue, "source has no </head> to insert into"))?;
    let href = url.replace('"', "%22");
    Ok(format!(
        "{}<link rel=\"canonical\" href=\"{href}\">\n{}",
        &source[..close],
        &source[close..]
    ))
}

fn add_schema_context(issue: &RemediationIssue, source: &str) -> Result<String, RemediationError> {
    let lower = source.to_ascii_lowercase();
    let open = lower
        .find("<script type=\"application/ld+json\"")
        .ok_or_else(|| needs_review(issue, "no JSON-LD script block in source"))?;
    let body_start = lower[open..]
        .find('>')
        .map(|i| i + open + 1)
        .ok_or_else(|| needs_review(issue, "JSON-LD script tag is incomplete"))?;
    let body_end = lower[body_start..]
        .find("</script>")
        .map(|i| i + body_start)
        .ok_or_else(|| needs_review(issue, "JSON-LD script block is not closed"))?;
    let raw = &source[body_start..body_end];
    let mut value: serde_json::Value = serde_json::from_str(raw).map_err(|_| {
        needs_review(
            issue,
            "JSON-LD is not valid JSON; fix syntax first (SEO-SCHEMA-01)",
        )
    })?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| needs_review(issue, "JSON-LD root is not a single object"))?;
    if !object.contains_key("@type") && !object.contains_key("@graph") {
        return Err(needs_review(
            issue,
            "JSON-LD has no @type; the type is an editorial decision",
        ));
    }
    if object.contains_key("@context") {
        return Err(needs_review(issue, "JSON-LD already declares @context"));
    }
    object.insert(
        "@context".into(),
        serde_json::Value::String("https://schema.org".into()),
    );
    let pretty = serde_json::to_string_pretty(&value)
        .map_err(|_| needs_review(issue, "could not re-serialize JSON-LD"))?;
    Ok(format!(
        "{}\n{pretty}\n{}",
        &source[..body_start],
        &source[body_end..]
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{remediate, RemediationOutcome, RemediationPolicy};
    use rgaa_rules::seo::{BusinessProfile, PageSnapshot, SeoMapper};

    const URL: &str = "https://example.test/page";

    fn seo_results(snapshot: &PageSnapshot) -> IndexMap<String, CriterionResult> {
        SeoMapper::evaluate(snapshot, None)
    }

    fn evidence() -> Vec<EvidenceRef> {
        vec![EvidenceRef::new("dom_snapshot", "sha256:seo")]
    }

    fn location() -> SourceLocation {
        SourceLocation {
            file: "src/layout.html".into(),
            line: 1,
            column: None,
        }
    }

    fn rgaa_finding(criterion: &str, severity: Option<&str>, status: CriterionStatus) -> Finding {
        let mut f = Finding::new(format!("rgaa-{criterion}"));
        f.rule = format!("rgaa-{criterion}");
        f.criterion_id = Some(criterion.into());
        f.url = URL.into();
        f.target = "h1".into();
        f.status = status;
        f.severity = severity.map(Into::into);
        f.source = "axe-core".into();
        f
    }

    #[test]
    fn failed_rules_become_fingerprinted_findings() {
        let snapshot = PageSnapshot {
            url: URL.into(),
            meta_robots: vec!["noindex".into()],
            ..Default::default()
        };
        let findings = seo_findings(&seo_results(&snapshot), URL, &evidence());
        assert!(!findings.is_empty());
        let noindex = findings.iter().find(|f| f.rule == "SEO-META-05").unwrap();
        assert!(noindex.id.starts_with("rgaa-fp-v1-"));
        assert_eq!(noindex.severity.as_deref(), Some("P0"));
        assert_eq!(noindex.target, "head > meta[name=robots]");
        assert_eq!(noindex.source, SEO_SOURCE);
        assert_eq!(noindex.status, CriterionStatus::Fail);
        assert!(findings.iter().all(|f| f.status == CriterionStatus::Fail));
        let heading = findings.iter().find(|f| f.rule == "SEO-HEAD-01").unwrap();
        assert!(heading.details.as_deref().unwrap().contains("9.1"));
    }

    #[test]
    fn na_and_pass_rules_produce_no_findings() {
        let snapshot = PageSnapshot {
            url: URL.into(),
            ..Default::default()
        };
        let results = seo_results(&snapshot);
        let findings = seo_findings(&results, URL, &evidence());
        let failed = results
            .values()
            .filter(|r| r.status == CriterionStatus::Fail)
            .count();
        assert_eq!(findings.len(), failed);
        assert!(findings.iter().all(|f| !f.rule.starts_with("SEO-NAP")));
    }

    #[test]
    fn criticality_never_downgrades_an_rgaa_fail_below_p1() {
        assert_eq!(
            criticality_of(&rgaa_finding("9.1", Some("minor"), CriterionStatus::Fail)),
            Criticality::P1
        );
        assert_eq!(
            criticality_of(&rgaa_finding("9.1", Some("info"), CriterionStatus::Fail)),
            Criticality::P1
        );
        assert_eq!(
            criticality_of(&rgaa_finding(
                "9.1",
                Some("critical"),
                CriterionStatus::Fail
            )),
            Criticality::P0
        );
        assert_eq!(
            criticality_of(&rgaa_finding("9.1", None, CriterionStatus::Fail)),
            Criticality::P1
        );
        assert_eq!(
            criticality_of(&rgaa_finding("9.1", None, CriterionStatus::NeedsReview)),
            Criticality::P2
        );
        assert_eq!(
            criticality_of(&rgaa_finding(
                "9.1",
                Some("minor"),
                CriterionStatus::NeedsReview
            )),
            Criticality::P2
        );
    }

    #[test]
    fn overlapping_seo_finding_folds_into_rgaa_finding_with_highest_tier() {
        let snapshot = PageSnapshot {
            url: URL.into(),
            ..Default::default()
        };
        let seo = seo_findings(&seo_results(&snapshot), URL, &evidence());
        let rgaa = vec![rgaa_finding(
            "9.1",
            Some("minor"),
            CriterionStatus::NeedsReview,
        )];
        let merged = merge_with_rgaa(&rgaa, &seo);

        let host = merged
            .iter()
            .find(|m| m.finding.criterion_id.as_deref() == Some("9.1"))
            .unwrap();
        assert_eq!(host.origins, vec![RGAA_SOURCE, SEO_SOURCE]);
        assert_eq!(
            host.criticality,
            Criticality::P1,
            "SEO-HEAD-01 (P1) beats RGAA NeedsReview (P2)"
        );
        assert!(host
            .finding
            .details
            .as_deref()
            .unwrap()
            .contains("SEO-HEAD-01"));
        assert!(
            merged.iter().all(|m| m.finding.rule != "SEO-HEAD-01"),
            "folded, not duplicated"
        );
        assert!(merged
            .iter()
            .any(|m| m.finding.rule == "SEO-META-01" && m.origins == vec![SEO_SOURCE]));
    }

    #[test]
    fn merge_orders_by_tier_then_rgaa_first() {
        let snapshot = PageSnapshot {
            url: URL.into(),
            meta_robots: vec!["noindex".into()],
            ..Default::default()
        };
        let seo = seo_findings(&seo_results(&snapshot), URL, &evidence());
        let rgaa = vec![
            rgaa_finding("1.1", Some("serious"), CriterionStatus::Fail),
            rgaa_finding("11.1", Some("critical"), CriterionStatus::Fail),
        ];
        let merged = merge_with_rgaa(&rgaa, &seo);
        let tiers: Vec<Criticality> = merged.iter().map(|m| m.criticality).collect();
        let mut sorted = tiers.clone();
        sorted.sort();
        assert_eq!(tiers, sorted);
        assert_eq!(
            merged[0].finding.criterion_id.as_deref(),
            Some("11.1"),
            "RGAA P0 first"
        );
        assert_eq!(merged[1].finding.rule, "SEO-META-05", "then SEO P0");
    }

    #[test]
    fn overlap_only_folds_on_same_url() {
        let snapshot = PageSnapshot {
            url: URL.into(),
            ..Default::default()
        };
        let seo = seo_findings(&seo_results(&snapshot), URL, &evidence());
        let mut other = rgaa_finding("9.1", None, CriterionStatus::Fail);
        other.url = "https://example.test/other".into();
        let merged = merge_with_rgaa(&[other], &seo);
        assert!(merged.iter().any(|m| m.finding.rule == "SEO-HEAD-01"));
    }

    #[test]
    fn issue_carries_html_framework_and_rgaa_overlap_criteria() {
        let snapshot = PageSnapshot {
            url: URL.into(),
            ..Default::default()
        };
        let seo = seo_findings(&seo_results(&snapshot), URL, &evidence());
        let f = seo.iter().find(|f| f.rule == "SEO-LANG-01").unwrap();
        let issue = issue_from_finding(f, "<html><head></head></html>", location());
        assert_eq!(issue.framework, Some(Framework::Html));
        assert_eq!(
            issue.criteria,
            vec!["SEO-LANG-01".to_string(), "RGAA-8.3".to_string()]
        );
        assert_eq!(issue.page_url, URL);
    }

    fn issue(rule: &str, html: &str) -> RemediationIssue {
        let mut f = Finding::new(format!("f-{rule}"));
        f.rule = rule.into();
        f.url = URL.into();
        f.source = SEO_SOURCE.into();
        issue_from_finding(&f, html, location())
    }

    #[test]
    fn removes_noindex_and_keeps_other_directives() {
        let html = "<html><head><meta name=\"robots\" content=\"noindex, nofollow\"></head></html>";
        let p = HtmlAdapter
            .propose(&issue("SEO-META-05", html), html)
            .unwrap();
        assert!(p.diff.contains("content=\"nofollow\""));
        assert!(!p.diff.to_ascii_lowercase().contains("noindex"));
        assert_eq!(p.files, vec!["src/layout.html"]);
    }

    #[test]
    fn removes_robots_meta_entirely_when_only_noindex() {
        let html = "<head><meta name=\"robots\" content=\"none\"><title>t</title></head>";
        let p = HtmlAdapter
            .propose(&issue("SEO-META-05", html), html)
            .unwrap();
        assert!(!p.diff.contains("<meta"));
        assert!(p.diff.contains("<title>t</title>"));
    }

    #[test]
    fn inserts_self_canonical_before_head_close() {
        let html = "<html><head><title>t</title></head><body></body></html>";
        let p = HtmlAdapter
            .propose(&issue("SEO-CANON-01", html), html)
            .unwrap();
        assert!(p
            .diff
            .contains(&format!("<link rel=\"canonical\" href=\"{URL}\">\n</head>")));
        let dup = HtmlAdapter.propose(&issue("SEO-CANON-01", &p.diff), &p.diff);
        assert!(matches!(dup, Err(RemediationError::NeedsReview { .. })));
    }

    #[test]
    fn adds_schema_context_to_typed_json_ld() {
        let html = "<head><script type=\"application/ld+json\">{\"@type\":\"Organization\",\"name\":\"A\"}</script></head>";
        let p = HtmlAdapter
            .propose(&issue("SEO-SCHEMA-02", html), html)
            .unwrap();
        assert!(p.diff.contains("\"@context\": \"https://schema.org\""));
        assert!(p.diff.contains("\"@type\": \"Organization\""));
        let untyped = "<head><script type=\"application/ld+json\">{\"name\":\"A\"}</script></head>";
        assert!(matches!(
            HtmlAdapter.propose(&issue("SEO-SCHEMA-02", untyped), untyped),
            Err(RemediationError::NeedsReview { reason, .. }) if reason.contains("@type")
        ));
    }

    #[test]
    fn content_rules_need_review_not_invented_text() {
        let html = "<html><head></head></html>";
        for rule in [
            "SEO-META-01",
            "SEO-META-03",
            "SEO-IMG-01",
            "GEO-05",
            "AEO-01",
            "SEO-SCHEMA-03",
        ] {
            assert!(
                matches!(HtmlAdapter.propose(&issue(rule, html), html), Err(RemediationError::NeedsReview { reason, .. }) if reason.contains("LLM")),
                "{rule}"
            );
        }
    }

    #[test]
    fn end_to_end_through_remediate_with_default_policy() {
        let html = "<html lang=\"fr\"><head><meta name=\"robots\" content=\"noindex\"><title>t</title></head></html>";
        let snapshot = PageSnapshot {
            url: URL.into(),
            lang: Some("fr".into()),
            title: Some("t".into()),
            meta_robots: vec!["noindex".into()],
            ..Default::default()
        };
        let findings = seo_findings(&seo_results(&snapshot), URL, &evidence());
        let issues: Vec<RemediationIssue> = findings
            .iter()
            .filter(|f| f.rule == "SEO-META-05" || f.rule == "SEO-CANON-01")
            .map(|f| issue_from_finding(f, html, location()))
            .collect();
        assert_eq!(issues.len(), 2);
        let outcomes = remediate(&issues, &RemediationPolicy::default(), &HtmlAdapter).unwrap();
        for o in &outcomes {
            let RemediationOutcome::Ok(g) = o else {
                panic!("expected proposal, got {o:?}")
            };
            assert!(g.proposal.requires_approval());
            assert!(g.proposal.ensure_approved().is_err());
        }
    }

    #[test]
    fn nap_findings_appear_only_with_profile() {
        let snapshot = PageSnapshot {
            url: URL.into(),
            body_text: "rien".into(),
            ..Default::default()
        };
        let profile = BusinessProfile {
            name: "Dupont".into(),
            phone: "0412345678".into(),
            address: "12 rue X".into(),
        };
        let with = SeoMapper::evaluate(&snapshot, Some(&profile));
        let findings = seo_findings(&with, URL, &evidence());
        assert!(findings
            .iter()
            .any(|f| f.rule == "SEO-NAP-01" && f.target == "body"));
    }
}
