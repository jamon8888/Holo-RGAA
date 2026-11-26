//! Hook-based probes installed **before** the page's own scripts (spec
//! `criteria-coverage-expansion` §7.2). They need `Page.addScriptToEvaluateOnNewDocument`,
//! verified on Obscura 0.2.4, and are driven by `ObscuraBridge::observe_with_hook`.
//!
//! 13.1 (issue #265): redirects and refreshes by script. The hook records every
//! `setTimeout`/`setInterval` (delay and callback source), wraps `location.assign` and
//! `location.replace`, and flags real user interaction. Obscura exposes no global
//! `Location` and `href` cannot be intercepted, and a script navigation discards the
//! hooked document, so the verdict compares two snapshots: the page URL once loaded and
//! the URL after ~6 s **without any interaction**. A page that moved on its own is sent
//! for review because the probe cannot establish whether the user can stop it.
//!
//! Partial mechanism: it can `fail` or ask for `review`, never `pass`. Carousel timers are
//! not redirections and stay silent.

use std::collections::HashMap;

use rgaa_core::{Classification, CriterionResult, CriterionStatus};

/// How long the page is observed without interaction.
pub const REDIRECT_OBSERVATION_MS: u64 = 6_000;

/// 13.8 observation window: it must exceed the 5 s motion threshold with margin, or a
/// 1 s slider can never show more than 5 s of motion after the load snapshot.
pub const MOTION_OBSERVATION_MS: u64 = 8_000;

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
  time_origin: performance.timeOrigin,
  title: document.title,
  interacted: !!(window.__rgaa && window.__rgaa.interacted),
  log: (window.__rgaa && window.__rgaa.log) || [],
  embedded: Array.from(document.querySelectorAll('object[data], embed[src], svg animate, svg script, canvas'))
    .map(e => e.tagName.toLowerCase())
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartialProbeStatus {
    Fail,
    NeedsReview,
}

impl From<PartialProbeStatus> for CriterionStatus {
    fn from(status: PartialProbeStatus) -> Self {
        match status {
            PartialProbeStatus::Fail => Self::Fail,
            PartialProbeStatus::NeedsReview => Self::NeedsReview,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedirectFinding {
    pub status: PartialProbeStatus,
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

    let moved = without_fragment(href_at_load) != without_fragment(href_after)
        || at_load
            .get("time_origin")
            .and_then(serde_json::Value::as_f64)
            != after.get("time_origin").and_then(serde_json::Value::as_f64);
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
            status: PartialProbeStatus::NeedsReview,
            details: format!(
                "navigation sans action de l'utilisateur: {href_at_load} → {href_after} en moins de {} s ({trace})",
                REDIRECT_OBSERVATION_MS / 1000
            ),
        });
    }
    // Test 1 also covers refreshes by embedded content (`object`, `embed`, `svg`, `canvas`):
    // what happens inside it is not observable here, so its mere presence is a `review` hint.
    let embedded: Vec<String> = at_load
        .get("embedded")
        .and_then(serde_json::Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    if direct.is_empty() && suspect.is_empty() && !embedded.is_empty() {
        let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
        for tag in &embedded {
            *counts.entry(tag.as_str()).or_default() += 1;
        }
        let list = counts
            .iter()
            .map(|(tag, n)| format!("{tag} ×{n}"))
            .collect::<Vec<_>>()
            .join(", ");
        return Some(RedirectFinding {
            status: PartialProbeStatus::NeedsReview,
            details: format!(
                "contenu intégré pouvant se rafraîchir seul, non observable ici: {list}"
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
            status: PartialProbeStatus::NeedsReview,
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
        status: finding.status.into(),
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
        review_required: finding.status == PartialProbeStatus::NeedsReview,
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
    const originalOpen = window.open;
    window.open = function (url, target, ...rest) {
      const name = target == null || target === '' ? '_blank' : String(target);
      rec('window.open', { url: String(url), target: name });
      if (['_self', '_parent', '_top'].includes(name)) return originalOpen.call(this, url, target, ...rest);
      return null;
    };
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
        .filter(|e| {
            !matches!(
                e.get("target").and_then(serde_json::Value::as_str),
                Some("_self" | "_parent" | "_top")
            )
        })
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
            status: PartialProbeStatus::Fail,
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
        status: PartialProbeStatus::NeedsReview,
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

/// How long listener registration is observed (listeners are installed at load or on
/// `DOMContentLoaded`; no interaction is simulated).
pub const LISTENER_OBSERVATION_MS: u64 = 1_500;

/// Listener inventory (issue #272): wraps `addEventListener` before any page script. Obscura
/// 0.2.4 gives `window`, `document` and each prototype their own implementation, so all of
/// them are wrapped, with a re-entrancy guard so a call delegated from one layer to the next
/// is logged once. Sensor constructors are recorded too.
pub const LISTENER_HOOK: &str = r#"
(() => {
  const log = [];
  window.__rgaa = { log };
  let busy = false;
  const describe = t => t === window ? 'window' : t === document ? 'document'
    : (t && t.tagName ? t.tagName.toLowerCase() + (t.id ? '#' + t.id : '') : String(t));
  const wrap = obj => {
    try {
      if (!obj || typeof obj.addEventListener !== 'function') return;
      const o = obj.addEventListener;
      obj.addEventListener = function (type, fn, opts) {
        if (busy) return o.call(this, type, fn, opts);
        busy = true;
        try { log.push({ type: String(type), target: describe(this) }); return o.call(this, type, fn, opts); }
        finally { busy = false; }
      };
    } catch (e) {}
  };
  wrap(window); wrap(document);
  for (const n of ['EventTarget', 'Node', 'Element', 'HTMLElement']) {
    try { wrap(window[n] && window[n].prototype); } catch (e) {}
  }
  for (const name of ['Accelerometer', 'Gyroscope', 'LinearAccelerationSensor', 'AbsoluteOrientationSensor']) {
    try {
      const C = window[name];
      if (typeof C === 'function') {
        window[name] = function (...a) { log.push({ type: 'sensor:' + name, target: 'window' }); return Reflect.construct(C, a, new.target || window[name]); };
        window[name].prototype = C.prototype;
      }
    } catch (e) {}
  }
})();
"#;

/// Read once, after the short observation window.
pub const LISTENER_READ: &str = r#"
JSON.stringify({
  log: (window.__rgaa && window.__rgaa.log) || [],
  accesskeys: Array.from(document.querySelectorAll('[accesskey]')).map(e =>
    e.tagName.toLowerCase() + (e.id ? '#' + e.id : '') + '[' + e.getAttribute('accesskey') + ']')
})
"#;

fn listener_hints(observation: &serde_json::Value) -> Vec<(&'static str, String)> {
    let after = observation.get("after").unwrap_or(observation);
    let entries: Vec<(String, String)> = after
        .get("log")
        .and_then(serde_json::Value::as_array)
        .map(|log| {
            log.iter()
                .filter_map(|e| {
                    Some((
                        e.get("type")?.as_str()?.to_string(),
                        e.get("target")?.as_str()?.to_string(),
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    let accesskeys: Vec<String> = after
        .get("accesskeys")
        .and_then(serde_json::Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    let list = |types: &[&str], global_only: bool| -> Vec<String> {
        let mut seen = std::collections::BTreeSet::new();
        entries
            .iter()
            .filter(|(t, target)| {
                types.iter().any(|x| t == x)
                    && (!global_only || target == "window" || target == "document")
            })
            .filter_map(|(t, target)| {
                let item = format!("{t} sur {target}");
                seen.insert(item.clone()).then_some(item)
            })
            .collect()
    };
    let mut hints = Vec::new();
    let mut keyboard = list(&["keydown", "keypress", "keyup"], true);
    keyboard.extend(accesskeys.iter().map(|a| format!("accesskey {a}")));
    if !keyboard.is_empty() {
        hints.push((
            "12.10",
            format!("raccourcis clavier possibles: {}", keyboard.join(", ")),
        ));
    }
    let gestures = list(
        &[
            "touchstart",
            "touchmove",
            "pointermove",
            "gesturestart",
            "gesturechange",
        ],
        false,
    );
    if !gestures.is_empty() {
        hints.push((
            "13.10",
            format!("gestes complexes possibles: {}", gestures.join(", ")),
        ));
    }
    let pointer = list(&["mousedown", "pointerdown", "touchstart"], false);
    if !pointer.is_empty() {
        hints.push((
            "13.11",
            format!(
                "action à l'appui (annulation à vérifier): {}",
                pointer.join(", ")
            ),
        ));
    }
    let motion = list(
        &[
            "devicemotion",
            "deviceorientation",
            "deviceorientationabsolute",
            "sensor:Accelerometer",
            "sensor:Gyroscope",
            "sensor:LinearAccelerationSensor",
            "sensor:AbsoluteOrientationSensor",
        ],
        false,
    );
    if !motion.is_empty() {
        hints.push((
            "13.12",
            format!("usage de capteurs de mouvement: {}", motion.join(", ")),
        ));
    }
    hints
}

/// Listener-inventory hints as criterion results: only `review`, never `fail` or `pass`;
/// a page with no matching listener yields no entry for that criterion.
#[must_use]
pub fn listener_results(observation: &serde_json::Value) -> HashMap<String, CriterionResult> {
    listener_hints(observation)
        .into_iter()
        .map(|(criterion, details)| {
            let finding = RedirectFinding {
                status: PartialProbeStatus::NeedsReview,
                details,
            };
            let mut map = redirect_results(Some(&finding));
            let mut result = map.remove("13.1").expect("redirect result");
            result.criterion_id = criterion.to_string();
            result.source = "script-hook".to_string();
            (criterion.to_string(), result)
        })
        .collect()
}

/// Viewports rendered for 13.9 (issue #268): portrait then landscape.
pub const ORIENTATION_VIEWPORTS: [(u32, u32); 2] = [(390, 844), (844, 390)];

/// Records `screen.orientation.lock(...)` calls. Obscura may expose no `screen.orientation`
/// at all, in which case a page's call would throw instead of being seen: a minimal shim is
/// installed so the call is observable either way.
pub const ORIENTATION_HOOK: &str = r#"
(() => {
  const lock = [];
  window.__rgaa = { lock };
  const unsupported = type => {
    lock.push(String(type));
    return Promise.reject(Object.assign(new Error('orientation lock unavailable'), { name: 'NotSupportedError' }));
  };
  try {
    const o = screen.orientation;
    if (o && typeof o.lock === 'function') {
      const originalLock = o.lock.bind(o);
      o.lock = function (type) { lock.push(String(type)); return originalLock(type); };
    } else {
      Object.defineProperty(screen, 'orientation', { configurable: true, value: { lock: unsupported, unlock() {}, type: 'portrait-primary', angle: 0 } });
    }
  } catch (e) {
    try { Object.defineProperty(screen, 'orientation', { configurable: true, value: { lock: unsupported, unlock() {} } }); } catch (e2) {}
  }
})();
"#;

/// Visible text and landmarks for one viewport.
pub const ORIENTATION_READ: &str = r#"
(() => {
  const visible = e => {
    const r = e.getBoundingClientRect();
    const c = getComputedStyle(e);
    return r.width > 0 && r.height > 0 && c.visibility !== 'hidden' && c.display !== 'none';
  };
  const texts = new Set();
  for (const e of document.body.querySelectorAll('*')) {
    if (['SCRIPT', 'STYLE', 'NOSCRIPT'].includes(e.tagName)) continue;
    const own = Array.from(e.childNodes).filter(n => n.nodeType === 3).map(n => n.textContent).join(' ')
      .replace(/\s+/g, ' ').trim().toLowerCase();
    if (own && visible(e)) texts.add(own);
  }
  const landmarks = Array.from(document.querySelectorAll('main, nav, header, footer, aside, [role=main], [role=navigation]'))
    .filter(visible).map(e => e.tagName.toLowerCase());
  return JSON.stringify({
    w: innerWidth, h: innerHeight,
    landscape: matchMedia('(orientation: landscape)').matches,
    texts: Array.from(texts), landmarks,
    lock: (window.__rgaa && window.__rgaa.lock) || []
  });
})()
"#;

fn rotate_message(texts: &[String]) -> Option<String> {
    texts
        .iter()
        .find(|t| {
            let rotate = ["tournez", "pivotez", "rotate", "retournez"]
                .iter()
                .any(|k| t.contains(k));
            let device = [
                "appareil",
                "téléphone",
                "écran",
                "device",
                "phone",
                "screen",
                "paysage",
                "portrait",
                "landscape",
            ]
            .iter()
            .any(|k| t.contains(k));
            rotate && device
        })
        .cloned()
}

fn string_set(snapshot: &serde_json::Value, key: &str) -> Vec<String> {
    snapshot
        .get(key)
        .and_then(serde_json::Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Verdict for 13.9 from the snapshots taken at `ORIENTATION_VIEWPORTS` (portrait, landscape).
/// `fail`: an orientation lock, a "rotate your device" message shown in only one orientation,
/// or a text difference in one orientation. Text differences are `review`: extracted nodes
/// can split the same content differently across layouts. Never `pass`; `None` when snapshots
/// show no restriction or text difference.
#[must_use]
pub fn parse_orientation_observation(snapshots: &[serde_json::Value]) -> Option<RedirectFinding> {
    let [portrait, landscape] = snapshots else {
        return None;
    };
    let locks: Vec<String> = string_set(landscape, "lock")
        .into_iter()
        .filter(|t| t != "any")
        .collect();
    if !locks.is_empty() {
        return Some(RedirectFinding {
            status: if locks.iter().any(|lock| lock == "natural") {
                PartialProbeStatus::NeedsReview
            } else {
                PartialProbeStatus::Fail
            },
            details: format!(
                "verrouillage de l'orientation: screen.orientation.lock({})",
                locks.join(", ")
            ),
        });
    }
    let (p_texts, l_texts) = (
        string_set(portrait, "texts"),
        string_set(landscape, "texts"),
    );
    for (label, shown, other) in [
        ("paysage", &l_texts, &p_texts),
        ("portrait", &p_texts, &l_texts),
    ] {
        if let Some(message) = rotate_message(shown) {
            if rotate_message(other).is_none() {
                return Some(RedirectFinding {
                    status: PartialProbeStatus::Fail,
                    details: format!("message de rotation affiché en {label}: « {message} »"),
                });
            }
        }
    }
    let mut review = Vec::new();
    for (label, from, to) in [
        ("paysage", &p_texts, &l_texts),
        ("portrait", &l_texts, &p_texts),
    ] {
        let lost: Vec<&String> = from.iter().filter(|t| !to.contains(t)).collect();
        if lost.is_empty() {
            continue;
        }
        let sample: Vec<&str> = lost.iter().take(3).map(|t| t.as_str()).collect();
        let detail = format!(
            "{} texte(s) sur {} absent(s) en {label}: {}",
            lost.len(),
            from.len(),
            sample.join(" | ")
        );
        if lost.len() * 2 >= from.len() {
            return Some(RedirectFinding {
                status: PartialProbeStatus::NeedsReview,
                details: format!("contenu masqué: {detail}"),
            });
        }
        review.push(detail);
    }
    (!review.is_empty()).then(|| RedirectFinding {
        status: PartialProbeStatus::NeedsReview,
        details: format!(
            "différence de contenu entre orientations (présentation ?): {}",
            review.join("; ")
        ),
    })
}

/// The 13.9 result in the shape the pipeline merges.
#[must_use]
pub fn orientation_results(finding: Option<&RedirectFinding>) -> HashMap<String, CriterionResult> {
    let mut map = redirect_results(finding);
    if let Some(mut result) = map.remove("13.1") {
        result.criterion_id = "13.9".to_string();
        map.insert("13.9".to_string(), result);
    }
    map
}

/// 13.8 (issue #267): automatic movement by script. A `MutationObserver` started at
/// `DOMContentLoaded` counts DOM changes with no interaction; the bridge reads it at load
/// and again after the observation window, and only the **difference** counts (the page's
/// own start-up work is not motion).
///
/// Capability gap, stated rather than worked around: on Obscura 0.2.4 `document.getAnimations()`
/// exists but returns no animation for a running CSS `infinite` animation, and `getComputedStyle`
/// reports empty `animation-*` properties. CSS animations are therefore **not** detected here;
/// `blink`/`marquee` stay with the axe rules. Canvas/WebGL motion is undetectable.
pub const MOTION_HOOK: &str = r#"
(() => {
  const state = { n: 0, targets: {}, interacted: false };
  window.__rgaa = state;
  for (const t of ['click', 'keydown', 'pointerdown', 'touchstart', 'mousedown']) {
    try { document.addEventListener(t, () => { state.interacted = true; }, true); } catch (e) {}
  }
  const label = n => {
    const e = n && n.nodeType === 1 ? n : (n && n.parentElement);
    return e ? e.tagName.toLowerCase() + (e.id ? '#' + e.id : '') : 'node';
  };
  const start = () => {
    try {
      const mo = new MutationObserver(list => {
        for (const m of list) {
          state.n++;
          const k = label(m.target);
          const now = Date.now();
          const target = state.targets[k] || (state.targets[k] = { n: 0, first: now, last: now });
          target.n++;
          target.last = now;
        }
      });
      mo.observe(document.documentElement, { subtree: true, childList: true, characterData: true, attributes: true });
    } catch (e) {}
  };
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', start); else start();
})();
"#;

/// Snapshot: mutation counters and the pause/stop controls found in the page.
pub const MOTION_READ: &str = r#"
(() => {
  const st = window.__rgaa || { n: 0, targets: {} };
  const name = e => ((e.getAttribute('aria-label') || '') + ' ' + (e.textContent || '') + ' ' + (e.getAttribute('title') || '')).replace(/\s+/g, ' ').trim();
  const controls = Array.from(document.querySelectorAll('button, [role=button], a, input[type=button], input[type=submit]'))
    .map(name).filter(n => /\b(pause|stop|arr[êe]t|arr[êe]ter|suspendre|freeze|mettre en pause)\b/i.test(n));
  const loaders = Array.from(document.querySelectorAll('[role=progressbar], [role=status], [aria-busy=true], [class*=spin], [class*=load], [id*=spin], [id*=load]'))
    .map(e => e.tagName.toLowerCase() + (e.id ? '#' + e.id : ''));
  return JSON.stringify({ n: st.n, targets: st.targets, snapshot: Date.now(), interacted: !!st.interacted, controls, loaders });
})()
"#;

/// Minimum number of DOM changes during the window to call it movement (one-off updates —
/// a late-loaded widget — are not).
const MOTION_MIN_MUTATIONS: u64 = 3;

/// A failure needs more than five seconds of observed motion on a target without a control.
/// Missing timing data stays `review`, never a fail or pass; `None` means no motion was seen.
#[must_use]
pub fn parse_motion_observation(observation: &serde_json::Value) -> Option<RedirectFinding> {
    let at_load = observation.get("at_load")?;
    let after = observation.get("after")?;
    let count = |v: &serde_json::Value| v.get("n").and_then(serde_json::Value::as_u64).unwrap_or(0);
    let delta = count(after).saturating_sub(count(at_load));
    if delta < MOTION_MIN_MUTATIONS {
        return None;
    }
    let targets = |v: &serde_json::Value|
        -> std::collections::BTreeMap<String, (u64, Option<u64>, Option<u64>)> {
        v.get("targets")
            .and_then(serde_json::Value::as_object)
            .map(|o| {
                o.iter()
                    .filter_map(|(k, target)| {
                        let count = target.get("n").and_then(serde_json::Value::as_u64)?;
                        let first = target.get("first").and_then(serde_json::Value::as_u64);
                        let last = target.get("last").and_then(serde_json::Value::as_u64);
                        Some((k.clone(), (count, first, last)))
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    let (before_t, after_t) = (targets(at_load), targets(after));
    let mut moving: Vec<(String, u64)> = after_t
        .iter()
        .map(|(k, (n, _, _))| {
            (
                k.clone(),
                n.saturating_sub(before_t.get(k).map_or(0, |(count, _, _)| *count)),
            )
        })
        .filter(|(_, n)| *n >= 2)
        .collect();
    moving.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    if moving.is_empty() {
        return None;
    }
    let trace = moving
        .iter()
        .take(3)
        .map(|(k, n)| format!("{k} ({n} mutations)"))
        .collect::<Vec<_>>()
        .join(", ");
    let controls = after
        .get("controls")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    let loaders: Vec<String> = after
        .get("loaders")
        .and_then(serde_json::Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let all_on_loaders = moving.iter().all(|(k, _)| loaders.contains(k));
    let started = at_load.get("snapshot").and_then(serde_json::Value::as_u64);
    let sustained = started.is_some_and(|started| {
        moving.iter().any(|(key, _)| {
            after_t.get(key).is_some_and(|(_, first, last)| {
                (*first)
                    .zip(*last)
                    .is_some_and(|(first, last)| last.saturating_sub(first.max(started)) > 5_000)
            })
        })
    });
    let window = format!(
        "{} mutations en {} s sans interaction",
        delta,
        MOTION_OBSERVATION_MS / 1000
    );
    if all_on_loaders {
        return Some(RedirectFinding {
            status: PartialProbeStatus::NeedsReview,
            details: format!("mouvement sur un indicateur de chargement (caractère essentiel à confirmer): {trace}; {window}"),
        });
    }
    if controls > 0 || !sustained {
        return Some(RedirectFinding {
            status: PartialProbeStatus::NeedsReview,
            details: format!("mouvement automatique avec {controls} contrôle(s) pause/stop repéré(s) (efficacité à vérifier): {trace}; {window}"),
        });
    }
    Some(RedirectFinding {
        status: PartialProbeStatus::Fail,
        details: format!("mouvement automatique sans contrôle pause/stop: {trace}; {window}"),
    })
}

/// The 13.8 result in the shape the pipeline merges.
#[must_use]
pub fn motion_results(finding: Option<&RedirectFinding>) -> HashMap<String, CriterionResult> {
    let mut map = redirect_results(finding);
    if let Some(mut result) = map.remove("13.1") {
        result.criterion_id = "13.8".to_string();
        map.insert("13.8".to_string(), result);
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
    fn an_uncontrolled_redirect_is_review_when_stop_controls_are_unobserved() {
        let observation = json!({
            "at_load": snapshot("http://x/r1.html", json!([
                {"kind": "setTimeout", "ms": 1500, "src": "function(){ location.replace('/t.html'); }"}
            ])),
            "after": snapshot("http://x/t.html", json!([])),
        });
        let finding = parse_redirect_observation(&observation).expect("finding");
        assert_eq!(finding.status, PartialProbeStatus::NeedsReview);
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
        assert_eq!(finding.status, PartialProbeStatus::NeedsReview);
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
    fn same_url_reload_changes_time_origin_and_is_reported_for_review() {
        let mut before = snapshot("http://x/reload.html", json!([]));
        before["time_origin"] = json!(1_000.0);
        let mut after = snapshot("http://x/reload.html", json!([]));
        after["time_origin"] = json!(7_000.0);
        let finding = parse_redirect_observation(&json!({"at_load": before, "after": after}))
            .expect("reload finding");
        assert_eq!(finding.status, PartialProbeStatus::NeedsReview);
    }

    #[test]
    fn the_probe_never_emits_pass() {
        for status in [PartialProbeStatus::Fail, PartialProbeStatus::NeedsReview] {
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
        assert_eq!(finding.status, PartialProbeStatus::Fail);
        assert!(finding.details.contains("window.open(http://x/ad)"));
        let clicked = json!({"after": {"log": [
            {"kind": "window.open", "interacted": true, "url": "http://x/ad"}]}});
        assert_eq!(parse_popup_observation(&clicked), None);
        let quiet = json!({"after": {"log": []}});
        assert_eq!(parse_popup_observation(&quiet), None);
        let same_context = json!({"after": {"log": [
            {"kind": "window.open", "interacted": false, "target": "_self", "url": "/next"}]}});
        assert_eq!(parse_popup_observation(&same_context), None);
    }

    #[test]
    fn a_pending_open_timer_is_review_and_popup_results_target_13_2() {
        let pending = json!({"after": {"log": [
            {"kind": "setTimeout", "ms": 30000, "src": "() => window.open('/x')"}]}});
        let finding = parse_popup_observation(&pending).expect("finding");
        assert_eq!(finding.status, PartialProbeStatus::NeedsReview);
        let map = popup_results(Some(&finding));
        assert!(map.contains_key("13.2") && !map.contains_key("13.1"));
        assert_ne!(map["13.2"].status, CriterionStatus::Pass);
    }

    #[test]
    fn listener_inventory_emits_review_hints_per_criterion_and_nothing_otherwise() {
        let observation = json!({"after": {
            "log": [
                {"type": "keydown", "target": "document"},
                {"type": "keydown", "target": "input#q"},
                {"type": "devicemotion", "target": "window"},
                {"type": "mousedown", "target": "button#b"},
                {"type": "touchmove", "target": "div#map"}
            ],
            "accesskeys": ["button#b[s]"]
        }});
        let results = listener_results(&observation);
        let ids: std::collections::BTreeSet<&str> = results.keys().map(String::as_str).collect();
        assert_eq!(
            ids,
            ["12.10", "13.10", "13.11", "13.12"].into_iter().collect()
        );
        for result in results.values() {
            assert_eq!(result.status, CriterionStatus::NeedsReview);
        }
        // a keydown on a field is not a global shortcut; the document one and accesskey are.
        let kb = results["12.10"].justification.clone().unwrap();
        assert!(kb.contains("keydown sur document") && kb.contains("accesskey"));
        assert!(!kb.contains("input#q"));
        assert!(listener_results(&json!({"after": {"log": [], "accesskeys": []}})).is_empty());
    }

    fn view(landscape: bool, texts: &[&str], lock: &[&str]) -> serde_json::Value {
        json!({"landscape": landscape, "texts": texts, "landmarks": [], "lock": lock})
    }

    #[test]
    fn orientation_lock_fails_but_text_differences_are_reviewed() {
        let all = ["titre", "contenu important"];
        let locked = [view(false, &all, &[]), view(true, &all, &["portrait"])];
        assert_eq!(
            parse_orientation_observation(&locked).unwrap().status,
            PartialProbeStatus::Fail
        );
        let any = [view(false, &all, &[]), view(true, &all, &["any"])];
        assert_eq!(parse_orientation_observation(&any), None);
        let natural = [view(false, &all, &[]), view(true, &all, &["natural"])];
        assert_eq!(
            parse_orientation_observation(&natural).unwrap().status,
            PartialProbeStatus::NeedsReview
        );
        let hidden = [view(false, &all, &[]), view(true, &[], &[])];
        let finding = parse_orientation_observation(&hidden).unwrap();
        assert_eq!(finding.status, PartialProbeStatus::NeedsReview);
        assert!(finding.details.contains("paysage"), "{}", finding.details);
        let message = [
            view(false, &all, &[]),
            view(true, &["veuillez tourner votre appareil", "titre"], &[]),
        ];
        // "tourner" is not in the keyword list; the content difference stays review.
        assert_eq!(
            parse_orientation_observation(&message).unwrap().status,
            PartialProbeStatus::NeedsReview
        );
        let rotate = [
            view(false, &all, &[]),
            view(
                true,
                &[
                    "tournez votre appareil en mode portrait",
                    "titre",
                    "contenu important",
                ],
                &[],
            ),
        ];
        assert!(parse_orientation_observation(&rotate)
            .unwrap()
            .details
            .contains("message de rotation"));
        let same = [view(false, &all, &[]), view(true, &all, &[])];
        assert_eq!(parse_orientation_observation(&same), None);
    }

    #[test]
    fn a_small_content_difference_is_review_not_fail() {
        let portrait = view(false, &["a", "b", "c", "d", "menu"], &[]);
        let landscape = view(true, &["a", "b", "c", "d"], &[]);
        let finding = parse_orientation_observation(&[portrait, landscape]).unwrap();
        assert_eq!(finding.status, PartialProbeStatus::NeedsReview);
        assert_ne!(
            orientation_results(Some(&finding))["13.9"].status,
            CriterionStatus::Pass
        );
    }

    #[test]
    fn regrouped_text_across_elements_is_not_a_failure() {
        let portrait = view(false, &["Account settings"], &[]);
        let landscape = view(true, &["Account", "settings"], &[]);
        assert_eq!(
            parse_orientation_observation(&[portrait, landscape])
                .unwrap()
                .status,
            PartialProbeStatus::NeedsReview
        );
    }

    fn motion(
        n_load: u64,
        n_after: u64,
        target: &str,
        controls: &[&str],
        loaders: &[&str],
    ) -> serde_json::Value {
        json!({
            "at_load": {"n": n_load, "snapshot": 0, "targets": {target: {"n": n_load, "first": 0, "last": 0}}, "controls": [], "loaders": []},
            "after": {"n": n_after, "snapshot": 6000, "targets": {target: {"n": n_after, "first": 0, "last": 6000}}, "controls": controls, "loaders": loaders},
        })
    }

    #[test]
    fn motion_fails_only_when_timing_shows_over_five_seconds() {
        let fail = parse_motion_observation(&motion(0, 6, "div#c", &[], &[])).unwrap();
        assert_eq!(fail.status, PartialProbeStatus::Fail);
        assert!(
            fail.details.contains("div#c (6 mutations)"),
            "{}",
            fail.details
        );
        let paused = parse_motion_observation(&motion(0, 6, "div#c", &["Pause"], &[])).unwrap();
        assert_eq!(paused.status, PartialProbeStatus::NeedsReview);
        let loader =
            parse_motion_observation(&motion(0, 6, "div#spin", &[], &["div#spin"])).unwrap();
        assert_eq!(loader.status, PartialProbeStatus::NeedsReview);
        let mut short = motion(0, 6, "div#c", &[], &[]);
        short["after"]["targets"]["div#c"]["last"] = serde_json::json!(1_000);
        let uncertain = parse_motion_observation(&short).unwrap();
        assert_eq!(uncertain.status, PartialProbeStatus::NeedsReview);
        assert_ne!(
            motion_results(Some(&fail))["13.8"].status,
            CriterionStatus::Pass
        );
    }

    #[test]
    fn start_up_work_and_one_off_updates_are_not_motion() {
        assert_eq!(
            parse_motion_observation(&motion(40, 41, "div#c", &[], &[])),
            None
        );
        assert_eq!(
            parse_motion_observation(&motion(0, 0, "div#c", &[], &[])),
            None
        );
        assert_eq!(
            parse_motion_observation(&motion(30, 30, "div#c", &[], &[])),
            None
        );
    }

    #[test]
    fn embedded_active_content_is_a_review_hint_but_a_navigation_still_fails() {
        let embedded = json!({
            "at_load": {"href": "http://x/p", "interacted": false, "log": [], "embedded": ["canvas", "canvas", "object"]},
            "after": snapshot("http://x/p", json!([])),
        });
        let finding = parse_redirect_observation(&embedded).unwrap();
        assert_eq!(finding.status, PartialProbeStatus::NeedsReview);
        assert!(finding.details.contains("canvas ×2") && finding.details.contains("object ×1"));
        let moved = json!({
            "at_load": {"href": "http://x/p", "interacted": false, "log": [], "embedded": ["canvas"]},
            "after": snapshot("http://x/q", json!([])),
        });
        assert_eq!(
            parse_redirect_observation(&moved).unwrap().status,
            PartialProbeStatus::NeedsReview
        );
    }
}
