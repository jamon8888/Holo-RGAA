// CDP Session Pool - Persistent WebSocket connections for Chrome DevTools Protocol
// Reuses WebSocket connections and browser targets across multiple CDP operations,
// eliminating per-call TLS handshake and target creation overhead.

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::debug;

/// A pooled CDP session with a persistent WebSocket and browser target.
///
/// The WebSocket sits behind its *own* lock rather than the pool's: a CDP
/// round trip can take tens of seconds (`wait_for_load`, `axe.run()`), and
/// holding the pool-wide lock across it serialized every session and blocked
/// `acquire` — no pooled operation ran in parallel.
struct PooledSession {
    ws: Mutex<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    session_id: String,
    target_id: String,
    created_at: Instant,
    last_used: Mutex<Instant>,
    /// Atomic so [`CdpSessionGuard::drop`] can release the slot without
    /// taking any lock — a `try_lock` there silently leaked the slot whenever
    /// it was contended.
    in_use: AtomicBool,
}

/// Guard that returns the session to the pool when dropped
pub struct CdpSessionGuard {
    session: Arc<PooledSession>,
    /// Held for the guard's lifetime so `max_concurrent` actually bounds the
    /// number of live sessions; a plain `acquire()` permit was released as
    /// soon as `acquire` returned, making the limit a no-op.
    _permit: OwnedSemaphorePermit,
}

impl CdpSessionGuard {
    /// Get the CDP session ID for this guard
    pub fn session_id(&self) -> &str {
        &self.session.session_id
    }

    /// Execute an async closure with mutable access to the WebSocket
    /// Navigate to a URL in the current target
    pub async fn navigate(&mut self, url: &str) -> Result<(), String> {
        let session_id = self.session_id().to_string();
        let url = url.to_string();
        let mut ws = self.session.ws.lock().await;
        cdp_send_session(
            &mut ws,
            &session_id,
            "Page.navigate",
            serde_json::json!({"url": url}),
        )
        .await?;
        Ok(())
    }

    /// Wait for page load
    pub async fn wait_for_load(&mut self, timeout: Duration) -> Result<(), String> {
        let session_id = self.session_id().to_string();
        let mut ws = self.session.ws.lock().await;
        wait_for_load(&mut ws, &session_id, timeout).await
    }

    /// Run axe-core evaluation
    pub async fn run_axe_core(&mut self, axe_source: &str) -> Result<String, String> {
        let session_id = self.session_id().to_string();
        let axe_source = axe_source.to_string();
        let mut ws = self.session.ws.lock().await;
        run_axe_core_static(&mut ws, &session_id, &axe_source).await
    }

    /// Evaluate JavaScript expression
    pub async fn eval_js(&mut self, expression: &str) -> Result<serde_json::Value, String> {
        let session_id = self.session_id().to_string();
        let expression = expression.to_string();
        let mut ws = self.session.ws.lock().await;
        eval_js_static(&mut ws, &session_id, &expression).await
    }
}

/// Send a CDP command on a session connection
async fn cdp_send_session(
    ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    session_id: &str,
    method: &str,
    params: Value,
) -> Result<Value, String> {
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;

    let request = serde_json::json!({
        "id": id,
        "method": method,
        "params": params,
        "sessionId": session_id,
    });

    ws.send(Message::Text(request.to_string()))
        .await
        .map_err(|e| format!("CDP session send failed: {e}"))?;

    while let Some(msg) = ws.next().await {
        let msg = msg.map_err(|e| format!("CDP session recv failed: {e}"))?;
        if let Message::Text(text) = msg {
            if let Ok(response) = serde_json::from_str::<Value>(&text) {
                if response.get("id").and_then(|v| v.as_u64()) == Some(id) {
                    if let Some(error) = response.get("error") {
                        return Err(format!("CDP session error: {}", error));
                    }
                    return Ok(response.get("result").cloned().unwrap_or(Value::Null));
                }
            }
        }
    }
    Err("CDP session connection closed".to_string())
}

/// Wait for navigation to finish by observing `Page.loadEventFired` /
/// `Page.lifecycleEvent` (name == "load") OR polling `document.readyState`
/// until "complete". Returns once either is observed, or Err on timeout.
async fn wait_for_load(
    ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    session_id: &str,
    timeout_dur: Duration,
) -> Result<(), String> {
    let deadline = Instant::now() + timeout_dur;
    let poll_interval = Duration::from_millis(300);
    let mut last_poll = Instant::now() - poll_interval - Duration::from_millis(1);
    let mut pending_readystate: Option<u64> = None;

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(
                "timed out waiting for page load (loadEventFired / readyState)".to_string(),
            );
        }

        // Issue a readyState poll when none is outstanding and the interval elapsed.
        if pending_readystate.is_none() && last_poll.elapsed() >= poll_interval {
            last_poll = Instant::now();
            let id = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64;
            let request = serde_json::json!({
                "id": id,
                "method": "Runtime.evaluate",
                "params": {
                    "expression": "document.readyState",
                    "returnByValue": true,
                },
                "sessionId": session_id,
            });

            ws.send(Message::Text(request.to_string()))
                .await
                .map_err(|e| format!("CDP send failed: {e}"))?;
            pending_readystate = Some(id);
        }

        let wait = if pending_readystate.is_some() {
            remaining
        } else {
            poll_interval.min(remaining)
        };

        match tokio::time::timeout(wait, ws.next()).await {
            Ok(Some(Ok(Message::Text(text)))) => {
                if let Ok(value) = serde_json::from_str::<Value>(&text) {
                    if let Some(id) = value.get("id").and_then(|v| v.as_u64()) {
                        if pending_readystate == Some(id) {
                            pending_readystate = None;
                            let val = value
                                .get("result")
                                .and_then(|r| r.get("result"))
                                .and_then(|v| v.get("value"))
                                .and_then(|v| v.as_str());
                            if val == Some("complete") {
                                return Ok(());
                            }
                        }
                    } else if let Some(method) = value.get("method").and_then(|m| m.as_str()) {
                        if method == "Page.loadEventFired" {
                            return Ok(());
                        }
                        if method == "Page.lifecycleEvent"
                            && value
                                .get("params")
                                .and_then(|p| p.get("name"))
                                .and_then(|n| n.as_str())
                                == Some("load")
                        {
                            return Ok(());
                        }
                    }
                }
            }
            Ok(Some(Ok(Message::Close(_)))) => {
                return Err("CDP WebSocket closed while waiting for page load".to_string());
            }
            Ok(Some(Err(e))) => {
                return Err(format!(
                    "CDP WebSocket error while waiting for page load: {e}"
                ));
            }
            Ok(None) => {
                return Err("CDP WebSocket stream ended while waiting for page load".to_string());
            }
            Err(_) => {
                // Timed out waiting for a message; re-check deadline and retry.
            }
            _ => {}
        }
    }
}

/// Static version of run_axe_core for use with pooled sessions (no &self needed)
async fn run_axe_core_static(
    ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    session_id: &str,
    axe_source: &str,
) -> Result<String, String> {
    // Wait for the page to load (lifecycle event or readyState), bounded.
    wait_for_load(ws, session_id, Duration::from_secs(15)).await?;

    // Inject axe-core via script source
    let inject = cdp_send_session(
        ws,
        session_id,
        "Runtime.evaluate",
        serde_json::json!({
            "expression": format!("(function() {{ {} }})()", axe_source),
        }),
    )
    .await?;

    if inject.get("exceptionDetails").is_some() {
        return Err("axe-core injection threw an exception".to_string());
    }

    // Run axe.run() and capture the resolved value directly.
    let result = cdp_send_session(
        ws,
        session_id,
        "Runtime.evaluate",
        serde_json::json!({
            "expression": "axe.run()",
            "awaitPromise": true,
            "returnByValue": true,
        }),
    )
    .await?;

    // Validate the result
    if let Some(ex) = result.get("exceptionDetails") {
        return Err(format!("axe.run() raised an exception: {ex}"));
    }

    let remote = result
        .get("result")
        .ok_or_else(|| "axe.run() response missing result object".to_string())?;

    if remote.get("subtype").and_then(|s| s.as_str()) == Some("error") {
        return Err("axe.run() returned an error object".to_string());
    }

    let violations = remote
        .get("value")
        .ok_or_else(|| "axe.run() result missing value".to_string())?;

    serde_json::to_string(violations)
        .map_err(|e| format!("failed to serialize axe violations: {e}"))
}

/// Static version of eval_js for use with pooled sessions (no &self needed)
async fn eval_js_static(
    ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    session_id: &str,
    expression: &str,
) -> Result<serde_json::Value, String> {
    let result = cdp_send_session(
        ws,
        session_id,
        "Runtime.evaluate",
        serde_json::json!({"expression": expression, "returnByValue": true}),
    )
    .await?;

    Ok(result)
}

/// Send a CDP command on the browser-level connection (no session ID)
async fn cdp_send(
    ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    method: &str,
    params: Value,
) -> Result<Value, String> {
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;

    let request = serde_json::json!({
        "id": id,
        "method": method,
        "params": params,
    });

    ws.send(Message::Text(request.to_string()))
        .await
        .map_err(|e| format!("CDP send failed: {e}"))?;

    while let Some(msg) = ws.next().await {
        let msg = msg.map_err(|e| format!("CDP recv failed: {e}"))?;
        if let Message::Text(text) = msg {
            if let Ok(response) = serde_json::from_str::<Value>(&text) {
                if response.get("id").and_then(|v| v.as_u64()) == Some(id) {
                    if let Some(error) = response.get("error") {
                        return Err(format!("CDP error: {}", error));
                    }
                    return Ok(response.get("result").cloned().unwrap_or(Value::Null));
                }
            }
        }
    }
    Err("CDP connection closed".to_string())
}

impl Drop for CdpSessionGuard {
    fn drop(&mut self) {
        // `last_used` is best-effort: this guard is the session's only user
        // while `in_use` is set, so the try_lock succeeds in practice, and a
        // stale timestamp only makes the session look older and be evicted
        // sooner. Releasing the slot is not best-effort — it happens either
        // way, which is what the old pool-wide `try_lock` could not promise.
        if let Ok(mut last_used) = self.session.last_used.try_lock() {
            *last_used = Instant::now();
        }
        self.session.in_use.store(false, Ordering::Release);
    }
}

/// CDP Session Pool - maintains a pool of persistent WebSocket connections
pub struct CdpSessionPool {
    browser_ws_url: String,
    semaphore: Arc<Semaphore>,
    /// The pool lock covers slot bookkeeping only — never a CDP round trip.
    pool: Arc<Mutex<Vec<Arc<PooledSession>>>>,
    max_idle: Duration,
    max_lifetime: Duration,
}

impl CdpSessionPool {
    /// Create a new CDP session pool
    ///
    /// # Errors
    /// Returns `Err` if `max_concurrent` is zero: a zero-permit semaphore
    /// makes every `acquire` wait forever, which is indistinguishable from a
    /// hung browser.
    pub async fn new(browser_ws_url: String, max_concurrent: usize) -> Result<Self, String> {
        if max_concurrent == 0 {
            return Err("CdpSessionPool requires max_concurrent >= 1".to_string());
        }
        Ok(Self {
            browser_ws_url,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            pool: Arc::new(Mutex::new(Vec::new())),
            max_idle: Duration::from_secs(60),
            max_lifetime: Duration::from_secs(300),
        })
    }

    /// Acquire a session from the pool (creates new if none available)
    pub async fn acquire(&self) -> Result<CdpSessionGuard, String> {
        // Owned, so the permit lives as long as the guard rather than as long
        // as this function.
        let permit = Arc::clone(&self.semaphore)
            .acquire_owned()
            .await
            .map_err(|_| "pool closed")?;

        let now = Instant::now();

        // Bookkeeping only: evict what has expired, then claim an idle slot.
        // The lock is dropped before any CDP I/O below.
        let reused = {
            let mut pool = self.pool.lock().await;

            // Expired sessions used to stay in the Vec forever, so the vector,
            // the sockets and the browser targets all grew without bound over
            // a long batch.
            let mut expired: Vec<Arc<PooledSession>> = Vec::new();
            pool.retain(|s| {
                if s.in_use.load(Ordering::Acquire) {
                    return true;
                }
                let last_used = s.last_used.try_lock().map(|l| *l).unwrap_or(s.created_at);
                let alive = now.duration_since(last_used) < self.max_idle
                    && now.duration_since(s.created_at) < self.max_lifetime;
                if !alive {
                    expired.push(Arc::clone(s));
                }
                alive
            });
            for session in expired {
                let ws_url = self.browser_ws_url.clone();
                // Off the lock: closing a target is itself a CDP round trip.
                tokio::spawn(async move { close_session(&session, &ws_url).await });
            }

            // `compare_exchange` claims the slot atomically, so two callers
            // racing here cannot walk away with the same session.
            pool.iter()
                .find(|s| {
                    s.in_use
                        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                        .is_ok()
                })
                .cloned()
        };

        if let Some(session) = reused {
            *session.last_used.lock().await = now;
            debug!("Reused CDP session {}", session.session_id);
            return Ok(CdpSessionGuard {
                session,
                _permit: permit,
            });
        }

        let (mut ws, _) = connect_async(&self.browser_ws_url)
            .await
            .map_err(|e| format!("CDP connect failed: {e}"))?;

        // Create target
        let target_id = cdp_send(
            &mut ws,
            "Target.createTarget",
            serde_json::json!({"url": "about:blank"}),
        )
        .await?
        .get("targetId")
        .and_then(|v| v.as_str())
        .ok_or("no targetId in createTarget response")?
        .to_string();

        // Attach to target
        let session_id = cdp_send(
            &mut ws,
            "Target.attachToTarget",
            serde_json::json!({"targetId": target_id, "flatten": true}),
        )
        .await?
        .get("sessionId")
        .and_then(|v| v.as_str())
        .ok_or("no sessionId in attachToTarget response")?
        .to_string();

        // Enable required domains
        cdp_send_session(&mut ws, &session_id, "Runtime.enable", Value::Null)
            .await
            .map_err(|e| format!("Runtime.enable failed: {e}"))?;
        cdp_send_session(&mut ws, &session_id, "Page.enable", Value::Null)
            .await
            .map_err(|e| format!("Page.enable failed: {e}"))?;
        cdp_send_session(
            &mut ws,
            &session_id,
            "Emulation.setDeviceMetricsOverride",
            serde_json::json!({
                "width": 1280,
                "height": 720,
                "deviceScaleFactor": 1,
                "mobile": false
            }),
        )
        .await
        .map_err(|e| format!("Emulation.setDeviceMetricsOverride failed: {e}"))?;

        let session = Arc::new(PooledSession {
            ws: Mutex::new(ws),
            session_id,
            target_id,
            created_at: now,
            last_used: Mutex::new(now),
            in_use: AtomicBool::new(true),
        });
        self.pool.lock().await.push(Arc::clone(&session));

        Ok(CdpSessionGuard {
            session,
            _permit: permit,
        })
    }
}

/// Best-effort teardown of an evicted session: close the browser target, then
/// let the socket drop. Failures are logged and ignored — the session is
/// already out of the pool, and an unreachable browser is the usual cause.
async fn close_session(session: &PooledSession, browser_ws_url: &str) {
    let mut ws = session.ws.lock().await;
    if let Err(e) = cdp_send_session(
        &mut ws,
        &session.session_id,
        "Target.closeTarget",
        serde_json::json!({"targetId": session.target_id}),
    )
    .await
    {
        debug!(
            "closing evicted CDP target {} on {browser_ws_url} failed: {e}",
            session.target_id
        );
    }
    let _ = ws.close(None).await;
}
