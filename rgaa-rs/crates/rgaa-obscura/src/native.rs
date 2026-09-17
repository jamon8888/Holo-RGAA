// Native Obscura library integration
// Uses a dedicated thread for the browser (obscura uses Deno/V8 which is !Send)
// and communicates via tokio channels for Send+Sync bridge.

use obscura::{Browser, Page};
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

/// axe-core bundled at build time — no CDN fetch at startup, so page
/// evaluation always runs a pinned, source-controlled script.
const AXE_SOURCE: &str = include_str!("../vendor/axe.min.js");
/// SHA-256 hex digest of the vendored axe-core bundle.
const AXE_SOURCE_SHA256: &str =
    "182a40dc5d8207e626c09861ad65027d45e85e3a56d01045d068e2e88ee432ea";

/// Fail startup if the vendored axe-core bundle does not match its pinned hash.
fn verify_axe_bundle() -> Result<(), ObscuraError> {
    use sha2::Digest;
    let digest = sha2::Sha256::digest(AXE_SOURCE.as_bytes());
    let hash = format!("{digest:x}");
    if hash != AXE_SOURCE_SHA256 {
        return Err(ObscuraError::ProcessStartup(format!(
            "axe-core integrity check failed: expected {AXE_SOURCE_SHA256}, got {hash}"
        )));
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct AxeViolationPayload {
    #[allow(dead_code)]
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
    let violations_json =
        serde_json::to_string(value).map_err(|e| ObscuraError::Json(e.to_string()))?;
    let mapping = rgaa_rules::AxeMapper::map(&violations_json)
        .map_err(|e| ObscuraError::Evaluation(e.to_string()))?;
    for (index, item) in array.iter().enumerate() {
        let _: AxeViolationPayload =
            serde_json::from_value(item.clone()).map_err(|error| {
                ObscuraError::Json(format!("invalid axe violation at index {index}: {error}"))
            })?;
    }
    let findings = mapping
        .values()
        .filter(|result| result.status == rgaa_core::CriterionStatus::Fail)
        .map(|result| rgaa_core::Finding::new(result.criterion_id.clone()))
        .collect();
    Ok(findings)
}

/// Requests sent to the browser worker thread.
#[allow(dead_code)]
pub(crate) enum BrowserRequest {
    Navigate {
        url: String,
        allow_private_network: bool,
        allow_file_access: bool,
    },
    EvalJs(String),
    Click(String),
    Screenshot,
    A11yTree,
    PageContext,
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
    // Kept for protocol completeness; the current substrate never produces it
    // (screenshot capture is reported as `UnsupportedConfiguration`).
    #[allow(dead_code)]
    String(String),
    VecString(Vec<String>),
    AnalyzeResult(Result<AnalyzePageResult, ObscuraError>),
    Error(ObscuraError),
}

/// Browser worker that owns the browser — and its current page — on a
/// dedicated thread. The page persists across requests so sequences like
/// navigate → click → assert_state all operate on the same document.
struct BrowserWorker {
    browser: Browser,
    page: Option<Page>,
    axe_source: String,
    rx: std_mpsc::Receiver<(BrowserRequest, tokio::sync::oneshot::Sender<BrowserResponse>)>,
    runtime: tokio::runtime::Runtime,
}

impl BrowserWorker {
    fn new(
        browser: Browser,
        axe_source: String,
        rx: std_mpsc::Receiver<(BrowserRequest, tokio::sync::oneshot::Sender<BrowserResponse>)>,
    ) -> Result<Self, ObscuraError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| {
                ObscuraError::ProcessStartup(format!("failed to create tokio runtime: {e}"))
            })?;
        Ok(Self {
            browser,
            page: None,
            axe_source,
            rx,
            runtime,
        })
    }

    fn run(mut self) {
        while let Ok((request, reply)) = self.rx.recv() {
            let response = match request {
                BrowserRequest::Navigate {
                    url,
                    allow_private_network,
                    allow_file_access,
                } => self.handle_navigate(&url, allow_private_network, allow_file_access),
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
                BrowserRequest::PageContext => {
                    self.handle_page_context()
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

    fn page_mut(&mut self) -> Result<&mut Page, ObscuraError> {
        self.page.as_mut().ok_or_else(|| {
            ObscuraError::Evaluation("no page open: navigate to a URL first".into())
        })
    }

    fn handle_navigate(
        &mut self,
        url: &str,
        allow_private_network: bool,
        allow_file_access: bool,
    ) -> BrowserResponse {
        if let Err(e) =
            crate::validate_url_for_navigation(url, allow_private_network, allow_file_access)
        {
            return BrowserResponse::Error(e);
        }
        // Destructure so the async block borrows disjoint fields, never `self`.
        let Self {
            browser, page, runtime, ..
        } = &mut *self;
        let result = runtime.block_on(async {
            let mut new_page = browser.new_page().await
                .map_err(|e| ObscuraError::ProcessStartup(format!("failed to create page: {e}")))?;
            new_page.goto(url).await
                .map_err(|e| ObscuraError::Navigation(format!("navigation failed: {e}")))?;
            // Enforce the same policy on the post-navigation URL so redirects
            // to private/file targets cannot bypass the pre-navigation check.
            let final_url = new_page.url();
            crate::validate_url_for_navigation(
                &final_url,
                allow_private_network,
                allow_file_access,
            )?;
            Ok::<Page, ObscuraError>(new_page)
        });
        match result {
            Ok(new_page) => {
                *page = Some(new_page);
                BrowserResponse::Unit(())
            }
            Err(e) => BrowserResponse::Error(e),
        }
    }

    fn handle_eval_js(&mut self, expr: &str) -> BrowserResponse {
        // `Page::evaluate` is synchronous: no runtime hop needed.
        match self.page_mut() {
            Ok(page) => BrowserResponse::Value(page.evaluate(expr)),
            Err(e) => BrowserResponse::Error(e),
        }
    }

    fn handle_click(&mut self, selector: &str) -> BrowserResponse {
        // `query_selector`/`click` are synchronous: no runtime hop needed.
        let result = match self.page_mut() {
            Ok(page) => {
                if let Some(element) = page.query_selector(selector) {
                    element
                        .click()
                        .map_err(|e| ObscuraError::Evaluation(format!("click failed: {e}")))
                } else {
                    Err(ObscuraError::Evaluation(format!(
                        "element not found: {selector}"
                    )))
                }
            }
            Err(e) => Err(e),
        };
        match result {
            Ok(()) => BrowserResponse::Unit(()),
            Err(e) => BrowserResponse::Error(e),
        }
    }

    fn handle_screenshot(&mut self) -> BrowserResponse {
        // The obscura substrate exposes no screenshot capture API. Report it
        // as unsupported so callers skip screenshot evidence instead of
        // failing on placeholder bytes that are not a PNG image.
        BrowserResponse::Error(ObscuraError::UnsupportedConfiguration(
            "screenshot capture is not available in the obscura substrate".into(),
        ))
    }

    fn handle_a11y_tree(&mut self) -> BrowserResponse {
        let result = match self.page_mut() {
            Ok(page) => {
                let val = page.evaluate(
                    r#"JSON.stringify(Array.from(document.querySelectorAll('[role],[aria-label]')).map(e => ({
                        role: e.getAttribute('role') || e.tagName.toLowerCase(),
                        name: e.getAttribute('aria-label') || e.textContent.trim().slice(0, 100)
                    })))"#,
                );
                serde_json::from_str(val.as_str().unwrap_or("[]"))
                    .map_err(|e| ObscuraError::Json(e.to_string()))
                    .map(|tree: serde_json::Value| tree)
            }
            Err(e) => Err(e),
        };
        match result {
            Ok(v) => BrowserResponse::Value(v),
            Err(e) => BrowserResponse::Error(e),
        }
    }

    /// DOM extraction script returning a `PageContext`-shaped JSON string.
    /// Field names and shapes mirror `rgaa_holo::PageContext` exactly.
    const PAGE_CONTEXT_SCRIPT: &str = r#"JSON.stringify((function(){
        var q = function(s){ return Array.prototype.slice.call(document.querySelectorAll(s)); };
        return {
            title: document.title || null,
            lang: document.documentElement.lang || null,
            headings: q('h1,h2,h3,h4,h5,h6').map(function(e){
                return {level: parseInt(e.tagName.substring(1), 10) || 1, text: (e.textContent || '').trim().slice(0, 200)};
            }),
            images: q('img').map(function(e){
                return {src: e.getAttribute('src') || '', alt: e.getAttribute('alt'), has_alt: e.hasAttribute('alt'), is_decorative: e.getAttribute('alt') === ''};
            }),
            iframes: q('iframe').map(function(e){
                return {src: e.getAttribute('src'), title: e.getAttribute('title'), has_title: e.hasAttribute('title')};
            }),
            links: q('a[href]').map(function(e){
                var t = (e.textContent || '').trim();
                return {href: e.getAttribute('href') || '', text: t.slice(0, 200), has_text: t.length > 0, is_empty: t.length === 0};
            }),
            forms: q('form').map(function(f){
                var inputs = Array.prototype.slice.call(f.querySelectorAll('input,select,textarea')).map(function(i){
                    var label = i.getAttribute('aria-label') || i.getAttribute('aria-labelledby');
                    var labelled = !!label || (!!i.id && !!document.querySelector('label[for="' + i.id + '"]'));
                    return {input_type: (i.getAttribute('type') || i.tagName.toLowerCase()), has_label: labelled, aria_label: i.getAttribute('aria-label'), placeholder: i.getAttribute('placeholder')};
                });
                return {id: f.getAttribute('id'), has_labels: inputs.some(function(i){ return i.has_label; }), has_submit: !!f.querySelector('input[type=submit],button[type=submit],button:not([type])'), inputs: inputs};
            }),
            media: q('audio,video').map(function(e){
                return {media_type: e.tagName.toLowerCase(), has_captions: !!e.querySelector('track[kind=captions],track[kind=subtitles]'), has_transcript: false, has_controls: e.hasAttribute('controls')};
            }),
            navigation: q('nav a[href]').map(function(e){ return e.getAttribute('href'); }).filter(function(h, i, a){ return !!h && a.indexOf(h) === i; })
        };
    })())"#;

    fn handle_page_context(&mut self) -> BrowserResponse {
        let result = match self.page_mut() {
            Ok(page) => {
                let val = page.evaluate(Self::PAGE_CONTEXT_SCRIPT);
                serde_json::from_str(val.as_str().unwrap_or("{}"))
                    .map_err(|e| ObscuraError::Json(e.to_string()))
                    .map(|ctx: serde_json::Value| ctx)
            }
            Err(e) => Err(e),
        };
        match result {
            Ok(v) => BrowserResponse::Value(v),
            Err(e) => BrowserResponse::Error(e),
        }
    }

    fn handle_type_input(&mut self, selector: &str, text: &str) -> BrowserResponse {
        // Serialize both values as JSON string literals and interpolate them
        // directly — never inside manually-escaped JS string delimiters — so
        // adversarial quotes/backslashes cannot break out (CWE-95).
        let script = match (
            serde_json::to_string(selector),
            serde_json::to_string(text),
        ) {
            (Ok(selector_json), Ok(text_json)) => format!(
                r#"(function() {{
                    var el = document.querySelector({selector});
                    if (!el) {{ throw new Error("selector not found"); }}
                    el.focus();
                    el.value = {text};
                    el.dispatchEvent(new Event('input', {{bubbles: true}}));
                    el.dispatchEvent(new Event('change', {{bubbles: true}}));
                    return true;
                }})()"#,
                selector = selector_json,
                text = text_json
            ),
            _ => {
                return BrowserResponse::Error(ObscuraError::Json(
                    "failed to serialize type_input arguments".into(),
                ))
            }
        };
        let result = match self.page_mut() {
            Ok(page) => {
                page.evaluate(&script);
                Ok::<(), ObscuraError>(())
            }
            Err(e) => Err(e),
        };
        match result {
            Ok(()) => BrowserResponse::Unit(()),
            Err(e) => BrowserResponse::Error(e),
        }
    }

    fn handle_press_key(&mut self, key: &str) -> BrowserResponse {
        // Same JSON-literal interpolation as `handle_type_input` (CWE-95).
        let script = match serde_json::to_string(key) {
            Ok(key_json) => format!(
                r#"document.dispatchEvent(new KeyboardEvent('keydown', {{key: {key}}}));
                   document.dispatchEvent(new KeyboardEvent('keyup', {{key: {key}}}));"#,
                key = key_json
            ),
            Err(_) => {
                return BrowserResponse::Error(ObscuraError::Json(
                    "failed to serialize press_key argument".into(),
                ))
            }
        };
        let result = match self.page_mut() {
            Ok(page) => {
                page.evaluate(&script);
                Ok::<(), ObscuraError>(())
            }
            Err(e) => Err(e),
        };
        match result {
            Ok(()) => BrowserResponse::Unit(()),
            Err(e) => BrowserResponse::Error(e),
        }
    }

    fn handle_tab_order(&mut self) -> BrowserResponse {
        let result = match self.page_mut() {
            Ok(page) => {
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
                serde_json::from_str(val.as_str().unwrap_or("[]"))
                    .map_err(|e| ObscuraError::Json(e.to_string()))
                    .map(|order: Vec<serde_json::Value>| order)
            }
            Err(e) => Err(e),
        };
        match result {
            Ok(v) => BrowserResponse::VecString(
                v.into_iter().map(|v| v.to_string()).collect(),
            ),
            Err(e) => BrowserResponse::Error(e),
        }
    }

    fn handle_assert_state(&mut self, script: &str) -> BrowserResponse {
        match self.page_mut() {
            Ok(page) => BrowserResponse::Value(page.evaluate(script)),
            Err(e) => BrowserResponse::Error(e),
        }
    }

    fn handle_analyze(&mut self, request: &AnalyzeRequest) -> BrowserResponse {
        // Destructure so the async block borrows disjoint fields, never `self`.
        let Self {
            browser,
            page,
            axe_source,
            runtime,
            ..
        } = &mut *self;
        let result = runtime.block_on(async {
            request.validate_supported()?;
            let started = std::time::Instant::now();

            // Reuse the persistent page when one is open; otherwise create it.
            let page: &mut Page = match page.as_mut() {
                Some(p) => p,
                None => {
                    let new_page = browser.new_page().await
                        .map_err(|e| ObscuraError::ProcessStartup(format!("failed to create page: {e}")))?;
                    page.insert(new_page)
                }
            };

            page.goto(&request.url).await
                .map_err(|e| ObscuraError::Navigation(format!("navigation failed: {e}")))?;

            // Same redirect policy as `handle_navigate`, with this request's flags.
            let final_url = page.url();
            crate::validate_url_for_navigation(
                &final_url,
                request.config.allow_private_network,
                request.config.allow_file_access,
            )?;

            page.settle(request.config.timeout_ms).await;

            // Inject vendored axe-core
            page.evaluate(axe_source);

            // Run axe-core
            page.evaluate(
                r#"(async () => {
                    const results = await axe.run(document);
                    window.__axeResult = JSON.stringify(results.violations);
                })()"#,
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
            // `completed` reports that analysis finished — not whether it
            // found violations. A clean page is a completed run.
            let completed = true;

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

    /// Channel for tests/plumbing with no browser backend; every send fails.
    fn disconnected() -> Self {
        let (tx, rx) = std_mpsc::sync_channel::<(
            BrowserRequest,
            tokio::sync::oneshot::Sender<BrowserResponse>,
        )>(1);
        drop(rx);
        Self { tx }
    }

    pub(crate) async fn send_async(&self, request: BrowserRequest) -> Result<BrowserResponse, ObscuraError> {
        use std::sync::mpsc::TrySendError;
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        // `try_send` — never block a Tokio worker on a full queue.
        match self.tx.try_send((request, reply_tx)) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                return Err(ObscuraError::ProcessStartup(
                    "browser worker busy: request queue full".into(),
                ))
            }
            Err(TrySendError::Disconnected(_)) => {
                return Err(ObscuraError::ProcessStartup(
                    "browser worker channel closed".into(),
                ))
            }
        }
        reply_rx.await
            .map_err(|_| ObscuraError::ProcessStartup("browser worker response dropped".into()))
    }

    pub async fn navigate(&self, url: &str) -> Result<(), ObscuraError> {
        self.navigate_with_policy(url, false, false).await
    }

    pub async fn navigate_with_policy(
        &self,
        url: &str,
        allow_private_network: bool,
        allow_file_access: bool,
    ) -> Result<(), ObscuraError> {
        match self.send_async(BrowserRequest::Navigate {
            url: url.to_string(),
            allow_private_network,
            allow_file_access,
        }).await? {
            BrowserResponse::Unit(()) => Ok(()),
            BrowserResponse::Error(e) => Err(e),
            _ => Err(ObscuraError::Evaluation("unexpected response type".into())),
        }
    }

    pub async fn eval_js(&self, expr: &str) -> Result<serde_json::Value, ObscuraError> {
        match self.send_async(BrowserRequest::EvalJs(expr.to_string())).await? {
            BrowserResponse::Value(v) => Ok(v),
            BrowserResponse::Error(e) => Err(e),
            _ => Err(ObscuraError::Evaluation("unexpected response type".into())),
        }
    }

    pub async fn click(&self, selector: &str) -> Result<(), ObscuraError> {
        match self.send_async(BrowserRequest::Click(selector.to_string())).await? {
            BrowserResponse::Unit(()) => Ok(()),
            BrowserResponse::Error(e) => Err(e),
            _ => Err(ObscuraError::Evaluation("unexpected response type".into())),
        }
    }

    pub async fn screenshot(&self) -> Result<String, ObscuraError> {
        match self.send_async(BrowserRequest::Screenshot).await? {
            BrowserResponse::String(s) => Ok(s),
            BrowserResponse::Error(e) => Err(e),
            _ => Err(ObscuraError::Evaluation("unexpected response type".into())),
        }
    }

    pub async fn a11y_tree(&self) -> Result<serde_json::Value, ObscuraError> {
        match self.send_async(BrowserRequest::A11yTree).await? {
            BrowserResponse::Value(v) => Ok(v),
            BrowserResponse::Error(e) => Err(e),
            _ => Err(ObscuraError::Evaluation("unexpected response type".into())),
        }
    }

    pub async fn page_context(&self) -> Result<serde_json::Value, ObscuraError> {
        match self.send_async(BrowserRequest::PageContext).await? {
            BrowserResponse::Value(v) => Ok(v),
            BrowserResponse::Error(e) => Err(e),
            _ => Err(ObscuraError::Evaluation("unexpected response type".into())),
        }
    }

    pub async fn type_input(&self, selector: &str, text: &str) -> Result<(), ObscuraError> {
        match self.send_async(BrowserRequest::TypeInput(selector.to_string(), text.to_string())).await? {
            BrowserResponse::Unit(()) => Ok(()),
            BrowserResponse::Error(e) => Err(e),
            _ => Err(ObscuraError::Evaluation("unexpected response type".into())),
        }
    }

    pub async fn press_key(&self, key: &str) -> Result<(), ObscuraError> {
        match self.send_async(BrowserRequest::PressKey(key.to_string())).await? {
            BrowserResponse::Unit(()) => Ok(()),
            BrowserResponse::Error(e) => Err(e),
            _ => Err(ObscuraError::Evaluation("unexpected response type".into())),
        }
    }

    pub async fn tab_order(&self) -> Result<Vec<serde_json::Value>, ObscuraError> {
        match self.send_async(BrowserRequest::TabOrder).await? {
            BrowserResponse::VecString(v) => {
                v.into_iter()
                    .map(|s| serde_json::from_str(&s).map_err(|e| ObscuraError::Json(e.to_string())))
                    .collect()
            }
            BrowserResponse::Error(e) => Err(e),
            _ => Err(ObscuraError::Evaluation("unexpected response type".into())),
        }
    }

    pub async fn assert_state(&self, script: &str) -> Result<serde_json::Value, ObscuraError> {
        match self.send_async(BrowserRequest::AssertState(script.to_string())).await? {
            BrowserResponse::Value(v) => Ok(v),
            BrowserResponse::Error(e) => Err(e),
            _ => Err(ObscuraError::Evaluation("unexpected response type".into())),
        }
    }

    pub async fn analyze(&self, request: &AnalyzeRequest) -> Result<AnalyzePageResult, ObscuraError> {
        match self.send_async(BrowserRequest::Analyze(request.clone())).await? {
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
        verify_axe_bundle()?;
        let browser = Browser::builder()
            .stealth(true)
            .build()
            .map_err(|e| ObscuraError::ProcessStartup(format!("failed to create browser: {e}")))?;

        let axe_source = AXE_SOURCE.to_string();

        let (tx, rx) = std_mpsc::sync_channel(1);

        thread::spawn(move || match BrowserWorker::new(browser, axe_source, rx) {
            Ok(worker) => worker.run(),
            Err(e) => tracing::error!(error = %e, "browser worker failed to start"),
        });

        let handle = BrowserHandle::new(tx);
        Ok(Self { handle })
    }

    /// Handle with no browser backend; every operation fails fast.
    pub fn new_disconnected() -> Self {
        Self {
            handle: BrowserHandle::disconnected(),
        }
    }

    pub async fn analyze(&self, request: &AnalyzeRequest) -> Result<AnalyzePageResult, ObscuraError> {
        self.handle
            .send_async(BrowserRequest::Analyze(request.clone()))
            .await
            .map_err(|e| ObscuraError::Evaluation(format!("channel error: {e}")))?
            .into_analyze_result()
    }

    pub async fn navigate_with_policy(
        &self,
        url: &str,
        allow_private_network: bool,
        allow_file_access: bool,
    ) -> Result<(), ObscuraError> {
        self.handle
            .navigate_with_policy(url, allow_private_network, allow_file_access)
            .await
    }

    pub async fn page_context(&self) -> Result<serde_json::Value, ObscuraError> {
        self.handle.page_context().await
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
                let resp = self.handle.send_async(BrowserRequest::Navigate {
                    url: url.clone(),
                    // Guided tests carry no policy flags; enforce default-deny.
                    allow_private_network: false,
                    allow_file_access: false,
                }).await?;
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
                        // `handle_a11y_tree` returns an array value directly.
                        let refs: Vec<String> = v
                            .as_array()
                            .map(|items| items.iter().map(|item| item.to_string()).collect())
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
                    // No capture API in this substrate: skip screenshot
                    // evidence (missing-requirement check flags it downstream).
                    BrowserResponse::Error(ObscuraError::UnsupportedConfiguration(_)) => {
                        Ok(GuidedObservation::default())
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

    #[tokio::test]
    async fn test_native_creation() {
        // Creation requires a browser and network access; only assert it terminates.
        let _ = ObscuraNative::new().await;
    }

    #[test]
    fn test_axe_bundle_integrity() {
        verify_axe_bundle().expect("vendored axe-core must match its pinned hash");
        assert!(
            AXE_SOURCE.contains("axe v4.9.1"),
            "vendored bundle must be axe-core 4.9.1"
        );
    }
}
