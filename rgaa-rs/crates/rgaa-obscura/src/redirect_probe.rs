//! Scripted-redirect observation (RGAA 13.1, issue #265).
//!
//! A page that navigates itself with no user action changes context unannounced. The
//! hook below is installed before any page script (see
//! [`ObscuraBridge::observe_with_init_script`]) and records each `location.replace` /
//! `location.assign` call with the delay since load, instead of letting it happen.
//!
//! Obscura 0.2.4 exposes no `Location` constructor: those methods are own, writable
//! properties of `location`, so they are patched there. Assigning `location.href` is
//! intercepted only if the property is configurable; when it is not, the gap is reported
//! in [`RedirectObservation::unhooked`] rather than silently ignored — a probe that
//! cannot see a mechanism must say so, since silence is never evidence of conformance.
//! Timers (`setTimeout`/`setInterval`) are deliberately **not** treated as redirects:
//! carousels use them legitimately.

use std::time::Duration;

use serde::Deserialize;

use crate::ObscuraBridge;

/// Init script: records scripted navigations into `window.__rgaaNav`.
pub const REDIRECT_HOOK: &str = r#"
(() => {
  const t0 = Date.now();
  const calls = [];
  const unhooked = [];
  window.__rgaaNav = { calls, unhooked };
  for (const method of ['replace', 'assign']) {
    try {
      location[method] = function (target) {
        calls.push({ method, target: String(target), at_ms: Date.now() - t0 });
      };
    } catch (e) { unhooked.push(method); }
  }
  try {
    Object.defineProperty(location, 'href', {
      configurable: true,
      get() { return ''; },
      set(target) { calls.push({ method: 'href', target: String(target), at_ms: Date.now() - t0 }); },
    });
  } catch (e) { unhooked.push('href'); }
})();
"#;

const READ: &str = "JSON.stringify(window.__rgaaNav || null)";

/// One navigation the page performed on its own.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ScriptedRedirect {
    /// `replace`, `assign` or `href`.
    pub method: String,
    pub target: String,
    /// Milliseconds between load and the call.
    pub at_ms: u64,
}

/// What the observation window saw.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct RedirectObservation {
    pub calls: Vec<ScriptedRedirect>,
    /// Navigation entry points that could not be hooked.
    #[serde(default)]
    pub unhooked: Vec<String>,
}

impl RedirectObservation {
    /// Parses the value returned by [`READ`] (a `Runtime.evaluate` result).
    ///
    /// # Errors
    /// Fails when the hook never ran (`null`) or the payload has an unexpected shape.
    pub fn from_cdp(result: &serde_json::Value) -> Result<Self, String> {
        let raw = result
            .pointer("/result/value")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("no value in observation result: {result}"))?;
        if raw == "null" {
            return Err("redirect hook did not run (window.__rgaaNav is absent)".to_string());
        }
        serde_json::from_str(raw).map_err(|e| format!("unexpected observation payload: {e}"))
    }
}

impl ObscuraBridge {
    /// Loads `url` and reports every scripted navigation made within `window`, with no
    /// user interaction. Sound only as a *positive* signal: an empty
    /// [`RedirectObservation::calls`] is not evidence that the page never redirects.
    ///
    /// # Errors
    /// Propagates CDP failures and a hook that did not run.
    pub async fn observe_scripted_redirects(
        &self,
        url: &str,
        window: Duration,
    ) -> Result<RedirectObservation, String> {
        let result = self
            .observe_with_init_script(url, REDIRECT_HOOK, window, READ)
            .await?;
        RedirectObservation::from_cdp(&result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cdp(value: &str) -> serde_json::Value {
        serde_json::json!({"result": {"type": "string", "value": value}})
    }

    #[test]
    fn parses_calls_and_gaps() {
        let obs = RedirectObservation::from_cdp(&cdp(
            r#"{"calls":[{"method":"replace","target":"/x","at_ms":310}],"unhooked":["href"]}"#,
        ))
        .unwrap();
        assert_eq!(obs.calls.len(), 1);
        assert_eq!(obs.calls[0].method, "replace");
        assert_eq!(obs.calls[0].at_ms, 310);
        assert_eq!(obs.unhooked, ["href"]);
    }

    #[test]
    fn a_hook_that_never_ran_is_an_error_not_an_empty_observation() {
        assert!(RedirectObservation::from_cdp(&cdp("null")).is_err());
    }

    #[test]
    fn missing_unhooked_defaults_to_empty() {
        let obs = RedirectObservation::from_cdp(&cdp(r#"{"calls":[]}"#)).unwrap();
        assert!(obs.calls.is_empty() && obs.unhooked.is_empty());
    }
}
