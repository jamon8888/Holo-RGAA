//! Script-driven motion (issue #267, RGAA 13.8 — script part only).
//!
//! A `MutationObserver`, installed before any page script, counts DOM mutations per
//! target while the page is left alone for the observation window. A target that keeps
//! mutating with no interaction is moving by script (carousel, ticker, blinking text).
//! The observation also reports whether a pause/stop control exists, by accessible name.
//!
//! Hints only. Spinners and loaders mutate too, so they must be told apart by review;
//! CSS animations are **not** covered: Obscura 0.2.4's `document.getAnimations()` is a
//! stub returning `[]` (measured, see #267), and canvas/WebGL motion is invisible here.

use std::time::Duration;

use serde::Deserialize;

use crate::ObscuraBridge;

/// Init script: counts mutations per target into `window.__rgaaMotion`.
pub const MOTION_HOOK: &str = r#"
(() => {
  const counts = new Map();
  const name = (n) => {
    const e = n.nodeType === 1 ? n : n.parentElement;
    return e ? e.tagName.toLowerCase() + (e.id ? '#' + e.id : '') : 'other';
  };
  window.__rgaaMotion = () => {
    const pause = [...document.querySelectorAll('button, [role=button], a')].some((e) =>
      /(pause|stop|arr[eê]t|suspend)/i.test(
        (e.getAttribute('aria-label') || '') + ' ' + (e.textContent || '') + ' ' + (e.title || '')));
    return JSON.stringify({
      mutated: [...counts].map(([target, count]) => ({ target, count })),
      has_pause_control: pause,
    });
  };
  const start = () => {
    new MutationObserver((records) => {
      for (const r of records) counts.set(name(r.target), (counts.get(name(r.target)) || 0) + 1);
    }).observe(document, { subtree: true, childList: true, characterData: true, attributes: true });
  };
  start();
})();
"#;

const READ: &str = "window.__rgaaMotion ? window.__rgaaMotion() : null";

/// A DOM target that kept mutating on its own.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MovingTarget {
    pub target: String,
    pub count: u64,
}

/// What the page did on its own during the window.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MotionObservation {
    pub mutated: Vec<MovingTarget>,
    /// A button or link named like pause/stop exists.
    pub has_pause_control: bool,
}

impl MotionObservation {
    /// Targets that mutated at least `min_count` times — repeated change, not a one-off
    /// render.
    #[must_use]
    pub fn repeating(&self, min_count: u64) -> Vec<&MovingTarget> {
        self.mutated
            .iter()
            .filter(|t| t.count >= min_count)
            .collect()
    }

    /// Parses the `Runtime.evaluate` result of [`READ`].
    ///
    /// # Errors
    /// Fails when the hook never ran or the payload has an unexpected shape.
    pub fn from_cdp(result: &serde_json::Value) -> Result<Self, String> {
        let raw = result
            .pointer("/value")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "motion hook did not run (no window.__rgaaMotion)".to_string())?;
        serde_json::from_str(raw).map_err(|e| format!("unexpected motion payload: {e}"))
    }
}

impl ObscuraBridge {
    /// Loads `url` and reports DOM motion over `window` with no interaction.
    ///
    /// # Errors
    /// Propagates CDP failures and a hook that did not run.
    pub async fn observe_script_motion(
        &self,
        url: &str,
        window: Duration,
    ) -> Result<MotionObservation, String> {
        let result = self
            .observe_with_init_script(url, MOTION_HOOK, window, READ)
            .await?;
        MotionObservation::from_cdp(&result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cdp(value: &str) -> serde_json::Value {
        serde_json::json!({"value": value})
    }

    #[test]
    fn parses_and_filters_repeating_targets() {
        let obs = MotionObservation::from_cdp(&cdp(
            r#"{"mutated":[{"target":"p#slide","count":9},{"target":"div","count":1}],"has_pause_control":false}"#,
        ))
        .unwrap();
        let moving = obs.repeating(3);
        assert_eq!(moving.len(), 1);
        assert_eq!(moving[0].target, "p#slide");
        assert!(!obs.has_pause_control);
    }

    #[test]
    fn a_hook_that_never_ran_is_an_error() {
        assert!(MotionObservation::from_cdp(&serde_json::json!({})).is_err());
    }
}
