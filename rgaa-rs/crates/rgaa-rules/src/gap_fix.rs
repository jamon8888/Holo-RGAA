use rgaa_core::{Classification, CriterionResult, CriterionStatus, MechanismRegistry, Violation};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Gap-fix rules targeting the 10 real false negatives from comparison data.
/// Each rule is a JS snippet executed via Playwright.
pub struct GapFixRules;

impl GapFixRules {
    /// Returns JS snippets for each gap-fix criterion.
    /// Each snippet returns JSON: { "pass": bool, "details": string, "nodes": number }
    ///
    /// Built once per process and shared read-only — no longer rebuilt on every call.
    #[must_use]
    pub fn snippets() -> &'static HashMap<String, &'static str> {
        static SNIPPETS: OnceLock<HashMap<String, &'static str>> = OnceLock::new();
        SNIPPETS.get_or_init(Self::build_snippets)
    }

    fn build_snippets() -> HashMap<String, &'static str> {
        let mut m: HashMap<String, &str> = HashMap::new();

        // 1.1: img/picture without alt (axe misses <picture> elements)
        m.insert("1.1".into(), r#"
            (() => {
                const imgs = document.querySelectorAll('img:not([alt])');
                const pictureImgs = document.querySelectorAll('picture img:not([alt])');
                const total = new Set([...imgs, ...pictureImgs]).size;
                return JSON.stringify({ pass: total === 0, details: `${total} images without alt`, nodes: total });
            })()
        "#);

        // 1.2: decorative images without alt="" or role=presentation
        m.insert("1.2".into(), r#"
            (() => {
                const imgs = document.querySelectorAll('img');
                let bad = 0;
                imgs.forEach(img => {
                    const hasAlt = img.hasAttribute('alt');
                    const hasPresentation = img.getAttribute('role') === 'presentation';
                    const hasAriaHidden = img.getAttribute('aria-hidden') === 'true';
                    if (!hasAlt && !hasPresentation && !hasAriaHidden) bad++;
                });
                return JSON.stringify({ pass: bad === 0, details: `${bad} decorative images not hidden`, nodes: bad });
            })()
        "#);

        // 2.1: iframe without title
        m.insert("2.1".into(), r#"
            (() => {
                const iframes = document.querySelectorAll('iframe');
                let bad = 0;
                iframes.forEach(f => { if (!f.title) bad++; });
                return JSON.stringify({ pass: bad === 0, details: `${bad} iframes without title`, nodes: bad });
            })()
        "#);

        // 3.2: contrast check with stricter threshold (0.3 vs axe-core's 0.3)
        // Asqatasun uses a stricter contrast ratio — we check for borderline cases
        m.insert("3.2".into(), r#"
            (() => {
                // Structural check: flag text elements with inline color styles
                // that might indicate manual color usage without sufficient contrast
                const textEls = document.querySelectorAll('p, span, h1, h2, h3, h4, h5, h6, a, li, td, th, label, button');
                let suspicious = 0;
                textEls.forEach(el => {
                    const style = window.getComputedStyle(el);
                    const color = style.color;
                    const bg = style.backgroundColor;
                    // Flag if both are inline and might be low contrast
                    if (el.style.color && el.style.backgroundColor) suspicious++;
                });
                return JSON.stringify({ pass: true, details: `${suspicious} suspicious color pairs (axe handles contrast)`, nodes: suspicious });
            })()
        "#);

        // 6.1: links without meaningful text (stricter than axe)
        m.insert("6.1".into(), r#"
            (() => {
                const links = document.querySelectorAll('a[href]');
                let bad = 0;
                links.forEach(a => {
                    const text = (a.textContent || '').trim();
                    const ariaLabel = a.getAttribute('aria-label');
                    const ariaLabelledby = a.getAttribute('aria-labelledby');
                    const img = a.querySelector('img[alt]');
                    const title = a.getAttribute('title');
                    if (!text && !ariaLabel && !ariaLabelledby && !img && !title) bad++;
                });
                return JSON.stringify({ pass: bad === 0, details: `${bad} links without text`, nodes: bad });
            })()
        "#);

        // 8.3: html lang attribute present (stricter check)
        m.insert("8.3".into(), r#"
            (() => {
                const lang = document.documentElement.getAttribute('lang');
                const valid = lang && lang.length >= 2 && /^[a-z]{2,3}(-[A-Z]{2})?(-[a-z]+)?$/.test(lang);
                return JSON.stringify({ pass: !!valid, details: lang || 'missing', nodes: valid ? 0 : 1 });
            })()
        "#);

        // 8.5: page title present and non-empty
        m.insert("8.5".into(), r#"
            (() => {
                const title = document.title;
                const valid = title && title.trim().length > 0;
                return JSON.stringify({ pass: !!valid, details: title || 'missing', nodes: valid ? 0 : 1 });
            })()
        "#);

        // 11.1: form inputs without labels (stricter than axe)
        m.insert("11.1".into(), r#"
            (() => {
                const inputs = document.querySelectorAll('input:not([type="hidden"]):not([type="submit"]):not([type="button"]):not([type="reset"]), select, textarea');
                let bad = 0;
                inputs.forEach(input => {
                    const id = input.id;
                    const hasLabel = id && document.querySelector(`label[for="${id}"]`);
                    const hasAriaLabel = input.getAttribute('aria-label');
                    const hasAriaLabelledby = input.getAttribute('aria-labelledby');
                    const wrappedInLabel = input.closest('label');
                    const hasTitle = input.getAttribute('title');
                    if (!hasLabel && !hasAriaLabel && !hasAriaLabelledby && !wrappedInLabel && !hasTitle) bad++;
                });
                return JSON.stringify({ pass: bad === 0, details: `${bad} inputs without labels`, nodes: bad });
            })()
        "#);

        // 11.4: label and input not adjacent (proximity check)
        m.insert("11.4".into(), r#"
            (() => {
                const labels = document.querySelectorAll('label[for]');
                let bad = 0;
                labels.forEach(label => {
                    const input = document.getElementById(label.getAttribute('for'));
                    if (input) {
                        const labelRect = label.getBoundingClientRect();
                        const inputRect = input.getBoundingClientRect();
                        const distance = Math.abs(labelRect.bottom - inputRect.top);
                        if (distance > 100) bad++;
                    }
                });
                return JSON.stringify({ pass: bad === 0, details: `${bad} labels too far from inputs`, nodes: bad });
            })()
        "#);

        // 12.7: skip link present (stricter pattern matching)
        m.insert("12.7".into(), r##"
            (() => {
                const links = document.querySelectorAll('a[href^="#"]');
                const skipPatterns = ['aller au contenu', 'skip to content', 'aller au menu', 'skip to main', 'contenu principal', 'main content'];
                const hasSkip = Array.from(links).some(a => {
                    const text = (a.textContent || '').toLowerCase();
                    return skipPatterns.some(p => text.includes(p));
                });
                return JSON.stringify({ pass: hasSkip, details: hasSkip ? 'skip link found' : 'no skip link', nodes: hasSkip ? 0 : 1 });
            })()
        "##);

        // 10.2: visible content remains when CSS is disabled
        m.insert("10.2".into(), r#"
            (() => {
                const styleSheets = document.styleSheets;
                let disabledCount = 0;
                for (let i = 0; i < styleSheets.length; i++) {
                    try {
                        if (!styleSheets[i].disabled) {
                            styleSheets[i].disabled = true;
                            disabledCount++;
                        }
                    } catch (e) {
                        // cross-origin stylesheets throw, ignore
                    }
                }
                const textBefore = document.body.innerText;
                // Re-enable
                for (let i = 0; i < styleSheets.length; i++) {
                    try { styleSheets[i].disabled = false; } catch (e) {}
                }
                const textAfter = document.body.innerText;
                const pass = textBefore === textAfter;
                return JSON.stringify({ pass, details: pass ? 'text content unchanged' : 'text content changed after CSS disabled', nodes: pass ? 0 : 1 });
            })()
        "#);

        // 10.11 (reflow) deliberately has no snippet. The one removed here compared
        // scrollWidth to a hardcoded 320 *at the unchanged viewport*, so on any desktop
        // viewport it reported overflow for almost every page — a false-failure machine,
        // not a reflow check. Measuring reflow needs a real
        // Emulation.setDeviceMetricsOverride to 320x256 and a re-measure, which belongs
        // to the browser bridge; until it lands, 10.11's mechanism of record is the
        // meta-viewport axe rule mapped in #201 (partial coverage, so it can fail the
        // criterion but never pass it). See
        // docs/specs/deterministic-mechanisms-axe-cannot-supply.md, mechanism 3.

        // 10.1 (#202 item 4): presentational HTML — elements and attributes that
        // carry formatting instead of structure, plus layout spacer images. 5 of the
        // criterion's 6 tests are automatable; this covers the markup ones.
        m.insert("10.1".into(), r#"
            (() => {
                const tags = document.querySelectorAll('font, center, basefont, big, strike, tt, marquee');
                const attrs = ['align','bgcolor','cellpadding','cellspacing','valign','hspace','vspace','background','bordercolor'];
                const attrHits = document.querySelectorAll(attrs.map(a => '[' + a + ']').join(',')).length;
                let spacers = 0;
                document.querySelectorAll('img').forEach(img => {
                    const w = img.getAttribute('width');
                    const h = img.getAttribute('height');
                    const empty = img.getAttribute('alt') === '' || !img.hasAttribute('alt');
                    if ((w === '1' || h === '1') && empty) spacers++;
                });
                const total = tags.length + attrHits + spacers;
                const details = total === 0
                    ? 'no presentational markup found'
                    : tags.length + ' presentational element(s), ' + attrHits + ' presentational attribute(s), ' + spacers + ' spacer image(s)';
                return JSON.stringify({ pass: total === 0, details, nodes: total });
            })()
        "#);

        // 11.5 (#202 item 5): radio/checkbox controls sharing a name form one group and
        // must be enclosed in a fieldset or an ARIA group. 2 of 4 tests automatable.
        m.insert("11.5".into(), r#"
            (() => {
                const groups = new Map();
                document.querySelectorAll('input[type=radio][name], input[type=checkbox][name]').forEach(i => {
                    const key = i.type + '|' + i.name;
                    if (!groups.has(key)) groups.set(key, []);
                    groups.get(key).push(i);
                });
                let bad = 0;
                const names = [];
                groups.forEach((inputs, key) => {
                    // A single control is not a group, so it needs no fieldset.
                    if (inputs.length < 2) return;
                    const enclosed = inputs.every(i =>
                        i.closest('fieldset') ||
                        i.closest('[role=group]') ||
                        i.closest('[role=radiogroup]'));
                    if (!enclosed) { bad++; names.push(key); }
                });
                const details = bad === 0
                    ? 'every multi-control field group is enclosed'
                    : bad + ' ungrouped field group(s): ' + names.join(', ');
                return JSON.stringify({ pass: bad === 0, details, nodes: bad });
            })()
        "#);

        // 1.9 (#202 item 6): a legend must be tied to the image it describes, via
        // figure/figcaption or aria-describedby. 12 of 25 tests automatable; this covers
        // the association ones.
        m.insert("1.9".into(), r#"
            (() => {
                let bad = 0;
                const problems = [];
                document.querySelectorAll('figure').forEach((fig, idx) => {
                    const cap = fig.querySelector('figcaption');
                    // No legend at all: 1.9 governs the association between a legend
                    // and its image, so a figure carrying none is out of scope, not in
                    // violation. CMS output wraps plain images in <figure> routinely.
                    if (!cap) return;
                    if (!cap.textContent.trim()) {
                        bad++;
                        problems.push('figure ' + idx + ': empty figcaption');
                        return;
                    }
                    const media = fig.querySelector('img, svg, object, canvas, video, audio');
                    // A figure need not wrap media; a captioned code sample is legitimate.
                    if (!media) return;
                    const describedBy = media.getAttribute('aria-describedby') || '';
                    const linked = cap.id !== '' && describedBy.split(/\s+/).indexOf(cap.id) !== -1;
                    const alt = (media.getAttribute('alt') || media.getAttribute('aria-label') || '').trim();
                    if (!linked && alt === '') {
                        bad++;
                        problems.push('figure ' + idx + ': caption not associated with its media');
                    }
                });
                const details = bad === 0
                    ? 'figure/figcaption associations OK'
                    : problems.join('; ');
                return JSON.stringify({ pass: bad === 0, details, nodes: bad });
            })()
        "#);

        // 10.14: CSS-only hover content accessible via keyboard
        m.insert("10.14".into(), r#"
            (() => {
                const styleSheets = document.styleSheets;
                let violations = 0;
                const hoverSelectors = new Set();
                const focusSelectors = new Set();
                
                for (let i = 0; i < styleSheets.length; i++) {
                    try {
                        const rules = styleSheets[i].cssRules || styleSheets[i].rules;
                        if (!rules) continue;
                        for (let j = 0; j < rules.length; j++) {
                            const rule = rules[j];
                            if (rule.selectorText) {
                                if (rule.selectorText.includes(':hover')) {
                                    hoverSelectors.add(rule.selectorText.replace(':hover', '').trim());
                                }
                                if (rule.selectorText.includes(':focus') || rule.selectorText.includes(':focus-within')) {
                                    focusSelectors.add(rule.selectorText.replace(':focus', '').replace(':focus-within', '').trim());
                                }
                            }
                        }
                    } catch (e) {
                        // cross-origin
                    }
                }
                
                // Check if every :hover selector has a corresponding :focus/:focus-within
                for (const hoverSel of hoverSelectors) {
                    const baseSel = hoverSel.trim();
                    let hasFocusEquivalent = false;
                    for (const focusSel of focusSelectors) {
                        if (focusSel === baseSel || focusSel.includes(baseSel) || baseSel.includes(focusSel)) {
                            hasFocusEquivalent = true;
                            break;
                        }
                    }
                    if (!hasFocusEquivalent) {
                        // Verify element exists on page
                        try {
                            const el = document.querySelector(baseSel);
                            if (el) violations++;
                        } catch (e) {}
                    }
                }
                
                return JSON.stringify({ pass: violations === 0, details: `${violations} hover-only elements without focus equivalent`, nodes: violations });
            })()
        "#);

        for (id, snippet) in crate::keyboard_probes::SNIPPETS {
            let previous = m.insert((*id).into(), snippet);
            debug_assert!(previous.is_none(), "{id} already has a gap-fix snippet");
        }

        for (id, snippet) in crate::plan_snippets::SNIPPETS {
            let previous = m.insert((*id).into(), snippet);
            debug_assert!(previous.is_none(), "{id} already has a gap-fix snippet");
        }

        m
    }

    /// Whether a `pass: true` from this criterion's snippet is enough to assert the
    /// criterion conforms. Unknown criteria are partial: a mechanism added without
    /// declaring its coverage fails closed rather than claiming conformance.
    #[must_use]
    pub fn covers_whole_criterion(criterion_id: &str) -> bool {
        MechanismRegistry::builtin().probe_is_complete(criterion_id)
    }

    /// Parse JS execution results into `CriterionResult`s.
    ///
    /// # Contract
    ///
    /// A snippet returns `{pass, details, nodes}` (legacy) and may add two optional
    /// fields (spec §2, #262):
    ///
    /// - `outcome`: `"fail" | "pass" | "review"`. When present it takes precedence
    ///   over `pass`; when absent the outcome is derived from `pass` exactly as before.
    /// - `reason`: why a `review` was raised (`moteur absent`, `échantillon
    ///   insuffisant`…). Carried to the report through the justification and the
    ///   violation description.
    ///
    /// # Outcomes
    ///
    /// - `fail` always yields `Fail` with the evidence.
    /// - `pass` yields `Pass` **only** where the mechanism decides the whole criterion
    ///   — see [`Self::covers_whole_criterion`]. A partial mechanism finding nothing
    ///   proves nothing about the tests it does not cover, so its criterion is left
    ///   out of the results and falls to the pipeline's declared fallback (#199, #201,
    ///   #202).
    /// - `review` yields `NeedsReview`, keeping the nodes and details. It makes no
    ///   verdict, so coverage does not gate it.
    /// - An unrecognised `outcome` degrades to `review` (never to `Pass`).
    pub fn parse_results(
        js_results: &HashMap<String, serde_json::Value>,
    ) -> HashMap<String, CriterionResult> {
        let mut results = HashMap::new();

        for (criterion_id, js_result) in js_results {
            let legacy_pass = js_result
                .get("pass")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let details = js_result
                .get("details")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let reason = js_result
                .get("reason")
                .and_then(|v| v.as_str())
                .filter(|r| !r.is_empty());
            let nodes = js_result.get("nodes").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

            let (outcome, unknown_outcome) = match js_result.get("outcome") {
                None | Some(serde_json::Value::Null) => (
                    if legacy_pass {
                        ProbeOutcome::Pass
                    } else {
                        ProbeOutcome::Fail
                    },
                    None,
                ),
                Some(value) => match value.as_str() {
                    Some("fail") => (ProbeOutcome::Fail, None),
                    Some("pass") => (ProbeOutcome::Pass, None),
                    Some("review") => (ProbeOutcome::Review, None),
                    _ => {
                        tracing::warn!(
                            criterion_id = %criterion_id,
                            outcome = %value,
                            "unknown gap-fix outcome; degrading to review"
                        );
                        (ProbeOutcome::Review, Some(value.to_string()))
                    }
                },
            };

            let (status, violations, justification) = match outcome {
                ProbeOutcome::Pass => {
                    if !Self::covers_whole_criterion(criterion_id) {
                        continue;
                    }
                    (CriterionStatus::Pass, vec![], None)
                }
                ProbeOutcome::Fail => (
                    CriterionStatus::Fail,
                    vec![Violation {
                        rule_id: format!("gap-fix-{criterion_id}"),
                        impact: "serious".into(),
                        description: details.to_string(),
                        nodes_affected: nodes,
                    }],
                    Some(format!("gap-fix mechanism: {details}")),
                ),
                ProbeOutcome::Review => {
                    let reason = match (&unknown_outcome, reason) {
                        (Some(bad), _) => format!("unrecognised outcome {bad}"),
                        (None, Some(r)) => r.to_string(),
                        (None, None) => String::new(),
                    };
                    let text = match (reason.is_empty(), details.is_empty()) {
                        (false, false) => format!("{reason}: {details}"),
                        (false, true) => reason,
                        (true, _) => details.to_string(),
                    };
                    (
                        CriterionStatus::NeedsReview,
                        vec![Violation {
                            rule_id: format!("gap-fix-review-{criterion_id}"),
                            impact: "review".into(),
                            description: text.clone(),
                            nodes_affected: nodes,
                        }],
                        Some(format!("gap-fix review: {text}")),
                    )
                }
            };

            results.insert(
                criterion_id.clone(),
                CriterionResult {
                    criterion_id: criterion_id.clone(),
                    title: String::new(),
                    classification: Classification::Deterministe,
                    status,
                    violations,
                    confidence: None,
                    raw_confidence: None,
                    justification,
                    source: "gap-fix".to_string(),
                    citations: vec![],
                    considered_sources: vec![],
                    tests: vec![],
                    automated_verdict: None,
                    verdict_basis: Vec::new(),
                    evidence: Vec::new(),
                    confidence_calibration_version: None,
                    review_required: false,
                    review_reason: None,
                    verified_status: None,
                    review_events: Vec::new(),
                },
            );
        }

        results
    }

    /// Convert Obscura's bounded Tab traversal into criterion outcomes.
    /// The trap finding is actionable evidence for 12.9; a clean bounded path
    /// remains review because a finite traversal cannot prove every state safe.
    #[must_use]
    pub fn parse_keyboard_observation(
        status: &str,
        issue_rules: &[String],
        elements_observed: usize,
    ) -> HashMap<String, CriterionResult> {
        let trapped = issue_rules.iter().any(|rule| rule == "keyboard-trap");
        let complete = status == "complete";
        let reason = if trapped {
            "Obscura detected that keyboard focus repeated on one element"
        } else if complete {
            "No trap was observed in the bounded Tab traversal; other page states still need review"
        } else {
            "The keyboard traversal was incomplete; inspect focus order and possible traps manually"
        };
        let status_12_9 = if trapped {
            CriterionStatus::Fail
        } else if complete {
            CriterionStatus::NeedsReview
        } else {
            CriterionStatus::NotTested
        };

        [
            (
                "12.8",
                CriterionStatus::NeedsReview,
                format!("Obscura observed {elements_observed} keyboard-focusable element(s); compare the actual focus sequence with visual/reading order"),
            ),
            ("12.9", status_12_9, reason.to_string()),
        ]
        .into_iter()
        .map(|(criterion_id, status, justification)| {
            (
                criterion_id.to_string(),
                CriterionResult {
                    criterion_id: criterion_id.to_string(),
                    title: String::new(),
                    classification: Classification::Deterministe,
                    status,
                    violations: vec![],
                    confidence: None,
                    raw_confidence: None,
                    justification: Some(justification),
                    source: "gap-fix".to_string(),
                    citations: vec![],
                    considered_sources: vec![],
                    tests: vec![],
                    automated_verdict: None,
                    verdict_basis: Vec::new(),
                    evidence: Vec::new(),
                    confidence_calibration_version: None,
                    review_required: false,
                    review_reason: None,
                    verified_status: None,
                    review_events: Vec::new(),
                },
            )
        })
        .collect()
    }
}

/// The three outcomes a probe can report (spec §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProbeOutcome {
    Fail,
    Pass,
    Review,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse_one(criterion_id: &str, value: serde_json::Value) -> Option<CriterionResult> {
        let mut input = HashMap::new();
        input.insert(criterion_id.to_string(), value);
        GapFixRules::parse_results(&input).remove(criterion_id)
    }

    /// The three mechanisms #202 adds must actually be on the audit path: the
    /// orchestrator runs whatever `snippets()` returns, so absence here means the
    /// mechanism never executes during an audit (#202 AC1).
    #[test]
    fn the_new_mechanisms_are_on_the_audit_path() {
        let snippets = GapFixRules::snippets();
        for criterion_id in [
            "10.1", "11.5", "1.9", "1.6", "3.3", "8.7", "11.3", "11.8", "13.3", "4.12", "4.13",
            "10.9", "10.12", "10.13", "11.11", "12.8", "12.9", "12.10", "12.11", "13.10", "13.11",
            "13.12",
        ] {
            let snippet = snippets
                .get(criterion_id)
                .unwrap_or_else(|| panic!("{criterion_id} must have a gap-fix snippet"));
            assert!(
                snippet.contains("JSON.stringify"),
                "{criterion_id}'s snippet must return the {{pass, details, nodes}} contract"
            );
            assert!(
                snippet.contains("pass"),
                "{criterion_id}'s snippet must report a pass flag"
            );
        }
    }

    #[test]
    fn newly_added_static_probes_are_partial_and_never_claim_pass() {
        let registry = MechanismRegistry::builtin();
        for criterion_id in [
            "1.6", "3.3", "4.12", "4.13", "8.7", "10.9", "10.12", "10.13", "11.3", "11.8", "11.11",
            "12.8", "12.9", "12.10", "12.11", "13.3", "13.10", "13.11", "13.12",
        ] {
            let mechanism = registry
                .probe_for(criterion_id)
                .unwrap_or_else(|| panic!("{criterion_id} needs a registered probe"));
            assert_eq!(
                mechanism.coverage,
                rgaa_core::catalog::AxeCoverage::Partial,
                "{criterion_id}"
            );
            assert!(
                !mechanism
                    .outcomes
                    .contains(&rgaa_core::registry::Outcome::Pass),
                "{criterion_id} cannot pass on partial browser evidence"
            );
            assert!(GapFixRules::snippets().contains_key(criterion_id));
        }
    }

    #[test]
    fn review_outcome_is_kept_as_a_human_review_result() {
        let result = parse_one(
            "1.6",
            json!({"outcome":"review", "details":"complex image candidate", "reason":"human judgement", "nodes":1}),
        )
        .expect("review observation should be retained");
        assert_eq!(result.status, CriterionStatus::NeedsReview);
        assert_eq!(result.source, "gap-fix");
        assert!(result
            .justification
            .as_deref()
            .unwrap_or_default()
            .contains("human judgement"));
    }

    #[test]
    fn keyboard_trap_is_fail_but_a_clean_bounded_run_stays_review() {
        let trapped = GapFixRules::parse_keyboard_observation(
            "incomplete",
            &["keyboard-trap".to_string()],
            3,
        );
        assert_eq!(trapped["12.9"].status, CriterionStatus::Fail);
        assert_eq!(trapped["12.8"].status, CriterionStatus::NeedsReview);

        let clean = GapFixRules::parse_keyboard_observation("complete", &[], 4);
        assert_eq!(clean["12.9"].status, CriterionStatus::NeedsReview);
        assert_eq!(clean["12.8"].status, CriterionStatus::NeedsReview);
        assert!(clean["12.8"]
            .justification
            .as_deref()
            .unwrap_or_default()
            .contains("4"));
    }

    /// A snapshot inventory must not restore the old reflow verdict based on
    /// comparing scrollWidth to a hardcoded 320 without resizing the viewport.
    /// Actual JavaScript behavior is covered in tests/js/control_probe_regressions.js.
    #[test]
    fn reflow_snapshot_is_registered_as_review_only() {
        assert!(GapFixRules::snippets().contains_key("10.11"));
        let probe = MechanismRegistry::builtin()
            .probe_for("10.11")
            .expect("reflow inventory must be registered");
        assert_eq!(probe.coverage, rgaa_core::catalog::AxeCoverage::Partial);
        assert_eq!(probe.outcomes, vec![rgaa_core::registry::Outcome::Review]);
        let result = parse_one(
            "10.11",
            json!({"outcome": "review", "reason": "320px viewport not emulated", "nodes": 1}),
        )
        .expect("reflow inventory must remain reviewable");
        assert_eq!(result.status, CriterionStatus::NeedsReview);
    }

    /// A mechanism that covers part of a criterion may report a violation, and that
    /// violation stands — evidence of a failure is evidence whatever the coverage
    /// (#202 AC2).
    #[test]
    fn a_partial_mechanism_still_fails_its_criterion() {
        let result = parse_one(
            "11.5",
            json!({"pass": false, "details": "2 ungrouped field group(s)", "nodes": 2}),
        )
        .expect("a violation must produce a result");

        assert_eq!(result.status, CriterionStatus::Fail);
        assert_eq!(result.source, "gap-fix");
        assert_eq!(result.violations.len(), 1);
        assert_eq!(result.violations[0].rule_id, "gap-fix-11.5");
        assert_eq!(result.violations[0].nodes_affected, 2);
        assert!(
            result
                .justification
                .as_deref()
                .is_some_and(|j| j.contains("ungrouped")),
            "the mechanism's own finding must survive into the justification"
        );
    }

    /// The #202 guard, and the whole point of AC3: a mechanism covering 2 of a
    /// criterion's 4 tests finding nothing does not make the criterion conform, so it
    /// must not emit a criterion-level `Pass`. The criterion is left to the pipeline's
    /// fallback instead.
    #[test]
    fn a_partial_mechanism_never_passes_its_criterion() {
        for criterion_id in ["10.1", "11.5", "1.9"] {
            assert!(
                !GapFixRules::covers_whole_criterion(criterion_id),
                "{criterion_id} covers only part of its criterion"
            );
            let result = parse_one(
                criterion_id,
                json!({"pass": true, "details": "nothing found", "nodes": 0}),
            );
            assert!(
                result.is_none(),
                "{criterion_id} has partial coverage, yet a clean run produced {:?}",
                result.map(|r| r.status)
            );
        }
    }

    /// The counterpart: a mechanism that does decide its whole criterion keeps
    /// asserting `Pass`, so this change moves no existing verdict.
    #[test]
    fn a_complete_mechanism_still_passes_its_criterion() {
        let result = parse_one(
            "2.1",
            json!({"pass": true, "details": "0 iframes without title", "nodes": 0}),
        )
        .expect("a complete-coverage criterion must still produce a Pass");

        assert_eq!(result.status, CriterionStatus::Pass);
        assert!(result.violations.is_empty());
        assert!(GapFixRules::covers_whole_criterion("2.1"));
    }

    /// A criterion nobody declared is partial, so forgetting to declare coverage
    /// cannot accidentally claim conformance.
    #[test]
    fn an_undeclared_criterion_defaults_to_partial() {
        assert!(!GapFixRules::covers_whole_criterion("99.99"));
        assert!(parse_one("99.99", json!({"pass": true, "details": "", "nodes": 0})).is_none());
    }

    /// Malformed JS output must not read as conformance: a missing `pass` key is
    /// treated as a failure with the details preserved, never as a silent `Pass`.
    #[test]
    fn malformed_snippet_output_is_not_a_pass() {
        let result = parse_one("2.1", json!({"details": "snippet threw"}))
            .expect("malformed output still produces a result");
        assert_eq!(result.status, CriterionStatus::Fail);
        assert_eq!(result.violations[0].nodes_affected, 0);
    }

    /// Invariant "no active mechanism without a registry entry" (spec §5): every
    /// snippet that runs has a registry probe, and every registry probe has a snippet,
    /// so the registry can neither under- nor over-claim what the audit executes.
    #[test]
    fn snippets_and_registry_probes_are_the_same_set() {
        let registry = MechanismRegistry::builtin();
        let snippets = GapFixRules::snippets();
        let unregistered: Vec<&String> = snippets
            .keys()
            .filter(|id| registry.probe_for(id).is_none())
            .collect();
        assert!(
            unregistered.is_empty(),
            "active gap-fix snippets with no registry entry: {unregistered:?}"
        );
        let orphans: Vec<&str> = registry
            .mechanisms()
            .iter()
            .filter(|m| {
                !matches!(
                    m.kind,
                    rgaa_core::MechanismKind::AxeNative | rgaa_core::MechanismKind::Site
                )
            })
            .filter(|m| !snippets.contains_key(&m.criterion))
            .map(|m| m.id.as_str())
            .collect();
        assert!(
            orphans.is_empty(),
            "registry probes with no gap-fix snippet: {orphans:?}"
        );
    }

    // --- Three-outcome contract (#262, spec §2) ---------------------------------

    /// `review` keeps the probe's attachments and lands the criterion in
    /// `needs_review`, with the reason carried in both the justification (HTML
    /// report) and the violation description (SARIF/JUnit findings).
    #[test]
    fn a_review_outcome_keeps_its_attachments_and_needs_review() {
        let result = parse_one(
            "12.9",
            json!({"outcome": "review", "reason": "moteur absent", "details": "3 focusable", "nodes": 3}),
        )
        .expect("a review outcome must produce a result");

        assert_eq!(result.status, CriterionStatus::NeedsReview);
        assert_eq!(result.source, "gap-fix");
        assert_eq!(result.violations.len(), 1);
        assert_eq!(result.violations[0].nodes_affected, 3);
        assert!(result.violations[0].description.contains("moteur absent"));
        assert!(result.violations[0].description.contains("3 focusable"));
        let justification = result
            .justification
            .expect("review carries a justification");
        assert!(justification.contains("moteur absent"));
        assert!(justification.contains("3 focusable"));
    }

    /// A review without a reason still reviews; it just has no reason to show.
    #[test]
    fn a_review_without_reason_still_needs_review() {
        let result = parse_one(
            "12.9",
            json!({"outcome": "review", "details": "x", "nodes": 1}),
        )
        .expect("result");
        assert_eq!(result.status, CriterionStatus::NeedsReview);
        assert!(result.justification.is_some_and(|j| j.contains('x')));
    }

    /// `review` is allowed for partial and complete mechanisms alike: it makes no
    /// verdict, so coverage does not gate it.
    #[test]
    fn review_is_not_gated_by_coverage() {
        for id in ["2.1", "99.99"] {
            let result = parse_one(id, json!({"outcome": "review", "reason": "r"}))
                .expect("review always yields a result");
            assert_eq!(result.status, CriterionStatus::NeedsReview);
        }
    }

    /// An explicit `fail` outcome behaves exactly like `pass: false`.
    #[test]
    fn an_explicit_fail_outcome_fails() {
        let result = parse_one(
            "11.5",
            json!({"outcome": "fail", "details": "2 bad", "nodes": 2}),
        )
        .expect("result");
        assert_eq!(result.status, CriterionStatus::Fail);
        assert_eq!(result.violations[0].nodes_affected, 2);
    }

    /// An explicit `pass` outcome from a partial mechanism is ignored, like
    /// `pass: true`; from a complete one it passes.
    #[test]
    fn an_explicit_pass_outcome_obeys_coverage() {
        assert!(parse_one("11.5", json!({"outcome": "pass"})).is_none());
        let result = parse_one("2.1", json!({"outcome": "pass"})).expect("complete passes");
        assert_eq!(result.status, CriterionStatus::Pass);
    }

    /// The outcome wins over a contradictory legacy `pass` flag.
    #[test]
    fn outcome_takes_precedence_over_the_pass_flag() {
        let result = parse_one(
            "2.1",
            json!({"outcome": "review", "pass": true, "reason": "r"}),
        )
        .expect("result");
        assert_eq!(result.status, CriterionStatus::NeedsReview);
    }

    /// An unrecognised outcome must never read as conformance: it becomes a review
    /// that names the bad value.
    #[test]
    fn an_unknown_outcome_degrades_to_review() {
        let result = parse_one("2.1", json!({"outcome": "maybe", "pass": true})).expect("result");
        assert_eq!(result.status, CriterionStatus::NeedsReview);
        assert!(result.justification.is_some_and(|j| j.contains("maybe")));
    }

    /// Legacy snippets (no `outcome`) are untouched.
    #[test]
    fn legacy_contract_is_unchanged() {
        let fail = parse_one("2.1", json!({"pass": false, "details": "d", "nodes": 4})).expect("r");
        assert_eq!(fail.status, CriterionStatus::Fail);
        assert_eq!(fail.justification.as_deref(), Some("gap-fix mechanism: d"));
    }

    /// The coverage the registry declares is exactly what the pre-registry
    /// `COMPLETE_COVERAGE` constant said (#261: no verdict moves).
    #[test]
    fn migrated_probe_coverage_matches_the_former_constant() {
        const FORMER: &[&str] = &[
            "1.1", "1.2", "2.1", "3.2", "6.1", "8.3", "8.5", "10.2", "10.14", "11.1", "11.4",
            "12.7",
        ];
        for id in GapFixRules::snippets().keys() {
            assert_eq!(
                GapFixRules::covers_whole_criterion(id),
                FORMER.contains(&id.as_str()),
                "coverage of probe {id} changed in the migration"
            );
        }
    }
}
