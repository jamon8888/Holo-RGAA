//! `screen.orientation.lock` calls (issue #268, RGAA 13.9 — hook part only).
//!
//! Obscura 0.2.4 exposes `screen.orientation` without a `lock` method (measured), so the
//! hook defines one that records the requested orientation. Any recorded call, with no
//! user interaction, is a page that tries to force an orientation. The content-diff
//! between portrait and landscape renderings (also in #268) is **not** implemented here.

use std::time::Duration;

use serde::Deserialize;

use crate::ObscuraBridge;

/// Init script: records `screen.orientation.lock(x)` into `window.__rgaaOrientation`.
pub const ORIENTATION_HOOK: &str = r#"
(() => {
  const calls = [];
  window.__rgaaOrientation = calls;
  try {
    screen.orientation.lock = function (orientation) {
      calls.push({ orientation: String(orientation) });
      return Promise.resolve();
    };
  } catch (e) { calls.push({ orientation: 'unhookable' }); }
})();
"#;

const READ: &str = "JSON.stringify(window.__rgaaOrientation || null)";

/// One orientation-lock request made by the page.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct OrientationLock {
    /// Requested orientation, or `unhookable` when the hook could not be installed.
    pub orientation: String,
}

/// Parses the `Runtime.evaluate` result of [`READ`].
///
/// # Errors
/// Fails when the hook never ran or the payload has an unexpected shape.
pub fn parse_locks(result: &serde_json::Value) -> Result<Vec<OrientationLock>, String> {
    let raw = result
        .pointer("/result/value")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("no value in observation result: {result}"))?;
    if raw == "null" {
        return Err("orientation hook did not run (window.__rgaaOrientation is absent)".into());
    }
    serde_json::from_str(raw).map_err(|e| format!("unexpected orientation payload: {e}"))
}

impl ObscuraBridge {
    /// Loads `url` and returns every `screen.orientation.lock` call within `window`.
    ///
    /// # Errors
    /// Propagates CDP failures and a hook that did not run.
    pub async fn observe_orientation_locks(
        &self,
        url: &str,
        window: Duration,
    ) -> Result<Vec<OrientationLock>, String> {
        let result = self
            .observe_with_init_script(url, ORIENTATION_HOOK, window, READ)
            .await?;
        parse_locks(&result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lock_requests() {
        let r = serde_json::json!({"result": {"value": r#"[{"orientation":"landscape"}]"#}});
        assert_eq!(parse_locks(&r).unwrap()[0].orientation, "landscape");
    }

    #[test]
    fn a_hook_that_never_ran_is_an_error() {
        let r = serde_json::json!({"result": {"value": "null"}});
        assert!(parse_locks(&r).is_err());
    }
}
