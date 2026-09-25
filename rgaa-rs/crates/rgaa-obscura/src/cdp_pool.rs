// CDP Session Pool - Persistent WebSocket connections for Chrome DevTools Protocol
// Reuses WebSocket connections and browser targets across multiple CDP operations,
// eliminating per-call TLS handshake and target creation overhead.

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, Semaphore};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::debug;

/// A pooled CDP session with a persistent WebSocket and browser target
struct PooledSession {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    session_id: String,
    created_at: Instant,
    last_used: Instant,
    in_use: bool,
}

/// Guard that returns the session to the pool when dropped
pub struct CdpSessionGuard {
    pool: Arc<Mutex<Vec<PooledSession>>>,
    index: usize,
    session_id: String,
}

impl CdpSessionGuard {
    /// Get the CDP session ID for this guard
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Execute an async closure with mutable access to the WebSocket
    /// Navigate to a URL in the current target
    pub async fn navigate(&mut self, url: &str) -> Result<(), String> {
        let session_id = self.session_id().to_string();
        let url = url.to_string();
        let mut pool = self.pool.lock().await;
        let ws = &mut pool[self.index].ws;
        cdp_send_session(
            ws,
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
        let mut pool = self.pool.lock().await;
        let ws = &mut pool[self.index].ws;
        wait_for_load(ws, &session_id, timeout).await
    }

    /// Run axe-core evaluation
    pub async fn run_axe_core(&mut self, axe_source: &str) -> Result<String, String> {
        let session_id = self.session_id().to_string();
        let axe_source = axe_source.to_string();
        let mut pool = self.pool.lock().await;
        let ws = &mut pool[self.index].ws;
        run_axe_core_static(ws, &session_id, &axe_source).await
    }

    /// Evaluate JavaScript expression
    pub async fn eval_js(&mut self, expression: &str) -> Result<serde_json::Value, String> {
        let session_id = self.session_id().to_string();
        let expression = expression.to_string();
        let mut pool = self.pool.lock().await;
        let ws = &mut pool[self.index].ws;
        eval_js_static(ws, &session_id, &expression).await
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
        if let Ok(mut pool) = self.pool.try_lock() {
            if let Some(session) = pool.get_mut(self.index) {
                session.in_use = false;
                session.last_used = Instant::now();
            }
        }
    }
}

/// CDP Session Pool - maintains a pool of persistent WebSocket connections
pub struct CdpSessionPool {
    browser_ws_url: String,
    semaphore: Arc<Semaphore>,
    pool: Arc<Mutex<Vec<PooledSession>>>,
    max_idle: Duration,
    max_lifetime: Duration,
}

impl CdpSessionPool {
    /// Create a new CDP session pool
    pub async fn new(browser_ws_url: String, max_concurrent: usize) -> Result<Self, String> {
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
        let _permit = self.semaphore.acquire().await.map_err(|_| "pool closed")?;

        let mut pool = self.pool.lock().await;
        let now = Instant::now();

        // Try to reuse an idle session within TTL
        if let Some(idx) = pool.iter().position(|s| {
            !s.in_use
                && now.duration_since(s.last_used) < self.max_idle
                && now.duration_since(s.created_at) < self.max_lifetime
        }) {
            let session = &mut pool[idx];
            session.in_use = true;
            session.last_used = now;
            debug!("Reused CDP session {}", session.session_id);
            return Ok(CdpSessionGuard {
                pool: self.pool.clone(),
                index: idx,
                session_id: session.session_id.clone(),
            });
        }

        // Create new session - release lock before async operations
        drop(pool);

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

        let mut pool = self.pool.lock().await;
        let session = PooledSession {
            ws,
            session_id: session_id.clone(),
            created_at: now,
            last_used: now,
            in_use: true,
        };
        let index = pool.len();
        pool.push(session);

        Ok(CdpSessionGuard {
            pool: self.pool.clone(),
            index,
            session_id,
        })
    }
}
