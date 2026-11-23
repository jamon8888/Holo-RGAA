//! `screen.orientation.lock` calls (issue #268, RGAA 13.9 — hook part only).
//!
//! Obscura 0.2.4 exposes `screen.orientation` without a `lock` method (measured), so the
//! hook defines one that records the requested orientation. Any recorded call, with no
//! user interaction, is a page that tries to force an orientation.
//!
//! The second half compares the **visible text** of the page in portrait (400×800) and
//! landscape (800×400), using `Emulation.setDeviceMetricsOverride` (measured working on
//! Obscura 0.2.4, with `matchMedia` following the override). Content must remain, layout
//! may change: only text present in one orientation and absent from the other counts,
//! plus a "rotate your device" message. Text hidden by `overflow` clipping is not
//! detectable by computed style and is out of scope; an empty diff proves nothing.

use std::time::Duration;

use serde::Deserialize;
use tokio_tungstenite::connect_async;

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
        .pointer("/value")
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

/// Collects the visible text blocks of the page (computed `display`/`visibility`), and
/// whether a "rotate your device" message is among them.
const VISIBLE_TEXT: &str = r#"
(() => {
  const visible = (el) => {
    for (let e = el; e && e.nodeType === 1; e = e.parentElement) {
      const cs = getComputedStyle(e);
      if (cs.display === 'none' || cs.visibility === 'hidden' || e.hasAttribute('hidden')) return false;
    }
    return true;
  };
  const texts = new Set();
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  for (let n = walker.nextNode(); n; n = walker.nextNode()) {
    const t = n.nodeValue.replace(/\s+/g, ' ').trim();
    if (t && n.parentElement && visible(n.parentElement)) texts.add(t);
  }
  const all = [...texts];
  const rotate = all.some((t) => /(tourn|pivot|rotate|turn your|orientation)/i.test(t));
  return JSON.stringify({ texts: all, rotate_message: rotate });
})()
"#;

/// Visible text of one rendering.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Rendering {
    pub texts: Vec<String>,
    pub rotate_message: bool,
}

/// Content difference between the portrait and landscape renderings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OrientationDiff {
    /// Visible in portrait only.
    pub lost_in_landscape: Vec<String>,
    /// Visible in landscape only.
    pub lost_in_portrait: Vec<String>,
    /// A "rotate your device" message showed in at least one orientation.
    pub rotate_message: bool,
}

impl OrientationDiff {
    /// Whether the content differs between orientations or asks the user to rotate.
    #[must_use]
    pub fn is_suspect(&self) -> bool {
        self.rotate_message
            || !self.lost_in_landscape.is_empty()
            || !self.lost_in_portrait.is_empty()
    }
}

/// Diffs two renderings by visible text.
#[must_use]
pub fn diff(portrait: &Rendering, landscape: &Rendering) -> OrientationDiff {
    let missing = |a: &Rendering, b: &Rendering| -> Vec<String> {
        a.texts
            .iter()
            .filter(|t| !b.texts.contains(t))
            .cloned()
            .collect()
    };
    OrientationDiff {
        lost_in_landscape: missing(portrait, landscape),
        lost_in_portrait: missing(landscape, portrait),
        rotate_message: portrait.rotate_message || landscape.rotate_message,
    }
}

impl ObscuraBridge {
    async fn render_with_viewport(
        &self,
        url: &str,
        width: u32,
        height: u32,
    ) -> Result<Rendering, String> {
        let ws_url = self.get_browser_ws_url().await?;
        let (mut ws, _) = connect_async(&ws_url)
            .await
            .map_err(|e| format!("WebSocket connect failed: {e}"))?;
        let target = Self::cdp_send(
            &mut ws,
            "Target.createTarget",
            serde_json::json!({"url": "about:blank"}),
        )
        .await?;
        let target_id = target
            .get("targetId")
            .and_then(|v| v.as_str())
            .ok_or("No targetId in createTarget response")?
            .to_string();
        let attached = Self::cdp_send(
            &mut ws,
            "Target.attachToTarget",
            serde_json::json!({"targetId": target_id, "flatten": true}),
        )
        .await?;
        let session_id = attached
            .get("sessionId")
            .and_then(|v| v.as_str())
            .ok_or("No sessionId in attachToTarget response")?
            .to_string();

        let outcome = async {
            Self::cdp_send_session(&mut ws, &session_id, "Page.enable", serde_json::json!({}))
                .await?;
            Self::cdp_send_session(
                &mut ws,
                &session_id,
                "Emulation.setDeviceMetricsOverride",
                serde_json::json!({"width": width, "height": height, "deviceScaleFactor": 1, "mobile": true}),
            )
            .await?;
            Self::cdp_send_session(
                &mut ws,
                &session_id,
                "Page.navigate",
                serde_json::json!({"url": url}),
            )
            .await?;
            Self::wait_for_load(&mut ws, &session_id, Duration::from_secs(15)).await?;
            let result = Self::cdp_send_session(
                &mut ws,
                &session_id,
                "Runtime.evaluate",
                serde_json::json!({"expression": VISIBLE_TEXT, "returnByValue": true}),
            )
            .await?;
            let raw = result
                .pointer("/value")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| format!("no value in rendering result: {result}"))?;
            serde_json::from_str::<Rendering>(raw)
                .map_err(|e| format!("unexpected rendering payload: {e}"))
        }
        .await;

        let _ = Self::cleanup_target(&mut ws, &session_id, &target_id).await;
        outcome
    }

    /// Renders `url` in portrait (400×800) and landscape (800×400) and diffs the visible
    /// text. A suspect diff is a hint for review, not a verdict.
    ///
    /// # Errors
    /// Propagates CDP failures.
    pub async fn compare_orientations(&self, url: &str) -> Result<OrientationDiff, String> {
        let portrait = self.render_with_viewport(url, 400, 800).await?;
        let landscape = self.render_with_viewport(url, 800, 400).await?;
        Ok(diff(&portrait, &landscape))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lock_requests() {
        let r = serde_json::json!({"value": r#"[{"orientation":"landscape"}]"#});
        assert_eq!(parse_locks(&r).unwrap()[0].orientation, "landscape");
    }

    #[test]
    fn a_hook_that_never_ran_is_an_error() {
        let r = serde_json::json!({"value": "null"});
        assert!(parse_locks(&r).is_err());
    }

    fn rendering(texts: &[&str], rotate: bool) -> Rendering {
        Rendering {
            texts: texts.iter().map(|t| (*t).to_string()).collect(),
            rotate_message: rotate,
        }
    }

    #[test]
    fn same_text_in_both_orientations_is_not_suspect() {
        let r = rendering(&["Bonjour", "Menu"], false);
        assert!(!diff(&r, &r.clone()).is_suspect());
    }

    #[test]
    fn content_hidden_in_one_orientation_is_reported() {
        let d = diff(
            &rendering(&["Bonjour", "Menu"], false),
            &rendering(&["Bonjour"], false),
        );
        assert_eq!(d.lost_in_landscape, ["Menu"]);
        assert!(d.is_suspect());
    }

    #[test]
    fn a_rotate_message_alone_is_suspect() {
        let d = diff(&rendering(&["A"], false), &rendering(&["A"], true));
        assert!(d.rotate_message && d.is_suspect());
    }
}
