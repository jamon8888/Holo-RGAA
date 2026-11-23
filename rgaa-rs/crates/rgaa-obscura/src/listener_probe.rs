//! Event-listener inventory (issue #272: RGAA 12.10, 13.10, 13.11, 13.12).
//!
//! Reuses [`ObscuraBridge::observe_with_init_script`]: a hook installed before any page
//! script records each `addEventListener` call (event type, target, delay). The
//! inventory yields **hints only** — never `fail`, never `pass`: it tells an agent or a
//! reviewer where to look. Listeners added after the observation window, or through
//! `on*` properties, are not seen, so an empty inventory proves nothing.
//!
//! Obscura 0.2.4 defines `addEventListener` on `window`, `Node.prototype`,
//! `Document.prototype` and `Element.prototype` (measured), not on `EventTarget` alone,
//! so every owner with its own copy is patched.

use std::time::Duration;

use serde::Deserialize;

use crate::ObscuraBridge;

/// Init script: records `addEventListener` calls into `window.__rgaaListeners`.
pub const LISTENER_HOOK: &str = r#"
(() => {
  const t0 = Date.now();
  const calls = [];
  window.__rgaaListeners = calls;
  const describe = (t) => {
    if (t === window) return 'window';
    if (t === document) return 'document';
    if (t && t.tagName) return t.tagName.toLowerCase() + (t.id ? '#' + t.id : '');
    return 'other';
  };
  const wrap = (owner) => {
    const original = owner.addEventListener;
    if (typeof original !== 'function') return;
    owner.addEventListener = function (type, ...rest) {
      try { calls.push({ type: String(type), target: describe(this), at_ms: Date.now() - t0 }); } catch (e) {}
      return original.call(this, type, ...rest);
    };
  };
  // Obscura defines the method on several prototypes rather than on EventTarget alone
  // (measured), so patch every owner that has its own copy.
  const owners = [window, typeof EventTarget !== 'undefined' && EventTarget.prototype,
    typeof Node !== 'undefined' && Node.prototype,
    typeof Document !== 'undefined' && Document.prototype,
    typeof Element !== 'undefined' && Element.prototype];
  for (const owner of owners) {
    if (owner && Object.prototype.hasOwnProperty.call(owner, 'addEventListener')) wrap(owner);
  }
})();
"#;

const READ: &str = "JSON.stringify(window.__rgaaListeners || null)";

/// One recorded `addEventListener` call.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ListenerCall {
    pub type_: String,
    pub target: String,
    pub at_ms: u64,
}

#[derive(Deserialize)]
struct RawCall {
    #[serde(rename = "type")]
    type_: String,
    target: String,
    at_ms: u64,
}

/// A pointer for a reviewer: this criterion deserves a look because of this listener.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListenerHint {
    pub criterion: &'static str,
    pub event: String,
    pub target: String,
}

/// Parses the `Runtime.evaluate` result of [`READ`].
///
/// # Errors
/// Fails when the hook never ran or the payload has an unexpected shape.
pub fn parse_listeners(result: &serde_json::Value) -> Result<Vec<ListenerCall>, String> {
    let raw = result
        .pointer("/value")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("no value in observation result: {result}"))?;
    if raw == "null" {
        return Err("listener hook did not run (window.__rgaaListeners is absent)".to_string());
    }
    let calls: Vec<RawCall> =
        serde_json::from_str(raw).map_err(|e| format!("unexpected listener payload: {e}"))?;
    Ok(calls
        .into_iter()
        .map(|c| ListenerCall {
            type_: c.type_,
            target: c.target,
            at_ms: c.at_ms,
        })
        .collect())
}

/// Maps recorded listeners to per-criterion hints (spec §8: hints, no verdict).
#[must_use]
pub fn hints(calls: &[ListenerCall]) -> Vec<ListenerHint> {
    let mut out = Vec::new();
    for call in calls {
        let event = call.type_.as_str();
        let global = call.target == "window" || call.target == "document";
        let mut push = |criterion| {
            out.push(ListenerHint {
                criterion,
                event: call.type_.clone(),
                target: call.target.clone(),
            });
        };
        if global && matches!(event, "keydown" | "keypress" | "keyup") {
            push("12.10");
        }
        if matches!(
            event,
            "touchstart" | "touchmove" | "pointermove" | "gesturestart" | "gesturechange"
        ) {
            push("13.10");
        }
        if matches!(event, "mousedown" | "pointerdown" | "touchstart") {
            push("13.11");
        }
        if matches!(event, "devicemotion" | "deviceorientation") {
            push("13.12");
        }
    }
    out
}

impl ObscuraBridge {
    /// Loads `url` and returns every listener registered within `window`.
    ///
    /// # Errors
    /// Propagates CDP failures and a hook that did not run.
    pub async fn observe_listeners(
        &self,
        url: &str,
        window: Duration,
    ) -> Result<Vec<ListenerCall>, String> {
        let result = self
            .observe_with_init_script(url, LISTENER_HOOK, window, READ)
            .await?;
        parse_listeners(&result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(type_: &str, target: &str) -> ListenerCall {
        ListenerCall {
            type_: type_.into(),
            target: target.into(),
            at_ms: 0,
        }
    }

    #[test]
    fn global_key_listeners_hint_12_10_but_element_ones_do_not() {
        let h = hints(&[call("keydown", "document"), call("keydown", "input#q")]);
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].criterion, "12.10");
        assert_eq!(h[0].target, "document");
    }

    #[test]
    fn touchstart_hints_both_gesture_and_action_criteria() {
        let criteria: Vec<_> = hints(&[call("touchstart", "div#map")])
            .iter()
            .map(|h| h.criterion)
            .collect();
        assert_eq!(criteria, ["13.10", "13.11"]);
    }

    #[test]
    fn motion_sensors_hint_13_12_and_plain_clicks_hint_nothing() {
        assert_eq!(
            hints(&[call("devicemotion", "window")])[0].criterion,
            "13.12"
        );
        assert!(hints(&[call("click", "button#ok"), call("scroll", "window")]).is_empty());
    }

    #[test]
    fn a_hook_that_never_ran_is_an_error() {
        let r = serde_json::json!({"value": "null"});
        assert!(parse_listeners(&r).is_err());
    }

    #[test]
    fn parses_recorded_calls() {
        let r = serde_json::json!({"value":
            r#"[{"type":"keydown","target":"document","at_ms":4}]"#});
        assert_eq!(
            parse_listeners(&r).unwrap(),
            [call("keydown", "document").with_at(4)]
        );
    }

    impl ListenerCall {
        fn with_at(mut self, at_ms: u64) -> Self {
            self.at_ms = at_ms;
            self
        }
    }
}
