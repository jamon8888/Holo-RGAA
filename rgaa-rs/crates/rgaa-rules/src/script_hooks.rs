//! Hook-based probes installed **before** the page's own scripts (spec
//! `criteria-coverage-expansion` §7.2). They need `Page.addScriptToEvaluateOnNewDocument`,
//! verified on Obscura 0.2.4, and are driven by `ObscuraBridge::observe_with_hook`.
//!
//! 13.1 (issue #265): redirects and refreshes by script. The hook records every
//! `setTimeout`/`setInterval` (delay and callback source), wraps `location.assign` and
//! `location.replace`, and flags real user interaction. Obscura exposes no global
//! `Location` and `href` cannot be intercepted, and a script navigation discards the
//! hooked document, so the verdict compares two snapshots: the page URL once loaded and
//! the URL after ~6 s **without any interaction**. A page that moved on its own is the
//! `fail`; the timer recorded in the first snapshot is the trace (call, delay).
//!
//! Partial mechanism: it can `fail` or ask for `review`, never `pass`. Carousel timers are
//! not redirections and stay silent.

use std::collections::HashMap;

use rgaa_core::{Classification, CriterionResult, CriterionStatus};

/// How long the page is observed without interaction.
pub const REDIRECT_OBSERVATION_MS: u64 = 6_000;

/// Installed before any page script.
pub const REDIRECT_HOOK: &str = r#"
(() => {
  const log = [];
  const state = { log, interacted: false };
  window.__rgaa = state;
  for (const t of ['click', 'keydown', 'pointerdown', 'touchstart', 'mousedown']) {
    try { document.addEventListener(t, () => { state.interacted = true; }, true); } catch (e) {}
  }
  const rec = (kind, detail) => log.push(Object.assign({ kind, t: Date.now() }, detail));
  const src = fn => typeof fn === 'function'
    ? Function.prototype.toString.call(fn).slice(0, 300)
    : String(fn).slice(0, 300);
  for (const name of ['assign', 'replace']) {
    try {
      const orig = window.location[name];
      window.location[name] = function (...a) { rec('location.' + name, { url: String(a[0]) }); return orig.apply(this, a); };
    } catch (e) {}
  }
  const wt = window.setTimeout, wi = window.setInterval;
  window.setTimeout = function (fn, ms, ...rest) { rec('setTimeout', { ms: Number(ms) || 0, src: src(fn) }); return wt.call(this, fn, ms, ...rest); };
  window.setInterval = function (fn, ms, ...rest) { rec('setInterval', { ms: Number(ms) || 0, src: src(fn) }); return wi.call(this, fn, ms, ...rest); };
})();
"#;

/// Read twice by the bridge: once at load, once after the observation window.
pub const REDIRECT_READ: &str = r#"
JSON.stringify({
  href: location.href,
  interacted: !!(window.__rgaa && window.__rgaa.interacted),
  log: (window.__rgaa && window.__rgaa.log) || []
})
"#;

fn without_fragment(url: &str) -> &str {
    url.split('#').next().unwrap_or(url)
}

fn looks_like_navigation(source: &str) -> bool {
    ["location", "href", ".assign(", ".replace(", "window.open"]
        .iter()
        .any(|needle| source.contains(needle))
}

/// Outcome of the redirect probe for 13.1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedirectFinding {
    pub status: CriterionStatus,
    pub details: String,
}

/// Turn the bridge's `{"at_load": .., "after": ..}` into a 13.1 finding.
/// `None` when the page neither moved nor registered a navigation-looking timer: the
/// probe stays silent and the criterion keeps the pipeline's fallback.
#[must_use]
pub fn parse_redirect_observation(observation: &serde_json::Value) -> Option<RedirectFinding> {
    let at_load = observation.get("at_load")?;
    let after = observation.get("after")?;
    let href_at_load = at_load.get("href")?.as_str()?;
    let href_after = after.get("href")?.as_str()?;
    let interacted = after
        .get("interacted")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);

    // Timers registered during load whose callback mentions a navigation.
    let suspect: Vec<String> = at_load
        .get("log")
        .and_then(serde_json::Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    let kind = entry.get("kind")?.as_str()?;
                    let source = entry.get("src").and_then(serde_json::Value::as_str)?;
                    if !matches!(kind, "setTimeout" | "setInterval")
                        || !looks_like_navigation(source)
                    {
                        return None;
                    }
                    let delay = entry
                        .get("ms")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0);
                    Some(format!("{kind}({delay} ms)"))
                })
                .collect()
        })
        .unwrap_or_default();
    let direct: Vec<String> = at_load
        .get("log")
        .and_then(serde_json::Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    let kind = entry.get("kind")?.as_str()?;
                    kind.starts_with("location.").then(|| {
                        let url = entry
                            .get("url")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("?");
                        format!("{kind}({url}) pendant le chargement")
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let moved = without_fragment(href_at_load) != without_fragment(href_after);
    if moved && !interacted {
        let trace = direct
            .iter()
            .chain(suspect.iter())
            .cloned()
            .collect::<Vec<_>>();
        let trace = if trace.is_empty() {
            "appelant non identifié".to_string()
        } else {
            trace.join(", ")
        };
        return Some(RedirectFinding {
            status: CriterionStatus::Fail,
            details: format!(
                "navigation sans action de l'utilisateur: {href_at_load} → {href_after} en moins de {} s ({trace})",
                REDIRECT_OBSERVATION_MS / 1000
            ),
        });
    }
    if !direct.is_empty() || !suspect.is_empty() {
        let trace = direct
            .iter()
            .chain(suspect.iter())
            .cloned()
            .collect::<Vec<_>>();
        return Some(RedirectFinding {
            status: CriterionStatus::NeedsReview,
            details: format!(
                "minuteur ou appel de navigation suspect sans redirection observée dans la fenêtre de {} s: {}",
                REDIRECT_OBSERVATION_MS / 1000,
                trace.join(", ")
            ),
        });
    }
    None
}

/// The 13.1 result in the shape the pipeline merges (same as the other gap-fix results).
#[must_use]
pub fn redirect_results(finding: Option<&RedirectFinding>) -> HashMap<String, CriterionResult> {
    let Some(finding) = finding else {
        return HashMap::new();
    };
    let result = CriterionResult {
        criterion_id: "13.1".to_string(),
        title: String::new(),
        classification: Classification::Deterministe,
        status: finding.status.clone(),
        violations: vec![],
        confidence: None,
        raw_confidence: None,
        justification: Some(finding.details.clone()),
        source: "script-hook".to_string(),
        citations: vec![],
        considered_sources: vec![],
        tests: vec![],
        automated_verdict: None,
        verdict_basis: Vec::new(),
        evidence: Vec::new(),
        confidence_calibration_version: None,
        review_required: finding.status == CriterionStatus::NeedsReview,
        review_reason: None,
        verified_status: None,
        review_events: Vec::new(),
    };
    HashMap::from([("13.1".to_string(), result)])
}

/// 13.2 (issue #266): windows opened without any user action. `window.open` is wrapped
/// before the page's scripts run (Obscura 0.2.4 lets a hook replace it); every call is
/// recorded with whether a real interaction had happened. `target="_blank"` on a link is
/// not an automatic opening and is never reported: only script calls are seen here.
pub const POPUP_HOOK: &str = r#"
(() => {
  const log = [];
  const state = { log, interacted: false };
  window.__rgaa = state;
  for (const t of ['click', 'keydown', 'pointerdown', 'touchstart', 'mousedown']) {
    try { document.addEventListener(t, () => { state.interacted = true; }, true); } catch (e) {}
  }
  const rec = (kind, detail) => log.push(Object.assign({ kind, interacted: state.interacted, t: Date.now() }, detail));
  const src = fn => typeof fn === 'function'
    ? Function.prototype.toString.call(fn).slice(0, 300)
    : String(fn).slice(0, 300);
  try {
    window.open = function (url, ...rest) { rec('window.open', { url: String(url) }); return null; };
  } catch (e) {}
  const wt = window.setTimeout;
  window.setTimeout = function (fn, ms, ...rest) { rec('setTimeout', { ms: Number(ms) || 0, src: src(fn) }); return wt.call(this, fn, ms, ...rest); };
})();
"#;

/// Outcome of the popup probe for 13.2 (same shape as [`RedirectFinding`]).
#[must_use]
pub fn parse_popup_observation(observation: &serde_json::Value) -> Option<RedirectFinding> {
    let after = observation.get("after")?;
    let entries = after.get("log")?.as_array()?;
    let opened: Vec<String> = entries
        .iter()
        .filter(|e| e.get("kind").and_then(serde_json::Value::as_str) == Some("window.open"))
        .filter(|e| e.get("interacted").and_then(serde_json::Value::as_bool) == Some(false))
        .map(|e| {
            format!(
                "window.open({})",
                e.get("url")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("?")
            )
        })
        .collect();
    if !opened.is_empty() {
        return Some(RedirectFinding {
            status: CriterionStatus::Fail,
            details: format!(
                "fenêtre ouverte sans action de l'utilisateur en moins de {} s: {}",
                REDIRECT_OBSERVATION_MS / 1000,
                opened.join(", ")
            ),
        });
    }
    // A timer that mentions window.open but has not fired in the window: deferred opening.
    let deferred: Vec<String> = entries
        .iter()
        .filter(|e| e.get("kind").and_then(serde_json::Value::as_str) == Some("setTimeout"))
        .filter(|e| {
            e.get("src")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|s| s.contains("window.open") || s.contains(".open("))
        })
        .map(|e| {
            format!(
                "setTimeout({} ms)",
                e.get("ms").and_then(serde_json::Value::as_u64).unwrap_or(0)
            )
        })
        .collect();
    (!deferred.is_empty()).then(|| RedirectFinding {
        status: CriterionStatus::NeedsReview,
        details: format!(
            "ouverture de fenêtre différée suspecte, non survenue dans la fenêtre de {} s: {}",
            REDIRECT_OBSERVATION_MS / 1000,
            deferred.join(", ")
        ),
    })
}

/// The 13.2 result in the shape the pipeline merges.
#[must_use]
pub fn popup_results(finding: Option<&RedirectFinding>) -> HashMap<String, CriterionResult> {
    let mut map = redirect_results(finding);
    if let Some(mut result) = map.remove("13.1") {
        result.criterion_id = "13.2".to_string();
        map.insert("13.2".to_string(), result);
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn snapshot(href: &str, log: serde_json::Value) -> serde_json::Value {
        json!({"href": href, "interacted": false, "log": log})
    }

    #[test]
    fn a_page_that_moves_on_its_own_fails_with_the_timer_trace() {
        let observation = json!({
            "at_load": snapshot("http://x/r1.html", json!([
                {"kind": "setTimeout", "ms": 1500, "src": "function(){ location.replace('/t.html'); }"}
            ])),
            "after": snapshot("http://x/t.html", json!([])),
        });
        let finding = parse_redirect_observation(&observation).expect("finding");
        assert_eq!(finding.status, CriterionStatus::Fail);
        assert!(
            finding.details.contains("setTimeout(1500 ms)"),
            "{}",
            finding.details
        );
        assert!(finding.details.contains("http://x/t.html"));
    }

    #[test]
    fn a_carousel_timer_is_not_a_redirection() {
        let observation = json!({
            "at_load": snapshot("http://x/r3.html", json!([
                {"kind": "setInterval", "ms": 1000, "src": "function(){ n=(n%3)+1; el.textContent=n; }"}
            ])),
            "after": snapshot("http://x/r3.html", json!([])),
        });
        assert_eq!(parse_redirect_observation(&observation), None);
    }

    #[test]
    fn a_navigation_timer_that_has_not_fired_in_the_window_is_review() {
        let observation = json!({
            "at_load": snapshot("http://x/slow.html", json!([
                {"kind": "setTimeout", "ms": 30000, "src": "() => { window.location.href = '/next'; }"}
            ])),
            "after": snapshot("http://x/slow.html", json!([])),
        });
        let finding = parse_redirect_observation(&observation).expect("finding");
        assert_eq!(finding.status, CriterionStatus::NeedsReview);
    }

    #[test]
    fn a_fragment_change_alone_is_not_a_redirect_and_static_pages_are_silent() {
        let observation = json!({
            "at_load": snapshot("http://x/p.html", json!([])),
            "after": snapshot("http://x/p.html#top", json!([])),
        });
        assert_eq!(parse_redirect_observation(&observation), None);
        assert!(redirect_results(None).is_empty());
    }

    #[test]
    fn the_probe_never_emits_pass() {
        for status in [CriterionStatus::Fail, CriterionStatus::NeedsReview] {
            let map = redirect_results(Some(&RedirectFinding {
                status,
                details: "x".into(),
            }));
            assert_ne!(map["13.1"].status, CriterionStatus::Pass);
        }
    }

    #[test]
    fn popups_without_interaction_fail_and_interacted_or_silent_pages_do_not() {
        let fail = json!({"after": {"log": [
            {"kind": "window.open", "interacted": false, "url": "http://x/ad"}]}});
        let finding = parse_popup_observation(&fail).expect("finding");
        assert_eq!(finding.status, CriterionStatus::Fail);
        assert!(finding.details.contains("window.open(http://x/ad)"));
        let clicked = json!({"after": {"log": [
            {"kind": "window.open", "interacted": true, "url": "http://x/ad"}]}});
        assert_eq!(parse_popup_observation(&clicked), None);
        let quiet = json!({"after": {"log": []}});
        assert_eq!(parse_popup_observation(&quiet), None);
    }

    #[test]
    fn a_pending_open_timer_is_review_and_popup_results_target_13_2() {
        let pending = json!({"after": {"log": [
            {"kind": "setTimeout", "ms": 30000, "src": "() => window.open('/x')"}]}});
        let finding = parse_popup_observation(&pending).expect("finding");
        assert_eq!(finding.status, CriterionStatus::NeedsReview);
        let map = popup_results(Some(&finding));
        assert!(map.contains_key("13.2") && !map.contains_key("13.1"));
        assert_ne!(map["13.2"].status, CriterionStatus::Pass);
    }
}
