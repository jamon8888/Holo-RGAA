//! Windows opened without user action (issue #266, RGAA 13.2).
//!
//! The hook, installed before any page script, replaces `window.open` with a recorder.
//! The page is then observed **without any interaction**, so every recorded call
//! happened on load or from a timer — an automatic opening. `target="_blank"` links are
//! plain navigation the user triggers; they never call `window.open` and are not
//! reported. A popup opened by a click is not triggered here (no click is simulated), so
//! it is not reported either.
//!
//! Partial by construction: popups opened after the window, or through other means
//! (`<form target>`, `showModalDialog`), are not seen, so an empty result proves nothing.

use std::time::Duration;

use serde::Deserialize;

use crate::ObscuraBridge;

/// Init script: records `window.open` calls into `window.__rgaaPopups`.
pub const POPUP_HOOK: &str = r#"
(() => {
  const t0 = Date.now();
  const calls = [];
  window.__rgaaPopups = calls;
  window.open = function (url) {
    calls.push({ url: url === undefined ? '' : String(url), at_ms: Date.now() - t0 });
    return null;
  };
})();
"#;

const READ: &str = "JSON.stringify(window.__rgaaPopups || null)";

/// One `window.open` call made with no user interaction.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AutoPopup {
    pub url: String,
    /// Milliseconds between load and the call.
    pub at_ms: u64,
}

/// Parses the `Runtime.evaluate` result of [`READ`].
///
/// # Errors
/// Fails when the hook never ran or the payload has an unexpected shape.
pub fn parse_popups(result: &serde_json::Value) -> Result<Vec<AutoPopup>, String> {
    let raw = result
        .pointer("/result/value")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("no value in observation result: {result}"))?;
    if raw == "null" {
        return Err("popup hook did not run (window.__rgaaPopups is absent)".to_string());
    }
    serde_json::from_str(raw).map_err(|e| format!("unexpected popup payload: {e}"))
}

impl ObscuraBridge {
    /// Loads `url` and returns every window the page opened on its own within `window`.
    ///
    /// # Errors
    /// Propagates CDP failures and a hook that did not run.
    pub async fn observe_auto_popups(
        &self,
        url: &str,
        window: Duration,
    ) -> Result<Vec<AutoPopup>, String> {
        let result = self
            .observe_with_init_script(url, POPUP_HOOK, window, READ)
            .await?;
        parse_popups(&result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_recorded_popups() {
        let r = serde_json::json!({"result": {"value": r#"[{"url":"/ad","at_ms":12}]"#}});
        assert_eq!(
            parse_popups(&r).unwrap(),
            [AutoPopup {
                url: "/ad".into(),
                at_ms: 12
            }]
        );
    }

    #[test]
    fn a_hook_that_never_ran_is_an_error_not_an_empty_list() {
        let r = serde_json::json!({"result": {"value": "null"}});
        assert!(parse_popups(&r).is_err());
    }
}
