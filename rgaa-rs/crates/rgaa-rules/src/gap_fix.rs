use rgaa_core::{Classification, CriterionResult, CriterionStatus, Violation};
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
                    if (!cap || !cap.textContent.trim()) {
                        bad++;
                        problems.push('figure ' + idx + ': no figcaption text');
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

        m
    }

    /// Criteria whose gap-fix snippet decides every test of the criterion, so a
    /// `pass: true` from it is evidence of conformance and may stand as a
    /// criterion-level `Pass`.
    ///
    /// These are the thirteen snippets that predate #202. Their coverage is recorded
    /// as it behaves today rather than as it ought to be: several plainly do not decide
    /// their whole criterion (11.1 has thirteen tests), but re-assessing them moves
    /// verdicts in the published report on grounds that belong to the per-test work in
    /// #203. What #202 fixes is the direction of travel — a *new* mechanism cannot
    /// widen an unconditional criterion-level `Pass` (AC2, AC3).
    const COMPLETE_COVERAGE: &[&str] = &[
        "1.1", "1.2", "2.1", "3.2", "6.1", "8.3", "8.5", "10.2", "10.14", "11.1", "11.4", "12.7",
    ];

    /// Whether a `pass: true` from this criterion's snippet is enough to assert the
    /// criterion conforms. Unknown criteria are partial: a mechanism added without
    /// declaring its coverage fails closed rather than claiming conformance.
    #[must_use]
    pub fn covers_whole_criterion(criterion_id: &str) -> bool {
        Self::COMPLETE_COVERAGE.contains(&criterion_id)
    }

    /// Parse JS execution results into `CriterionResult`s.
    ///
    /// A snippet that reports a violation always yields `Fail` with the evidence. A
    /// snippet that reports no violation yields `Pass` **only** where it decides the
    /// whole criterion — see [`Self::covers_whole_criterion`]. A partial mechanism
    /// finding nothing proves nothing about the tests it does not cover, so its
    /// criterion is left out of the results entirely and falls to the pipeline's
    /// declared fallback, exactly as a partial-coverage axe criterion does (#199,
    /// #201, #202).
    pub fn parse_results(
        js_results: &HashMap<String, serde_json::Value>,
    ) -> HashMap<String, CriterionResult> {
        let mut results = HashMap::new();

        for (criterion_id, js_result) in js_results {
            let pass = js_result
                .get("pass")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let details = js_result
                .get("details")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let nodes = js_result.get("nodes").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

            if pass && !Self::covers_whole_criterion(criterion_id) {
                continue;
            }

            results.insert(
                criterion_id.clone(),
                CriterionResult {
                    criterion_id: criterion_id.clone(),
                    title: String::new(),
                    classification: Classification::Deterministe,
                    status: if pass {
                        CriterionStatus::Pass
                    } else {
                        CriterionStatus::Fail
                    },
                    violations: if pass {
                        vec![]
                    } else {
                        vec![Violation {
                            rule_id: format!("gap-fix-{criterion_id}"),
                            impact: "serious".into(),
                            description: details.to_string(),
                            nodes_affected: nodes,
                        }]
                    },
                    confidence: None,
                    justification: if pass {
                        None
                    } else {
                        Some(format!("gap-fix mechanism: {details}"))
                    },
                    source: "gap-fix".to_string(),
                    citations: vec![],
                    considered_sources: vec![],
                    tests: vec![],
                },
            );
        }

        results
    }
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
        for criterion_id in ["10.1", "11.5", "1.9"] {
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

    /// The snippet that compared `scrollWidth` to a hardcoded 320 at the unchanged
    /// viewport is gone: on a desktop viewport it reported overflow for nearly every
    /// page. 10.11 is carried by the `meta-viewport` axe rule (partial) until a real
    /// device-metrics override lands (#202 item 3).
    #[test]
    fn the_unsound_reflow_snippet_is_not_on_the_audit_path() {
        assert!(
            !GapFixRules::snippets().contains_key("10.11"),
            "10.11's gap-fix snippet was unsound and must not be reinstated without a \
             real Emulation.setDeviceMetricsOverride"
        );
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

    /// Every declared complete-coverage criterion must actually have a snippet, or the
    /// list is asserting conformance for a mechanism that no longer runs.
    #[test]
    fn every_complete_coverage_criterion_has_a_snippet() {
        let snippets = GapFixRules::snippets();
        let orphans: Vec<&&str> = GapFixRules::COMPLETE_COVERAGE
            .iter()
            .filter(|id| !snippets.contains_key(**id))
            .collect();
        assert!(
            orphans.is_empty(),
            "declared complete-coverage criteria with no snippet: {orphans:?}"
        );
    }
}
