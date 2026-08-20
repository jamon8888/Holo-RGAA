use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio::sync::Semaphore;
use tokio::time::{timeout, Instant};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{error, info, warn};

pub mod config;
pub mod evidence;
pub mod guided;
pub mod results;

pub use config::{
    AdvancedRulePolicy, AnalyzeConfig, AnalyzeRequest, CookieReference, NeedsReviewPolicy,
    PreScanAction, ScreenshotPolicy, Viewport,
};
pub use evidence::{EvidenceArtifact, EvidenceRef, EvidenceStore};
pub use guided::{
    is_stable_accessibility_reference, GuidedAction, GuidedExecutor, GuidedObservation,
    GuidedRunResult, GuidedStep, GuidedTest, TerminationReason,
};
pub use results::{AnalyzePageResult, ObscuraError};
use rgaa_core::{CriterionStatus, Finding, FindingFingerprint};
use rgaa_rules::AxeMapper;
use serde::Deserialize;
use sha2::{Digest, Sha256};

const AXE_CORE_CDN: &str = "https://cdnjs.cloudflare.com/ajax/libs/axe-core/4.9.1/axe.min.js";

#[derive(Debug, Deserialize)]
struct AxeViolationPayload {
    id: String,
    impact: Option<String>,
    description: String,
    help: Option<String>,
    nodes: Vec<AxeNodePayload>,
}

#[derive(Debug, Deserialize)]
struct AxeNodePayload {
    target: Vec<String>,
    html: String,
    #[serde(rename = "failureSummary")]
    failure_summary: Option<String>,
}

fn validate_axe_payload(
    value: &serde_json::Value,
) -> Result<Vec<AxeViolationPayload>, ObscuraError> {
    let array = value
        .as_array()
        .ok_or_else(|| ObscuraError::Json("axe violations must be an array".into()))?;
    let mut payload = Vec::with_capacity(array.len());
    for (index, item) in array.iter().enumerate() {
        let violation =
            serde_json::from_value::<AxeViolationPayload>(item.clone()).map_err(|error| {
                ObscuraError::Json(format!("invalid axe violation at index {index}: {error}"))
            })?;
        if violation.id.trim().is_empty()
            || violation.description.trim().is_empty()
            || violation.nodes.is_empty()
        {
            return Err(ObscuraError::Json(format!(
                "invalid axe violation at index {index}: required fields are empty"
            )));
        }
        if violation
            .nodes
            .iter()
            .any(|node| node.target.is_empty() || node.html.trim().is_empty())
        {
            return Err(ObscuraError::Json(format!(
                "invalid axe violation at index {index}: malformed node"
            )));
        }
        payload.push(violation);
    }
    Ok(payload)
}

fn findings_from_axe(
    url: &str,
    value: &serde_json::Value,
    evidence: &[rgaa_core::EvidenceRef],
) -> Result<Vec<Finding>, ObscuraError> {
    let payload = validate_axe_payload(value)?;
    let normalized =
        serde_json::to_string(value).map_err(|error| ObscuraError::Json(error.to_string()))?;
    let mapped = AxeMapper::map(&normalized);
    let mut findings = Vec::new();
    for violation in payload {
        let mut criteria = mapped
            .iter()
            .filter(|(_, criterion)| {
                criterion
                    .violations
                    .iter()
                    .any(|item| item.rule_id == violation.id)
            })
            .map(|(criterion_id, _)| criterion_id.clone())
            .collect::<Vec<_>>();
        criteria.sort();
        if criteria.is_empty() {
            criteria.push("unmapped".into());
        }
        for (node_index, node) in violation.nodes.iter().enumerate() {
            for criterion_id in &criteria {
                let target = node.target.join(" | ");
                let mut finding =
                    Finding::new(format!("rgaa-{criterion_id}-{}-{node_index}", violation.id));
                finding.rule = violation.id.clone();
                finding.criterion_id = (criterion_id != "unmapped").then(|| criterion_id.clone());
                finding.url = url.into();
                finding.target = target;
                finding.html = Some(node.html.clone());
                finding.details = node
                    .failure_summary
                    .clone()
                    .or_else(|| Some(violation.description.clone()));
                finding.status = CriterionStatus::Fail;
                finding.severity = violation.impact.clone().or_else(|| Some("unknown".into()));
                finding.description = Some(violation.description.clone());
                finding.remediation = violation.help.clone();
                finding.evidence = evidence.to_vec();
                finding.source = "axe-core".into();
                findings.push(finding);
            }
        }
    }
    findings.sort_by_key(|finding| {
        (
            finding.id.clone(),
            FindingFingerprint::from_finding(finding),
        )
    });
    Ok(findings)
}

pub struct ObscuraBridge {
    binary_path: String,
    server_port: u16,
    server_process: Option<Child>,
}

impl Default for ObscuraBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl ObscuraBridge {
    pub fn new() -> Self {
        Self {
            binary_path: "obscura".to_string(),
            server_port: 9222,
            server_process: None,
        }
    }

    pub fn with_binary_path(path: String) -> Self {
        Self {
            binary_path: path,
            server_port: 9222,
            server_process: None,
        }
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.server_port = port;
        self
    }

    /// Analyze one page using the structured audit contract.
    ///
    /// Request validation failures are returned as [`ObscuraError`]. Browser
    /// failures are returned in the page envelope so callers cannot mistake a
    /// failed page for a clean page.
    pub async fn analyze(
        &self,
        request: &AnalyzeRequest,
    ) -> Result<AnalyzePageResult, ObscuraError> {
        request.validate_supported()?;
        let started = Instant::now();
        let axe_source = match timeout(
            Duration::from_millis(request.config.timeout_ms),
            self.fetch_axe_source(),
        )
        .await
        {
            Ok(Ok(source)) => source,
            Ok(Err(error)) => {
                return Ok(AnalyzePageResult::failed(
                    &request.url,
                    Self::classify_error(error),
                    started.elapsed().as_millis() as u64,
                ))
            }
            Err(_) => {
                return Ok(AnalyzePageResult::failed(
                    &request.url,
                    ObscuraError::Timeout("timed out fetching axe-core".into()),
                    started.elapsed().as_millis() as u64,
                ))
            }
        };
        let mut attempt = 0;
        let (raw, evidence) = loop {
            let result = timeout(
                Duration::from_millis(request.config.timeout_ms),
                self.run_configured_axe(request, &axe_source),
            )
            .await;
            match result {
                Ok(Ok(result)) => break result,
                Ok(Err(_error)) if attempt < usize::from(request.config.retry_limit) => {
                    attempt += 1;
                    continue;
                }
                Ok(Err(error)) => {
                    return Ok(AnalyzePageResult::failed(
                        &request.url,
                        error,
                        started.elapsed().as_millis() as u64,
                    ))
                }
                Err(_) if attempt < usize::from(request.config.retry_limit) => {
                    attempt += 1;
                    continue;
                }
                Err(_) => {
                    return Ok(AnalyzePageResult::failed(
                        &request.url,
                        ObscuraError::Timeout(format!(
                            "analysis timed out after {} ms",
                            request.config.timeout_ms
                        )),
                        started.elapsed().as_millis() as u64,
                    ))
                }
            }
        };
        let violations = match serde_json::from_str::<serde_json::Value>(&raw) {
            Ok(violations) => violations,
            Err(error) => {
                return Ok(AnalyzePageResult::failed(
                    &request.url,
                    ObscuraError::Json(error.to_string()),
                    started.elapsed().as_millis() as u64,
                ))
            }
        };
        let findings = match findings_from_axe(&request.url, &violations, &evidence) {
            Ok(findings) => findings,
            Err(error) => {
                return Ok(AnalyzePageResult::failed(
                    &request.url,
                    error,
                    started.elapsed().as_millis() as u64,
                ))
            }
        };

        let completed = !evidence.is_empty();
        Ok(AnalyzePageResult {
            url: request.url.clone(),
            findings,
            evidence,
            errors: Vec::new(),
            completed,
            duration_ms: started.elapsed().as_millis() as u64,
        })
    }

    /// Execute a versioned guided test through the Obscura browser adapter.
    pub async fn run_guided_test(
        &self,
        test: &GuidedTest,
    ) -> Result<GuidedRunResult, ObscuraError> {
        let mut executor = guided::ObscuraGuidedExecutor::connect(self).await?;
        let root = std::env::temp_dir()
            .join("rgaa-guided-evidence")
            .join(&test.id);
        let store = EvidenceStore::new(root);
        let result = test.run(&mut executor, Some(&store)).await;
        let cleanup = executor.close().await;
        match (result, cleanup) {
            (Ok(result), Ok(())) => Ok(result),
            (Ok(_), Err(error)) => Err(error),
            (Err(error), _) => Err(error),
        }
    }

    fn classify_error(error: String) -> ObscuraError {
        let lower = error.to_ascii_lowercase();
        if lower.contains("timed out") || lower.contains("timeout") {
            ObscuraError::Timeout(error)
        } else if lower.contains("navigation") || lower.contains("load") {
            ObscuraError::Navigation(error)
        } else if lower.contains("websocket") || lower.contains("cdp") {
            ObscuraError::CdpTransport(error)
        } else if lower.contains("json") || lower.contains("result") {
            ObscuraError::Json(error)
        } else if lower.contains("screenshot")
            || lower.contains("evidence")
            || lower.contains("dom evidence")
        {
            ObscuraError::Evidence(error)
        } else if lower.contains("missing secret") || lower.contains("policy") {
            ObscuraError::PolicyDenied(error)
        } else if lower.contains("unsupported") {
            ObscuraError::UnsupportedConfiguration(error)
        } else if lower.contains("failed to spawn") {
            ObscuraError::ProcessStartup(error)
        } else {
            ObscuraError::Evaluation(error)
        }
    }

    async fn run_configured_axe(
        &self,
        request: &AnalyzeRequest,
        axe_source: &str,
    ) -> Result<(String, Vec<rgaa_core::EvidenceRef>), ObscuraError> {
        let ws_url = self
            .get_browser_ws_url()
            .await
            .map_err(Self::classify_error)?;
        let (mut ws, _) = connect_async(&ws_url)
            .await
            .map_err(|error| ObscuraError::CdpTransport(error.to_string()))?;
        let target_id = Self::cdp_send(
            &mut ws,
            "Target.createTarget",
            serde_json::json!({"url": request.url}),
        )
        .await
        .map_err(Self::classify_error)?
        .get("targetId")
        .and_then(|value| value.as_str())
        .ok_or_else(|| ObscuraError::CdpTransport("missing target id".into()))?
        .to_owned();
        let session_id = Self::cdp_send(
            &mut ws,
            "Target.attachToTarget",
            serde_json::json!({"targetId": target_id, "flatten": true}),
        )
        .await
        .map_err(Self::classify_error)?
        .get("sessionId")
        .and_then(|value| value.as_str())
        .ok_or_else(|| ObscuraError::CdpTransport("missing session id".into()))?
        .to_owned();

        let outcome = self
            .run_axe_core_configured(&mut ws, &session_id, request, axe_source)
            .await;
        let cleanup = Self::cleanup_target(&mut ws, &session_id, &target_id).await;
        match (outcome.map_err(Self::classify_error), cleanup) {
            (Ok(value), Ok(())) => Ok(value),
            (Ok(_), Err(error)) => Err(Self::classify_error(error)),
            (Err(error), _) => Err(error),
        }
    }

    async fn run_axe_core_configured(
        &self,
        ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        session_id: &str,
        request: &AnalyzeRequest,
        axe_source: &str,
    ) -> Result<(String, Vec<rgaa_core::EvidenceRef>), String> {
        Self::wait_for_load(
            ws,
            session_id,
            Duration::from_millis(request.config.timeout_ms),
        )
        .await?;
        Self::cdp_send_session(
            ws,
            session_id,
            "Emulation.setDeviceMetricsOverride",
            serde_json::json!({
                "width": request.config.viewport.width,
                "height": request.config.viewport.height,
                "deviceScaleFactor": 1,
                "mobile": request.config.profile == "mobile"
            }),
        )
        .await?;
        self.apply_cookies(ws, session_id, request).await?;
        self.apply_pre_scan_actions(ws, session_id, request).await?;
        let inject = Self::cdp_send_session(
            ws,
            session_id,
            "Runtime.evaluate",
            serde_json::json!({"expression": format!("(function() {{ {} }})()", axe_source)}),
        )
        .await?;
        if inject.get("exceptionDetails").is_some() {
            return Err("axe-core injection threw an exception".into());
        }
        let axe_argument = request
            .config
            .selector
            .as_ref()
            .map(|selector| serde_json::to_string(selector).map_err(|error| error.to_string()))
            .transpose()?;
        let expression = match axe_argument {
            Some(selector) => format!("axe.run({selector})"),
            None => "axe.run()".into(),
        };
        let result = Self::cdp_send_session(
            ws,
            session_id,
            "Runtime.evaluate",
            serde_json::json!({"expression": expression, "awaitPromise": true, "returnByValue": true}),
        )
        .await?;
        let raw = Self::validate_axe_result(&result)?;
        let violations = serde_json::from_str::<serde_json::Value>(&raw)
            .map_err(|error| format!("invalid axe JSON: {error}"))?;
        let evidence = self
            .capture_evidence(ws, session_id, request, &violations)
            .await?;
        Ok((raw, evidence))
    }

    async fn apply_cookies(
        &self,
        ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        session_id: &str,
        request: &AnalyzeRequest,
    ) -> Result<(), String> {
        if request.config.cookie_references.is_empty() {
            return Ok(());
        }
        Self::cdp_send_session(ws, session_id, "Network.enable", serde_json::json!({})).await?;
        for cookie in &request.config.cookie_references {
            let key = cookie
                .name
                .chars()
                .map(|character| {
                    if character.is_ascii_alphanumeric() {
                        character.to_ascii_uppercase()
                    } else {
                        '_'
                    }
                })
                .collect::<String>();
            let value = std::env::var(format!("RGAA_COOKIE_{key}"))
                .map_err(|_| format!("missing secret for cookie reference '{}'", cookie.name))?;
            let mut params = serde_json::json!({
                "name": cookie.name,
                "value": value,
                "url": request.url,
            });
            if let Some(domain) = &cookie.domain {
                params["domain"] = serde_json::Value::String(domain.clone());
            }
            let result =
                Self::cdp_send_session(ws, session_id, "Network.setCookie", params).await?;
            if result.get("success").and_then(|value| value.as_bool()) == Some(false) {
                return Err(format!(
                    "browser rejected cookie reference '{}'",
                    cookie.name
                ));
            }
        }
        Ok(())
    }

    async fn apply_pre_scan_actions(
        &self,
        ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        session_id: &str,
        request: &AnalyzeRequest,
    ) -> Result<(), String> {
        for action in &request.config.pre_scan_actions {
            let (selector, body) = match action {
                PreScanAction::Click { selector } => {
                    (selector, "element.click(); return true;".to_string())
                }
                PreScanAction::Fill { selector, value } => {
                    let value = serde_json::to_string(value).map_err(|error| error.to_string())?;
                    (
                        selector,
                        format!(
                            "element.value = {value}; element.dispatchEvent(new Event('input', {{bubbles:true}})); element.dispatchEvent(new Event('change', {{bubbles:true}})); return true;"
                        ),
                    )
                }
            };
            let selector = serde_json::to_string(selector).map_err(|error| error.to_string())?;
            let expression = format!(
                "(() => {{ const element = document.querySelector({selector}); if (!element) throw new Error('pre-scan selector not found'); {body} }})()"
            );
            let result = Self::cdp_send_session(
                ws,
                session_id,
                "Runtime.evaluate",
                serde_json::json!({"expression": expression, "returnByValue": true}),
            )
            .await?;
            if result.get("exceptionDetails").is_some() {
                return Err("pre-scan action evaluation failed".into());
            }
        }
        Ok(())
    }

    async fn capture_evidence(
        &self,
        ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        session_id: &str,
        request: &AnalyzeRequest,
        violations: &serde_json::Value,
    ) -> Result<Vec<rgaa_core::EvidenceRef>, String> {
        let dom = Self::cdp_send_session(
            ws,
            session_id,
            "Runtime.evaluate",
            serde_json::json!({
                "expression": "document.documentElement.outerHTML",
                "returnByValue": true
            }),
        )
        .await?;
        let dom = dom
            .get("result")
            .and_then(|value| value.get("value"))
            .and_then(|value| value.as_str())
            .ok_or_else(|| "DOM evidence was not returned by the browser".to_string())?;
        let mut evidence = vec![Self::evidence_ref("dom_snapshot", dom.as_bytes())];
        let should_screenshot = match request.config.screenshot_policy {
            ScreenshotPolicy::None => false,
            ScreenshotPolicy::Always => true,
            ScreenshotPolicy::OnFailure => {
                violations.as_array().is_some_and(|items| !items.is_empty())
            }
        };
        if should_screenshot {
            let screenshot = Self::cdp_send_session(
                ws,
                session_id,
                "Page.captureScreenshot",
                serde_json::json!({"format": "png"}),
            )
            .await?;
            let data = screenshot
                .get("data")
                .and_then(|value| value.as_str())
                .ok_or_else(|| "screenshot evidence was not returned by the browser".to_string())?;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(data)
                .map_err(|error| format!("invalid screenshot evidence: {error}"))?;
            evidence.push(Self::evidence_ref("screenshot", &bytes));
        }
        Ok(evidence)
    }

    fn evidence_ref(kind: &str, bytes: &[u8]) -> rgaa_core::EvidenceRef {
        let digest = Sha256::digest(bytes);
        rgaa_core::EvidenceRef::new(kind, format!("sha256:{digest:x}"))
    }

    /// Start the obscura CDP server as a background process
    pub async fn start_server(&mut self) -> Result<(), String> {
        info!(port = self.server_port, "Starting Obscura CDP server");

        let child = Command::new(&self.binary_path)
            .arg("serve")
            .arg("--port")
            .arg(self.server_port.to_string())
            .arg("--quiet")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start obscura serve: {e}"))?;

        self.server_process = Some(child);

        // Wait for server to be ready
        for i in 0..50 {
            if let Ok(resp) = reqwest::get(&format!(
                "http://127.0.0.1:{}/json/version",
                self.server_port
            ))
            .await
            {
                if resp.status().is_success() {
                    info!(attempt = i, "Obscura CDP server ready");
                    return Ok(());
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Err("Obscura CDP server failed to start within 5s".to_string())
    }

    /// Stop the background CDP server
    pub async fn stop_server(&mut self) {
        if let Some(mut child) = self.server_process.take() {
            let _ = child.kill().await;
            info!("Obscura CDP server stopped");
        }
    }

    /// Get the browser-level WebSocket URL from /json/version
    async fn get_browser_ws_url(&self) -> Result<String, String> {
        let resp = reqwest::get(&format!(
            "http://127.0.0.1:{}/json/version",
            self.server_port
        ))
        .await
        .map_err(|e| format!("Failed to get CDP version: {e}"))?;

        let version: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse CDP version: {e}"))?;

        version["webSocketDebuggerUrl"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "No webSocketDebuggerUrl in /json/version".to_string())
    }

    /// Fetch the axe-core source once (used by single and batch runs)
    async fn fetch_axe_source(&self) -> Result<String, String> {
        reqwest::get(AXE_CORE_CDN)
            .await
            .map_err(|e| format!("Failed to fetch axe-core: {e}"))?
            .text()
            .await
            .map_err(|e| format!("Failed to read axe-core: {e}"))
    }

    /// Run axe-core via CDP (supports async evaluation)
    ///
    /// Fetches the axe-core source once and delegates to [`Self::run_axe_with_script`].
    pub async fn run_axe(&self, url: &str) -> Result<String, String> {
        let axe_source = self.fetch_axe_source().await?;
        self.run_axe_with_script(url, &axe_source).await
    }

    /// Run axe-core against `url` using a pre-fetched axe-core source string.
    ///
    /// This avoids re-downloading axe-core per URL when batching. The created CDP
    /// target is always detached/closed on every exit path (success or error).
    pub(crate) async fn run_axe_with_script(
        &self,
        url: &str,
        axe_source: &str,
    ) -> Result<String, String> {
        // 1. Connect to browser-level WebSocket
        let ws_url = self.get_browser_ws_url().await?;
        let (mut ws, _) = connect_async(&ws_url)
            .await
            .map_err(|e| format!("Failed to connect to CDP WebSocket: {e}"))?;

        // 2. Create a new target and get its ID
        let target_id = {
            let resp = Self::cdp_send(
                &mut ws,
                "Target.createTarget",
                serde_json::json!({
                    "url": url,
                }),
            )
            .await?;
            resp.get("targetId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "No targetId in createTarget response".to_string())?
                .to_string()
        };

        // 3. Attach to the target and get session ID
        let session_id = {
            let resp = Self::cdp_send(
                &mut ws,
                "Target.attachToTarget",
                serde_json::json!({
                    "targetId": target_id,
                    "flatten": true,
                }),
            )
            .await?;
            resp.get("sessionId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "No sessionId in attachToTarget response".to_string())?
                .to_string()
        };

        // 4. Run the actual evaluation, then always clean up the target.
        let outcome = self.run_axe_core(&mut ws, &session_id, axe_source).await;

        let cleanup = Self::cleanup_target(&mut ws, &session_id, &target_id).await;
        match (outcome, cleanup) {
            (Ok(value), Ok(())) => Ok(value),
            (Ok(_), Err(error)) => Err(error),
            (Err(error), _) => Err(error),
        }
    }

    /// Inner axe-core evaluation: wait for navigation, inject axe, then run it.
    async fn run_axe_core(
        &self,
        ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        session_id: &str,
        axe_source: &str,
    ) -> Result<String, String> {
        // Wait for the page to load (lifecycle event or readyState), bounded.
        Self::wait_for_load(ws, session_id, Duration::from_secs(15)).await?;

        // 5. Inject axe-core via script source
        let inject = Self::cdp_send_session(
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

        // 6. Run axe.run() and capture the resolved value directly.
        let result = Self::cdp_send_session(
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

        Self::validate_axe_result(&result)
    }

    /// Validate the resolved axe.run() result. Any missing/exception/null/subtype
    /// result is treated as an error so a failure cannot masquerade as a clean run.
    ///
    /// `result` is the CDP `Runtime.evaluate` "result" object: `{ result: <RemoteObject>, exceptionDetails? }`.
    fn validate_axe_result(result: &serde_json::Value) -> Result<String, String> {
        if let Some(ex) = result.get("exceptionDetails") {
            return Err(format!("axe.run() raised an exception: {ex}"));
        }

        let remote = result
            .get("result")
            .ok_or_else(|| "axe.run() response missing result object".to_string())?;

        if remote.get("subtype").and_then(|s| s.as_str()) == Some("error") {
            return Err("axe.run() returned an error object".to_string());
        }

        let value = remote
            .get("value")
            .ok_or_else(|| "axe.run() result value is missing".to_string())?;

        if value.is_null() {
            return Err("axe.run() result value is null".to_string());
        }

        let violations = value
            .get("violations")
            .ok_or_else(|| "axe result is missing the 'violations' field".to_string())?;

        if !violations.is_array() {
            return Err("axe result 'violations' is not an array".to_string());
        }

        serde_json::to_string(violations)
            .map_err(|e| format!("failed to serialize axe violations: {e}"))
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
                let id = Self::cdp_issue(
                    ws,
                    "Runtime.evaluate",
                    serde_json::json!({
                        "expression": "document.readyState",
                        "returnByValue": true,
                    }),
                    Some(session_id),
                )
                .await?;
                pending_readystate = Some(id);
            }

            let wait = if pending_readystate.is_some() {
                remaining
            } else {
                poll_interval.min(remaining)
            };

            match tokio::time::timeout(wait, ws.next()).await {
                Ok(Some(Ok(Message::Text(text)))) => {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
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
                    return Err(
                        "CDP WebSocket stream ended while waiting for page load".to_string()
                    );
                }
                Err(_) => {
                    // Timed out waiting for a message; re-check deadline and retry.
                }
                _ => {}
            }
        }
    }

    /// Run axe-core on multiple URLs concurrently using CDP workers.
    ///
    /// Fetches axe-core once, then bounds concurrent `run_axe_with_script` calls
    /// with a semaphore sized by `concurrency` (treated as 1 when 0).
    pub async fn run_axe_batch(
        &self,
        urls: &[String],
        concurrency: usize,
    ) -> Result<HashMap<String, String>, String> {
        if urls.is_empty() {
            return Ok(HashMap::new());
        }

        let concurrency = std::cmp::max(1, concurrency);

        info!(
            urls = urls.len(),
            concurrency, "Running batch axe-core audit via CDP"
        );

        // Fetch axe-core once for the whole batch.
        let axe_source = self.fetch_axe_source().await?;
        let sem = Arc::new(Semaphore::new(concurrency));

        let (tx, mut rx) = mpsc::channel(urls.len());

        for url in urls {
            let tx = tx.clone();
            let binary_path = self.binary_path.clone();
            let port = self.server_port;
            let url = url.clone();
            let axe = axe_source.clone();
            let sem = Arc::clone(&sem);

            tokio::spawn(async move {
                let _permit = sem.acquire().await.expect("semaphore closed unexpectedly");

                let bridge = ObscuraBridge {
                    binary_path,
                    server_port: port,
                    server_process: None,
                };

                let result = bridge.run_axe_with_script(&url, &axe).await;
                let _ = tx.send((url, result)).await;
            });
        }

        drop(tx);

        let mut results = HashMap::new();
        while let Some((url, result)) = rx.recv().await {
            match result {
                Ok(violations) => {
                    results.insert(url, violations);
                }
                Err(e) => {
                    warn!(url = %url, error = %e, "axe-core failed");
                }
            }
        }

        Ok(results)
    }

    /// Run gap-fix snippets on a single URL using CLI (sync)
    pub async fn run_gap_fix(
        &self,
        url: &str,
        snippets: &HashMap<String, &str>,
    ) -> Result<HashMap<String, serde_json::Value>, String> {
        let mut results = HashMap::new();

        for (criterion_id, snippet) in snippets {
            let script = Self::build_gap_fix_script(snippet);
            match self.run_obscura_fetch(url, &script).await {
                Ok(output) => {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&output) {
                        results.insert(criterion_id.clone(), value);
                    }
                }
                Err(e) => {
                    error!("Gap-fix failed for {}: {}", criterion_id, e);
                }
            }
        }

        Ok(results)
    }

    /// Run gap-fix snippets on multiple URLs concurrently via the CLI `scrape` command.
    ///
    /// All URLs are passed to a single `obscura scrape` invocation (scrape accepts
    /// multiple positional URLs). The single JSON object returned is parsed into a
    /// per-URL map; a non-conforming payload is an error (entries are never dropped silently).
    pub async fn run_gap_fix_batch(
        &self,
        urls: &[String],
        snippets: &HashMap<String, &str>,
        concurrency: usize,
    ) -> Result<HashMap<String, HashMap<String, serde_json::Value>>, String> {
        let snippet_decls: String = snippets
            .iter()
            .map(|(id, snippet)| {
                format!(
                    r#"
    const snippet_{id} = (() => {{
      try {{
        {snippet}
      }} catch (e) {{
        {{ success: false, error: e.message }};
      }}
    }})();
 "#
                )
            })
            .collect();

        let object_entries: String = snippets
            .keys()
            .map(|id| format!("'{id}': snippet_{id}"))
            .collect::<Vec<_>>()
            .join(", ");

        let script = format!(
            r#"
  {snippet_decls}
  JSON.stringify({{{object_entries}}});
 "#
        );

        info!(
            urls = urls.len(),
            snippets = snippets.len(),
            concurrency,
            "Running batch gap-fix via CLI"
        );

        let output = timeout(Duration::from_secs(300), async {
            Command::new(&self.binary_path)
                .arg("scrape")
                .args(urls.iter())
                .arg("--eval")
                .arg(&script)
                .arg("--concurrency")
                .arg(concurrency.to_string())
                .arg("--format")
                .arg("json")
                .arg("--timeout")
                .arg("60")
                .arg("--quiet")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
                .map_err(|e| format!("Failed to spawn obscura scrape: {e}"))
        })
        .await
        .map_err(|_| "obscura scrape timed out after 300s".to_string())??;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(stderr = %stderr, "obscura scrape gap-fix failed");
            return Err(format!("obscura scrape gap-fix failed: {stderr}"));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed = Self::parse_scrape_results(&stdout)?;

        let mut all_results = HashMap::new();
        for (url, value) in parsed {
            let map = serde_json::from_value::<HashMap<String, serde_json::Value>>(value)
                .map_err(|e| format!("failed to parse gap-fix eval for {url}: {e}"))?;
            all_results.insert(url, map);
        }

        Ok(all_results)
    }

    /// Extract page context using CLI (sync)
    pub async fn extract_page_context(&self, url: &str) -> Result<serde_json::Value, String> {
        let script = Self::build_page_context_script();
        let output = self.run_obscura_fetch(url, script).await?;
        serde_json::from_str(&output).map_err(|e| e.to_string())
    }

    /// Extract page context for multiple URLs concurrently using CLI scrape.
    ///
    /// All URLs are passed to a single `obscura scrape` invocation and the result
    /// is parsed into a per-URL map (entries are never dropped silently).
    pub async fn extract_page_context_batch(
        &self,
        urls: &[String],
        concurrency: usize,
    ) -> Result<HashMap<String, serde_json::Value>, String> {
        let script = Self::build_page_context_script();

        info!(
            urls = urls.len(),
            concurrency, "Running batch page context extraction via CLI"
        );

        let output = timeout(Duration::from_secs(300), async {
            Command::new(&self.binary_path)
                .arg("scrape")
                .args(urls.iter())
                .arg("--eval")
                .arg(script)
                .arg("--concurrency")
                .arg(concurrency.to_string())
                .arg("--format")
                .arg("json")
                .arg("--timeout")
                .arg("60")
                .arg("--quiet")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
                .map_err(|e| format!("Failed to spawn obscura scrape: {e}"))
        })
        .await
        .map_err(|_| "obscura scrape timed out after 300s".to_string())??;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(stderr = %stderr, "obscura scrape page context failed");
            return Err(format!("obscura scrape page context failed: {stderr}"));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Self::parse_scrape_results(&stdout)
    }

    /// Parse the real Obscura `scrape` output: a single JSON object containing a
    /// `results` array, where each element has a `url` and an `eval` (string) field.
    /// Returns an error if the payload is not in this shape.
    fn parse_scrape_results(stdout: &str) -> Result<HashMap<String, serde_json::Value>, String> {
        let value: serde_json::Value = serde_json::from_str(stdout)
            .map_err(|e| format!("failed to parse obscura scrape output: {e}"))?;

        let results = value
            .get("results")
            .and_then(|r| r.as_array())
            .ok_or_else(|| "obscura scrape output missing 'results' array".to_string())?;

        let mut map = HashMap::new();
        for entry in results {
            let url = entry
                .get("url")
                .and_then(|u| u.as_str())
                .ok_or_else(|| "scrape result entry missing 'url'".to_string())?;
            let eval = entry
                .get("eval")
                .and_then(|e| e.as_str())
                .ok_or_else(|| format!("scrape result entry for '{url}' missing 'eval'"))?;
            let parsed = serde_json::from_str::<serde_json::Value>(eval)
                .map_err(|e| format!("failed to parse eval for '{url}': {e}"))?;
            map.insert(url.to_string(), parsed);
        }

        Ok(map)
    }

    // --- Script builders (sync) ---

    fn build_gap_fix_script(snippet: &str) -> String {
        format!(
            r#"
 (() => {{
   try {{
     {snippet}
     return JSON.stringify({{ success: true }});
   }} catch (e) {{
     return JSON.stringify({{ success: false, error: e.message }});
   }}
 }})()
 "#
        )
    }

    fn build_page_context_script() -> &'static str {
        r#"
 (() => {
   const title = document.title;
   const lang = document.documentElement.lang;
   const headings = Array.from(document.querySelectorAll('h1, h2, h3, h4, h5, h6'))
     .map(h => ({ level: parseInt(h.tagName[1]), text: h.textContent.trim() }));
   const landmarks = Array.from(document.querySelectorAll('header, nav, main, aside, footer, [role="banner"], [role="navigation"], [role="main"], [role="complementary"], [role="contentinfo"]'))
     .map(el => ({ tag: el.tagName.toLowerCase(), role: el.getAttribute('role'), label: el.getAttribute('aria-label') }));
   const images = Array.from(document.querySelectorAll('img'))
     .map(img => ({ src: img.src, alt: img.alt, hasAlt: img.hasAttribute('alt') }));
   const forms = Array.from(document.querySelectorAll('form'))
     .map(form => ({ action: form.action, inputs: Array.from(form.querySelectorAll('input, select, textarea')).length }));
   return JSON.stringify({ title, lang, headings, landmarks, images, forms });
 })()
 "#
    }

    /// Send a CDP command (browser-level, no session) and get the result
    async fn cdp_send(
        ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let id = Self::cdp_issue(ws, method, params, None).await?;
        Self::cdp_wait_response(ws, id).await
    }

    /// Send a CDP command with session ID and get the result
    async fn cdp_send_session(
        ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        session_id: &str,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let id = Self::cdp_issue(ws, method, params, Some(session_id)).await?;
        Self::cdp_wait_response(ws, id).await
    }

    async fn cleanup_target(
        ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        session_id: &str,
        target_id: &str,
    ) -> Result<(), String> {
        let detach = Self::cdp_send(
            ws,
            "Target.detachFromTarget",
            serde_json::json!({"sessionId": session_id}),
        )
        .await
        .map(|_| ());
        let close = Self::cdp_send(
            ws,
            "Target.closeTarget",
            serde_json::json!({"targetId": target_id}),
        )
        .await
        .map(|_| ());
        let socket = ws.close(None).await.map_err(|error| error.to_string());

        detach.and(close).and(socket)
    }

    /// Build and send a CDP message, returning the generated request id.
    async fn cdp_issue(
        ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        method: &str,
        params: serde_json::Value,
        session_id: Option<&str>,
    ) -> Result<u64, String> {
        let id = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            % 1_000_000) as u64;

        let mut msg = serde_json::json!({
            "id": id,
            "method": method,
            "params": params,
        });
        if let Some(sid) = session_id {
            msg["sessionId"] = serde_json::Value::String(sid.to_string());
        }

        ws.send(Message::Text(msg.to_string()))
            .await
            .map_err(|e| format!("CDP send failed: {e}"))?;

        Ok(id)
    }

    /// Wait for a CDP response with a matching id, skipping events
    async fn cdp_wait_response(
        ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        expected_id: u64,
    ) -> Result<serde_json::Value, String> {
        let deadline = Instant::now() + Duration::from_secs(30);

        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(format!(
                    "CDP timeout waiting for response id={}",
                    expected_id
                ));
            }

            match tokio::time::timeout(remaining, ws.next()).await {
                Ok(Some(Ok(Message::Text(text)))) => {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                        // Check if this is a response to our command
                        if let Some(id) = value.get("id").and_then(|v| v.as_u64()) {
                            if id == expected_id {
                                if let Some(error) = value.get("error") {
                                    return Err(format!("CDP error: {}", error));
                                }
                                return Ok(value
                                    .get("result")
                                    .cloned()
                                    .unwrap_or(serde_json::Value::Null));
                            }
                        }
                        // Otherwise it's an event, skip it
                    }
                }
                Ok(Some(Ok(Message::Close(_)))) => return Err("CDP WebSocket closed".to_string()),
                Ok(Some(Err(e))) => return Err(format!("CDP WebSocket error: {e}")),
                Ok(None) => return Err("CDP WebSocket stream ended".to_string()),
                Err(_) => {
                    return Err(format!(
                        "CDP timeout waiting for response id={}",
                        expected_id
                    ))
                }
                _ => {}
            }
        }
    }

    /// Run a single obscura fetch command (sync operations)
    async fn run_obscura_fetch(&self, url: &str, script: &str) -> Result<String, String> {
        info!("Running Obscura fetch for {}", url);

        let output = timeout(Duration::from_secs(120), async {
            Command::new(&self.binary_path)
                .arg("fetch")
                .arg(url)
                .arg("--eval")
                .arg(script)
                .arg("--quiet")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
                .map_err(|e| format!("Failed to spawn obscura: {e}"))
        })
        .await
        .map_err(|_| "Obscura fetch timed out after 120s".to_string())??;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(stderr = %stderr, "Obscura fetch failed");
            return Err(format!("Obscura fetch failed: {stderr}"));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let lines: Vec<&str> = stdout.lines().collect();
        if let Some(last) = lines.last() {
            Ok(last.to_string())
        } else {
            Ok(stdout)
        }
    }
}

impl Drop for ObscuraBridge {
    fn drop(&mut self) {
        if let Some(mut child) = self.server_process.take() {
            let _ = child.start_kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Network-dependent: requires a reachable browser/CDP server and example.com.
    #[tokio::test]
    async fn test_run_axe_with_broken_script_surfaces_error() {
        let mut bridge = ObscuraBridge::new().with_port(9244);
        bridge.start_server().await.expect("failed to start server");

        // A broken axe source throws during injection, which must surface as Err
        // rather than being silently treated as a clean (empty) result.
        let broken = "throw new Error('boom')";
        let result = bridge
            .run_axe_with_script("https://example.com", broken)
            .await;

        bridge.stop_server().await;

        assert!(
            result.is_err(),
            "broken evaluation should surface an Err, got: {:?}",
            result.ok()
        );
    }

    #[tokio::test]
    async fn analyze_rejects_invalid_request_before_starting_browser_work() {
        let bridge = ObscuraBridge::new();
        let request = AnalyzeRequest {
            url: "file:///secret.html".into(),
            config: AnalyzeConfig::default(),
        };

        assert!(matches!(
            bridge.analyze(&request).await,
            Err(ObscuraError::Validation(_))
        ));
    }

    #[test]
    fn browser_failures_are_classified_as_page_errors() {
        let result = AnalyzePageResult::failed(
            "https://unreachable.test",
            ObscuraBridge::classify_error("timed out waiting for page load".into()),
            1,
        );

        assert!(!result.completed);
        assert_eq!(result.errors[0].code, "timeout");
    }

    #[test]
    fn malformed_axe_items_are_rejected_before_mapping() {
        let payload = serde_json::json!([{}]);
        assert!(matches!(
            validate_axe_payload(&payload),
            Err(ObscuraError::Json(_))
        ));
    }

    #[test]
    fn axe_findings_preserve_context_and_have_stable_order() {
        let payload = serde_json::json!([{
            "id": "image-alt",
            "impact": "critical",
            "description": "Images must have alternate text",
            "help": "Ensure alt text",
            "nodes": [{"target": ["#z"], "html": "<img id=\"z\">"}, {"target": ["#a"], "html": "<img id=\"a\">"}]
        }]);
        let evidence = vec![rgaa_core::EvidenceRef::new("dom_snapshot", "sha256:x")];
        let findings = findings_from_axe("https://example.test", &payload, &evidence)
            .expect("valid axe payload");
        assert_eq!(findings.len(), 14);
        assert!(findings.windows(2).all(|pair| pair[0].id <= pair[1].id));
        let finding = findings
            .iter()
            .find(|finding| finding.target == "#a")
            .expect("target preserved");
        assert_eq!(finding.html.as_deref(), Some("<img id=\"a\">"));
        assert_eq!(finding.criterion_id.as_deref(), Some("1.1"));
        assert_eq!(finding.source, "axe-core");
        assert_eq!(finding.evidence, evidence);
    }
}
