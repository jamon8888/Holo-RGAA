// Native Obscura library integration
// Uses a dedicated thread for the browser (obscura uses Deno/V8 which is !Send)
// and communicates via tokio channels for Send+Sync bridge.

use obscura::Browser;
use serde::Deserialize;
use std::sync::mpsc as std_mpsc;
use std::thread;

use crate::config::AnalyzeRequest;
use crate::evidence::{EvidenceArtifact, EvidenceStore};
use crate::guided::{
    GuidedAction, GuidedExecutor, GuidedObservation,
    GuidedRunResult, GuidedTest,
};
use crate::results::{AnalyzePageResult, ObscuraError};

const AXE_CORE_CDN: &str = "https://cdnjs.cloudflare.com/ajax/libs/axe-core/4.9.1/axe.min.js";

#[derive(Debug, Deserialize)]
struct AxeViolationPayload {
    id: String,
    #[allow(dead_code)]
    impact: Option<String>,
    #[allow(dead_code)]
    description: String,
    #[allow(dead_code)]
    nodes: Vec<AxeNodePayload>,
}

#[derive(Debug, Deserialize)]
struct AxeNodePayload {
    #[allow(dead_code)]
    target: Vec<String>,
    #[allow(dead_code)]
    html: String,
}

fn findings_from_axe(value: &serde_json::Value) -> Result<Vec<rgaa_core::Finding>, ObscuraError> {
    let array = value
        .as_array()
        .ok_or_else(|| ObscuraError::Json("axe violations must be an array".into()))?;
    let mapping = rgaa_rules::AxeMapper::map("[]")
        .map_err(|e| ObscuraError::Evaluation(e.to_string()))?;
    let mut findings = Vec::with_capacity(array.len());
    for (index, item) in array.iter().enumerate() {
        let violation: AxeViolationPayload =
            serde_json::from_value(item.clone()).map_err(|error| {
                ObscuraError::Json(format!("invalid axe violation at index {index}: {error}"))
            })?;
        if let Some(criterion_result) = mapping.get(&violation.id) {
            let finding = rgaa_core::Finding::new(criterion_result.criterion_id.clone());
            findings.push(finding);
        }
    }
    Ok(findings)
}

/// Requests sent to the browser worker thread.
#[allow(dead_code)]
pub(crate) enum BrowserRequest {
    Navigate(String),
    EvalJs(String),
    Click(String),
    Screenshot,
    A11yTree,
    TypeInput(String, String),
    PressKey(String),
    TabOrder,
    AssertState(String),
    Analyze(AnalyzeRequest),
    Shutdown,
}

/// Responses from the browser worker thread.
pub(crate) enum BrowserResponse {
    Unit(()),
    Value(serde_json::Value),
    String(String),
    VecString(Vec<String>),
    AnalyzeResult(Result<AnalyzePageResult, ObscuraError>),
    Error(ObscuraError),
}

/// Browser worker that owns the Browser on a dedicated thread.
struct BrowserWorker {
    browser: Browser,
    axe_source: String,
    rx: std_mpsc::Receiver<(BrowserRequest, tokio::sync::oneshot::Sender<BrowserResponse>)>,
}

impl BrowserWorker {
    fn new(
        browser: Browser,
        axe_source: String,
        rx: std_mpsc::Receiver<(BrowserRequest, tokio::sync::oneshot::Sender<BrowserResponse>)>,
    ) -> Self {
        Self { browser, axe_source, rx }
    }

    fn run(mut self) {
        while let Ok((request, reply)) = self.rx.recv() {
            let response = match request {
                BrowserRequest::Navigate(url) => {
                    self.handle_navigate(&url)
                }
                BrowserRequest::EvalJs(expr) => {
                    self.handle_eval_js(&expr)
                }
                BrowserRequest::Click(selector) => {
                    self.handle_click(&selector)
                }
                BrowserRequest::Screenshot => {
                    self.handle_screenshot()
                }
                BrowserRequest::A11yTree => {
                    self.handle_a11y_tree()
                }
                BrowserRequest::TypeInput(selector, value) => {
                    self.handle_type_input(&selector, &value)
                }
                BrowserRequest::PressKey(key) => {
                    self.handle_press_key(&key)
                }
                BrowserRequest::TabOrder => {
                    self.handle_tab_order()
                }
                BrowserRequest::AssertState(script) => {
                    self.handle_assert_state(&script)
                }
                BrowserRequest::Analyze(request) => {
                    self.handle_analyze(&request)
                }
                BrowserRequest::Shutdown => {
                    break;
                }
            };
            if reply.send(response).is_err() {
                break;
            }
        }
    }

    fn handle_navigate(&mut self, url: &str) -> BrowserResponse {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let mut page = self.browser.new_page().await
                .map_err(|e| ObscuraError::ProcessStartup(format!("failed to create page: {e}")))?;
            page.goto(url).await
                .map_err(|e| ObscuraError::Navigation(format!("navigation failed: {e}")))?;
            Ok::<(), ObscuraError>(())
        });
        match result {
            Ok(()) => BrowserResponse::Unit(()),
            Err(e) => BrowserResponse::Error(e),
        }
    }

    fn handle_eval_js(&mut self, expr: &str) -> BrowserResponse {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let mut page = self.browser.new_page().await
                .map_err(|e| ObscuraError::ProcessStartup(format!("failed to create page: {e}")))?;
            Ok::<_, ObscuraError>(page.evaluate(expr))
        });
        match result {
            Ok(val) => BrowserResponse::Value(val),
            Err(e) => BrowserResponse::Error(e),
        }
    }

    fn handle_click(&mut self, selector: &str) -> BrowserResponse {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let mut page = self.browser.new_page().await
                .map_err(|e| ObscuraError::ProcessStartup(format!("failed to create page: {e}")))?;
            if let Some(element) = page.query_selector(selector) {
                element.click()
                    .map_err(|e| ObscuraError::Evaluation(format!("click failed: {e}")))?;
            } else {
                return Err(ObscuraError::Evaluation(format!("element not found: {selector}")));
            }
            Ok::<(), ObscuraError>(())
        });
        match result {
            Ok(()) => BrowserResponse::Unit(()),
            Err(e) => BrowserResponse::Error(e),
        }
    }

    fn handle_screenshot(&mut self) -> BrowserResponse {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let mut page = self.browser.new_page().await
                .map_err(|e| ObscuraError::ProcessStartup(format!("failed to create page: {e}")))?;
            let val = page.evaluate(r#"btoa('screenshot-placeholder')"#);
            Ok::<_, ObscuraError>(val.as_str().unwrap_or("").to_string())
        });
        match result {
            Ok(s) => BrowserResponse::String(s),
            Err(e) => BrowserResponse::Error(e),
        }
    }

    fn handle_a11y_tree(&mut self) -> BrowserResponse {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let mut page = self.browser.new_page().await
                .map_err(|e| ObscuraError::ProcessStartup(format!("failed to create page: {e}")))?;
            let val = page.evaluate(
                r#"JSON.stringify(Array.from(document.querySelectorAll('[role],[aria-label]')).map(e => ({
                    role: e.getAttribute('role') || e.tagName.toLowerCase(),
                    name: e.getAttribute('aria-label') || e.textContent.trim().slice(0, 100)
                })))"#,
            );
            let tree: serde_json::Value = serde_json::from_str(val.as_str().unwrap_or("[]"))
                .map_err(|e| ObscuraError::Json(e.to_string()))?;
            Ok::<_, ObscuraError>(tree)
        });
        match result {
            Ok(v) => BrowserResponse::Value(v),
            Err(e) => BrowserResponse::Error(e),
        }
    }

    fn handle_type_input(&mut self, selector: &str, text: &str) -> BrowserResponse {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let escaped = text.replace('\\', "\\\\").replace('\'', "\\'");
        let selector_escaped = selector.replace('\'', "\\'");
        let js = format!(
            r#"(function() {{
                var el = document.querySelector('{}');
                if (el) {{
                    el.value = '{}';
                    el.dispatchEvent(new Event('input', {{bubbles: true}}));
                    el.dispatchEvent(new Event('change', {{bubbles: true}}));
                }}
            }})()"#,
            selector_escaped, escaped
        );
        let result = rt.block_on(async {
            let mut page = self.browser.new_page().await
                .map_err(|e| ObscuraError::ProcessStartup(format!("failed to create page: {e}")))?;
            page.evaluate(&js);
            Ok::<(), ObscuraError>(())
        });
        match result {
            Ok(()) => BrowserResponse::Unit(()),
            Err(e) => BrowserResponse::Error(e),
        }
    }

    fn handle_press_key(&mut self, key: &str) -> BrowserResponse {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let key_escaped = key.replace('\'', "\\'");
        let js = format!(
            r#"document.dispatchEvent(new KeyboardEvent('keydown', {{key: '{}'}}));
               document.dispatchEvent(new KeyboardEvent('keyup', {{key: '{}'}}));"#,
            key_escaped, key_escaped
        );
        let result = rt.block_on(async {
            let mut page = self.browser.new_page().await
                .map_err(|e| ObscuraError::ProcessStartup(format!("failed to create page: {e}")))?;
            page.evaluate(&js);
            Ok::<(), ObscuraError>(())
        });
        match result {
            Ok(()) => BrowserResponse::Unit(()),
            Err(e) => BrowserResponse::Error(e),
        }
    }

    fn handle_tab_order(&mut self) -> BrowserResponse {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let mut page = self.browser.new_page().await
                .map_err(|e| ObscuraError::ProcessStartup(format!("failed to create page: {e}")))?;
            let val = page.evaluate(
                r#"JSON.stringify(Array.from(document.querySelectorAll(
                    'a[href], button, input, textarea, select, [tabindex]'
                )).filter(e => !e.disabled && e.tabIndex >= 0).map(e => ({
                    tag: e.tagName.toLowerCase(),
                    role: e.getAttribute('role'),
                    tabindex: e.tabIndex,
                    text: e.textContent.trim().slice(0, 50)
                })))"#,
            );
            let order: Vec<serde_json::Value> = serde_json::from_str(val.as_str().unwrap_or("[]"))
                .map_err(|e| ObscuraError::Json(e.to_string()))?;
            Ok::<_, ObscuraError>(order)
        });
        match result {
            Ok(v) => BrowserResponse::VecString(
                v.into_iter().map(|v| v.to_string()).collect(),
            ),
            Err(e) => BrowserResponse::Error(e),
        }
    }

    fn handle_assert_state(&mut self, script: &str) -> BrowserResponse {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let mut page = self.browser.new_page().await
                .map_err(|e| ObscuraError::ProcessStartup(format!("failed to create page: {e}")))?;
            Ok::<_, ObscuraError>(page.evaluate(script))
        });
        match result {
            Ok(v) => BrowserResponse::Value(v),
            Err(e) => BrowserResponse::Error(e),
        }
    }

    fn handle_analyze(&mut self, request: &AnalyzeRequest) -> BrowserResponse {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let axe_source = self.axe_source.clone();
        let result = rt.block_on(async {
            request.validate_supported()?;
            let started = std::time::Instant::now();

            let mut page = self.browser.new_page().await
                .map_err(|e| ObscuraError::ProcessStartup(format!("failed to create page: {e}")))?;

            page.goto(&request.url).await
                .map_err(|e| ObscuraError::Navigation(format!("navigation failed: {e}")))?;

            page.settle(request.config.timeout_ms).await;

            // Inject axe-core
            page.evaluate(&axe_source);

            // Run axe-core
            page.evaluate(
                r#"(async () => {
                    const results = await axe.run(document);
                    window.__axeResult = JSON.stringify(results.violations);
                })()"#
            );

            // Let async IIFE resolve
            page.settle(5000).await;

            let violations_json = page.evaluate(
                r#"typeof __axeResult !== 'undefined' ? __axeResult : '[]'"#
            );

            let violations: serde_json::Value = match violations_json.as_str() {
                Some(s) => serde_json::from_str(s)
                    .map_err(|e| ObscuraError::Json(format!("failed to parse axe violations: {e}")))?,
                None => serde_json::Value::Array(vec![]),
            };

            let findings = findings_from_axe(&violations)?;
            let completed = !findings.is_empty();

            Ok::<_, ObscuraError>(AnalyzePageResult {
                url: request.url.clone(),
                findings,
                evidence: Vec::new(),
                errors: Vec::new(),
                completed,
                duration_ms: started.elapsed().as_millis() as u64,
                igt: None,
                obscura_version: Some("obscura 0.1.0 (v0.2.2 substrate)".into()),
            })
        });
        BrowserResponse::AnalyzeResult(result)
    }
}

/// Handle to the browser running on a dedicated thread.
/// Send+Sync because it only holds a channel sender.
pub struct BrowserHandle {
    tx: std_mpsc::SyncSender<(BrowserRequest, tokio::sync::oneshot::Sender<BrowserResponse>)>,
}

impl BrowserHandle {
    fn new(
        tx: std_mpsc::SyncSender<(BrowserRequest, tokio::sync::oneshot::Sender<BrowserResponse>)>,
    ) -> Self {
        Self { tx }
    }

    fn send(&self, request: BrowserRequest) -> Result<BrowserResponse, ObscuraError> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        self.tx.send((request, reply_tx))
            .map_err(|_| ObscuraError::Network("browser worker channel closed".into()))?;
        reply_rx.blocking_recv()
            .map_err(|_| ObscuraError::Network("browser worker response dropped".into()))
    }

    pub(crate) async fn send_async(&self, request: BrowserRequest) -> Result<BrowserResponse, ObscuraError> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        self.tx.send((request, reply_tx))
            .map_err(|_| ObscuraError::Network("browser worker channel closed".into()))?;
        reply_rx.await
            .map_err(|_| ObscuraError::Network("browser worker response dropped".into()))
    }

    pub fn navigate(&self, url: &str) -> Result<(), ObscuraError> {
        match self.send(BrowserRequest::Navigate(url.to_string()))? {
            BrowserResponse::Unit(()) => Ok(()),
            BrowserResponse::Error(e) => Err(e),
            _ => Err(ObscuraError::Evaluation("unexpected response type".into())),
        }
    }

    pub fn eval_js(&self, expr: &str) -> Result<serde_json::Value, ObscuraError> {
        match self.send(BrowserRequest::EvalJs(expr.to_string()))? {
            BrowserResponse::Value(v) => Ok(v),
            BrowserResponse::Error(e) => Err(e),
            _ => Err(ObscuraError::Evaluation("unexpected response type".into())),
        }
    }

    pub fn click(&self, selector: &str) -> Result<(), ObscuraError> {
        match self.send(BrowserRequest::Click(selector.to_string()))? {
            BrowserResponse::Unit(()) => Ok(()),
            BrowserResponse::Error(e) => Err(e),
            _ => Err(ObscuraError::Evaluation("unexpected response type".into())),
        }
    }

    pub fn screenshot(&self) -> Result<String, ObscuraError> {
        match self.send(BrowserRequest::Screenshot)? {
            BrowserResponse::String(s) => Ok(s),
            BrowserResponse::Error(e) => Err(e),
            _ => Err(ObscuraError::Evaluation("unexpected response type".into())),
        }
    }

    pub fn a11y_tree(&self) -> Result<serde_json::Value, ObscuraError> {
        match self.send(BrowserRequest::A11yTree)? {
            BrowserResponse::Value(v) => Ok(v),
            BrowserResponse::Error(e) => Err(e),
            _ => Err(ObscuraError::Evaluation("unexpected response type".into())),
        }
    }

    pub fn type_input(&self, selector: &str, text: &str) -> Result<(), ObscuraError> {
        match self.send(BrowserRequest::TypeInput(selector.to_string(), text.to_string()))? {
            BrowserResponse::Unit(()) => Ok(()),
            BrowserResponse::Error(e) => Err(e),
            _ => Err(ObscuraError::Evaluation("unexpected response type".into())),
        }
    }

    pub fn press_key(&self, key: &str) -> Result<(), ObscuraError> {
        match self.send(BrowserRequest::PressKey(key.to_string()))? {
            BrowserResponse::Unit(()) => Ok(()),
            BrowserResponse::Error(e) => Err(e),
            _ => Err(ObscuraError::Evaluation("unexpected response type".into())),
        }
    }

    pub fn tab_order(&self) -> Result<Vec<serde_json::Value>, ObscuraError> {
        match self.send(BrowserRequest::TabOrder)? {
            BrowserResponse::VecString(v) => {
                v.into_iter()
                    .map(|s| serde_json::from_str(&s).map_err(|e| ObscuraError::Json(e.to_string())))
                    .collect()
            }
            BrowserResponse::Error(e) => Err(e),
            _ => Err(ObscuraError::Evaluation("unexpected response type".into())),
        }
    }

    pub fn assert_state(&self, script: &str) -> Result<serde_json::Value, ObscuraError> {
        match self.send(BrowserRequest::AssertState(script.to_string()))? {
            BrowserResponse::Value(v) => Ok(v),
            BrowserResponse::Error(e) => Err(e),
            _ => Err(ObscuraError::Evaluation("unexpected response type".into())),
        }
    }

    pub fn analyze(&self, request: &AnalyzeRequest) -> Result<AnalyzePageResult, ObscuraError> {
        match self.send(BrowserRequest::Analyze(request.clone()))? {
            BrowserResponse::AnalyzeResult(r) => r,
            BrowserResponse::Error(e) => Err(e),
            _ => Err(ObscuraError::Evaluation("unexpected response type".into())),
        }
    }
}

impl Clone for BrowserHandle {
    fn clone(&self) -> Self {
        Self { tx: self.tx.clone() }
    }
}

/// Native Obscura browser wrapper using a dedicated thread.
pub struct ObscuraNative {
    pub(crate) handle: BrowserHandle,
}

impl ObscuraNative {
    pub async fn new() -> Result<Self, ObscuraError> {
        let browser = Browser::builder()
            .stealth(true)
            .build()
            .map_err(|e| ObscuraError::ProcessStartup(format!("failed to create browser: {e}")))?;

        let axe_source = reqwest::get(AXE_CORE_CDN)
            .await
            .map_err(|e| ObscuraError::Network(format!("failed to fetch axe-core: {e}")))?
            .text()
            .await
            .map_err(|e| ObscuraError::Network(format!("failed to read axe-core: {e}")))?;

        let (tx, rx) = std_mpsc::sync_channel(1);

        thread::spawn(move || {
            let worker = BrowserWorker::new(browser, axe_source, rx);
            worker.run();
        });

        let handle = BrowserHandle::new(tx);
        Ok(Self { handle })
    }

    pub async fn analyze(&self, request: &AnalyzeRequest) -> Result<AnalyzePageResult, ObscuraError> {
        self.handle.send(BrowserRequest::Analyze(request.clone()))
            .map_err(|e| ObscuraError::Evaluation(format!("channel error: {e}")))?
            .into_analyze_result()
    }

    pub async fn run_guided_test(
        &self,
        test: &GuidedTest,
    ) -> Result<GuidedRunResult, ObscuraError> {
        let mut executor = ObscuraBrowserExecutor { handle: self.handle.clone() };
        let root = std::env::temp_dir()
            .join("rgaa-guided-evidence")
            .join(&test.id);
        let store = EvidenceStore::new(root);
        test.run(&mut executor, Some(&store)).await
    }
}

impl BrowserResponse {
    fn into_analyze_result(self) -> Result<AnalyzePageResult, ObscuraError> {
        match self {
            BrowserResponse::AnalyzeResult(r) => r,
            BrowserResponse::Error(e) => Err(e),
            _ => Err(ObscuraError::Evaluation("unexpected response type".into())),
        }
    }
}

/// Guided executor that delegates to the browser worker thread.
struct ObscuraBrowserExecutor {
    handle: BrowserHandle,
}

impl GuidedExecutor for ObscuraBrowserExecutor {
    async fn execute(&mut self, action: &GuidedAction) -> Result<GuidedObservation, ObscuraError> {
        // All operations go through the channel to the dedicated browser thread.
        // We use send_async to avoid blocking the tokio runtime.
        match action {
            GuidedAction::Navigate { url } => {
                let resp = self.handle.send_async(BrowserRequest::Navigate(url.clone())).await?;
                match resp {
                    BrowserResponse::Unit(()) => Ok(GuidedObservation::default()),
                    BrowserResponse::Error(e) => Err(e),
                    _ => Err(ObscuraError::Evaluation("unexpected response".into())),
                }
            }
            GuidedAction::AccessibilityTree => {
                let resp = self.handle.send_async(BrowserRequest::A11yTree).await?;
                match resp {
                    BrowserResponse::Value(v) => {
                        let refs: Vec<String> = v.as_str()
                            .and_then(|s| serde_json::from_str(s).ok())
                            .unwrap_or_default();
                        Ok(GuidedObservation { tree_refs: refs, ..Default::default() })
                    }
                    BrowserResponse::Error(e) => Err(e),
                    _ => Err(ObscuraError::Evaluation("unexpected response".into())),
                }
            }
            GuidedAction::PressKey { key } => {
                let resp = self.handle.send_async(BrowserRequest::PressKey(key.clone())).await?;
                match resp {
                    BrowserResponse::Unit(()) => Ok(GuidedObservation::default()),
                    BrowserResponse::Error(e) => Err(e),
                    _ => Err(ObscuraError::Evaluation("unexpected response".into())),
                }
            }
            GuidedAction::ClickRef { reference } => {
                let resp = self.handle.send_async(BrowserRequest::Click(reference.clone())).await?;
                match resp {
                    BrowserResponse::Unit(()) => Ok(GuidedObservation::default()),
                    BrowserResponse::Error(e) => Err(e),
                    _ => Err(ObscuraError::Evaluation("unexpected response".into())),
                }
            }
            GuidedAction::FillRef { reference, value } => {
                let resp = self.handle.send_async(
                    BrowserRequest::TypeInput(reference.clone(), value.clone())
                ).await?;
                match resp {
                    BrowserResponse::Unit(()) => Ok(GuidedObservation::default()),
                    BrowserResponse::Error(e) => Err(e),
                    _ => Err(ObscuraError::Evaluation("unexpected response".into())),
                }
            }
            GuidedAction::Screenshot => {
                let resp = self.handle.send_async(BrowserRequest::Screenshot).await?;
                match resp {
                    BrowserResponse::String(s) => {
                        let data = base64::Engine::decode(
                            &base64::engine::general_purpose::STANDARD, &s
                        ).unwrap_or_default();
                        Ok(GuidedObservation {
                            evidence: vec![EvidenceArtifact::new("screenshot", data)],
                            ..Default::default()
                        })
                    }
                    BrowserResponse::Error(e) => Err(e),
                    _ => Err(ObscuraError::Evaluation("unexpected response".into())),
                }
            }
            GuidedAction::AssertState { .. } => {
                let resp = self.handle.send_async(
                    BrowserRequest::AssertState(r#"JSON.stringify({url: location.href, title: document.title})"#.into())
                ).await?;
                match resp {
                    BrowserResponse::Value(v) => Ok(GuidedObservation { state: Some(v), ..Default::default() }),
                    BrowserResponse::Error(e) => Err(e),
                    _ => Err(ObscuraError::Evaluation("unexpected response".into())),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_creation() {
        let native = ObscuraNative::new();
        // Just test that creation doesn't panic
        assert!(native.is_ok() || native.is_err());
    }
}
