// rgaa-obscura: Browser automation via native Obscura library
// Browser runs on a dedicated thread; bridge is Send+Sync via channels.

pub mod config;
pub mod evidence;
pub mod guided;
pub mod results;
pub mod native;

pub use config::{
    AdvancedRulePolicy, AnalyzeConfig, AnalyzeRequest, CookieReference, CookieSameSite,
    NeedsReviewPolicy, PreScanAction, ScreenshotConfig, ScreenshotFormat, ScreenshotPolicy, Viewport,
    WaitForState, MAX_WAITFOR_TIMEOUT_MS,
};
pub use evidence::{EvidenceArtifact, EvidenceRef, EvidenceStore};
pub use guided::{
    is_stable_accessibility_reference, GuidedAction, GuidedExecutor, GuidedObservation,
    GuidedRunResult, GuidedStep, GuidedTest, TerminationReason,
};
pub use results::{AnalyzePageResult, IgtElement, IgtIssue, IgtResult, IgtResults, ObscuraError};
pub use native::{ObscuraNative, BrowserHandle};

/// High-level async wrapper around the native Obscura library.
/// Send+Sync because the browser runs on a dedicated thread.
pub struct ObscuraBridge {
    native: ObscuraNative,
}

impl ObscuraBridge {
    pub async fn new() -> Result<Self, ObscuraError> {
        let native = ObscuraNative::new().await?;
        Ok(Self { native })
    }

    /// Validate a URL against security policy before navigation.
    /// Checks the file:// scheme, literal private-network hosts, and — for
    /// http(s) — the DNS-resolved destination addresses, so a public hostname
    /// resolving to a private address cannot bypass the default-deny policy.
    fn validate_url_security(url: &str, config: &AnalyzeConfig) -> Result<(), ObscuraError> {
        validate_url_for_navigation(
            url,
            config.allow_private_network,
            config.allow_file_access,
        )
    }

    /// Create a bridge from environment variable `RGAA_OBSCURA_BIN`.
    pub async fn from_env_async() -> Result<Self, ObscuraError> {
        Self::new().await
    }

    /// Inert bridge with no browser backend; every operation fails fast.
    /// For unit tests that exercise tool plumbing without a browser.
    #[must_use]
    pub fn new_disconnected() -> Self {
        Self {
            native: ObscuraNative::new_disconnected(),
        }
    }

    pub async fn analyze(&self, request: &AnalyzeRequest) -> Result<AnalyzePageResult, ObscuraError> {
        Self::validate_url_security(&request.url, &request.config)?;
        self.native.analyze(request).await
    }

    pub async fn run_guided_test(&self, test: &GuidedTest) -> Result<GuidedRunResult, ObscuraError> {
        self.native.run_guided_test(test).await
    }

    /// Extract page context (title, headings, images, links, forms, media, navigation).
    /// Returns a `PageContext`-shaped payload built from the live DOM — not
    /// the analysis result envelope — so deserialization into `PageContext`
    /// preserves page content for NA detection and AI-assisted evaluation.
    pub async fn extract_page_context(&self, url: &str) -> Result<serde_json::Value, ObscuraError> {
        let config = AnalyzeConfig {
            viewport: Viewport { width: 1280, height: 720 },
            ..Default::default()
        };
        Self::validate_url_security(url, &config)?;
        self.native.navigate_with_policy(url, false, false).await?;
        self.native.page_context().await
    }

    /// Run axe-core analysis on a URL, returning violations JSON string
    pub async fn run_axe(&self, url: &str) -> Result<String, ObscuraError> {
        let config = AnalyzeConfig::default();
        Self::validate_url_security(url, &config)?;
        let request = AnalyzeRequest {
            url: url.to_string(),
            config,
        };
        let result = self.native.analyze(&request).await?;
        serde_json::to_string(&result.findings).map_err(|e| ObscuraError::Json(e.to_string()))
    }

    /// Run axe-core analysis on multiple URLs
    pub async fn run_axe_batch(&self, urls: &[String], _concurrency: usize) -> Result<Vec<(String, String)>, ObscuraError> {
        let mut results = Vec::with_capacity(urls.len());
        for url in urls {
            let axe_json = self.run_axe(url).await?;
            results.push((url.clone(), axe_json));
        }
        Ok(results)
    }

    /// Extract page context for multiple URLs
    pub async fn extract_page_context_batch(&self, urls: &[String], _concurrency: usize) -> Result<Vec<(String, serde_json::Value)>, ObscuraError> {
        let mut results = Vec::with_capacity(urls.len());
        for url in urls {
            let ctx = self.extract_page_context(url).await?;
            results.push((url.clone(), ctx));
        }
        Ok(results)
    }

    /// Run gap-fix JS snippets against a URL, returning per-criterion results
    pub async fn run_gap_fix(&self, _url: &str, snippets: &std::collections::HashMap<String, &str>) -> Result<std::collections::HashMap<String, serde_json::Value>, ObscuraError> {
        let mut results = std::collections::HashMap::new();
        for (criterion_id, script) in snippets {
            // Execute each snippet via eval_js and parse the result
            match self.native.handle.eval_js(script).await {
                Ok(value) => {
                    results.insert(criterion_id.clone(), value);
                }
                Err(e) => {
                    tracing::warn!(criterion_id, error = %e, "gap-fix snippet failed");
                    results.insert(criterion_id.clone(), serde_json::json!({"pass": false, "details": e.to_string()}));
                }
            }
        }
        Ok(results)
    }

    // --- BrowserSession API methods (delegated to browser worker thread) ---

    /// Navigate to a URL
    pub async fn navigate(&self, url: &str) -> Result<(), ObscuraError> {
        self.native.handle.navigate(url).await
    }

    /// Evaluate JavaScript in the page
    pub async fn eval_js(&self, expression: &str) -> Result<serde_json::Value, ObscuraError> {
        self.native.handle.eval_js(expression).await
    }

    /// Click an element by CSS selector
    pub async fn click_element(&self, _url: &str, selector: &str) -> Result<(), ObscuraError> {
        self.native.handle.click(selector).await
    }

    /// Take a screenshot (returns base64-encoded PNG)
    pub async fn screenshot(&self, _url: &str) -> Result<String, ObscuraError> {
        self.native.handle.screenshot().await
    }

    /// Get the accessibility tree
    pub async fn get_accessibility_tree(&self, _url: &str) -> Result<serde_json::Value, ObscuraError> {
        self.native.handle.a11y_tree().await
    }

    /// Type text into an input element
    pub async fn type_input(&self, _url: &str, selector: &str, text: &str) -> Result<(), ObscuraError> {
        self.native.handle.type_input(selector, text).await
    }

    /// Press a keyboard key
    pub async fn press_key(&self, _url: &str, key: &str) -> Result<(), ObscuraError> {
        self.native.handle.press_key(key).await
    }

    /// Get the tab order of focusable elements
    pub async fn get_tab_order(&self, _url: &str) -> Result<Vec<serde_json::Value>, ObscuraError> {
        self.native.handle.tab_order().await
    }

    /// Assert page state by evaluating a JavaScript predicate
    pub async fn assert_state(&self, _url: &str, script: &str) -> Result<serde_json::Value, ObscuraError> {
        self.native.handle.assert_state(script).await
    }

    #[allow(dead_code)]
    fn classify_error(error: String) -> ObscuraError {
        let lower = error.to_ascii_lowercase();
        if lower.contains("timed out") || lower.contains("timeout") {
            ObscuraError::Timeout(error)
        } else if lower.contains("navigation") || lower.contains("load") {
            ObscuraError::Navigation(error)
        } else if lower.contains("json") || lower.contains("result") {
            ObscuraError::Json(error)
        } else if lower.contains("screenshot") || lower.contains("evidence") {
            ObscuraError::Evidence(error)
        } else if lower.contains("missing secret") || lower.contains("policy") {
            ObscuraError::PolicyDenied(error)
        } else if lower.contains("unsupported") {
            ObscuraError::UnsupportedConfiguration(error)
        } else if lower.contains("failed to spawn") || lower.contains("process") {
            ObscuraError::ProcessStartup(error)
        } else if lower.contains("network") || lower.contains("connect") {
            ObscuraError::Network(error)
        } else {
            ObscuraError::Evaluation(error)
        }
    }
}

/// Validate a URL against the navigation security policy.
///
/// Checks the file:// scheme, literal private-network hosts, and — for
/// http(s) — the DNS-resolved destination addresses, so a public hostname
/// resolving to a private address cannot bypass the default-deny policy.
/// The same check must be applied to the post-navigation URL to cover
/// redirects at the browser network boundary.
///
/// DNS resolution failure fails open (the literal-host check still applies)
/// so offline or misconfigured DNS does not hard-fail static validation.
pub(crate) fn validate_url_for_navigation(
    url: &str,
    allow_private_network: bool,
    allow_file_access: bool,
) -> Result<(), ObscuraError> {
    use std::net::ToSocketAddrs;

    let parsed = reqwest::Url::parse(url)
        .map_err(|e| ObscuraError::Validation(format!("invalid URL: {e}")))?;

    if parsed.scheme() == "file" && !allow_file_access {
        return Err(ObscuraError::PolicyDenied(
            "file:// URLs require allow_file_access=true in config".into(),
        ));
    }

    if matches!(parsed.scheme(), "http" | "https") {
        if let Some(host) = parsed.host_str() {
            if is_private_network(host) && !allow_private_network {
                return Err(ObscuraError::PolicyDenied(format!(
                    "private/intranet host '{host}' requires allow_private_network=true in config"
                )));
            }
            // DNS-resolved addresses: a public name pointing at a private
            // address must not bypass the policy.
            if !allow_private_network && host.parse::<std::net::IpAddr>().is_err() {
                if let Ok(addrs) = (host, 443u16).to_socket_addrs() {
                    for addr in addrs {
                        if is_private_ip(&addr.ip()) {
                            return Err(ObscuraError::PolicyDenied(format!(
                                "host '{host}' resolves to private address {}",
                                addr.ip()
                            )));
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Check if an IP address is in a private/intranet/non-public range.
fn is_private_ip(ip: &std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_link_local()
                || v4.is_private()
                || v4.is_unspecified()
                || (v4.octets()[0] == 100 && v4.octets()[1] & 0xC0 == 0x40)  // 100.64.0.0/10 (CGNAT)
                || (v4.octets()[0] == 192 && v4.octets()[1] == 0 && v4.octets()[2] == 0)  // 192.0.0.0/24
                || (v4.octets()[0] == 192 && v4.octets()[1] == 0 && v4.octets()[2] == 2)  // 192.0.2.0/24 (TEST-NET-1)
                || (v4.octets()[0] == 198 && v4.octets()[1] == 51 && v4.octets()[2] == 100)  // 198.51.100.0/24 (TEST-NET-2)
                || (v4.octets()[0] == 203 && v4.octets()[1] == 0 && v4.octets()[2] == 113)  // 203.0.113.0/24 (TEST-NET-3)
                || v4.octets()[0] >= 224  // multicast + broadcast
        }
        std::net::IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_unicast_link_local()
                || v6.segments()[0] == 0xfe80  // link-local
                || v6.segments()[0] & 0xFE00 == 0xFC00  // ULA (fc00::/7)
        }
    }
}

/// Check if a hostname resolves to a private/intranet network range.
/// Returns true for RFC 1918, link-local, loopback, and other non-public ranges.
fn is_private_network(host: &str) -> bool {
    // Check for IP addresses first
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        return is_private_ip(&ip);
    }

    // Check for private/intranet hostnames
    let lower = host.to_ascii_lowercase();
    lower == "localhost"
        || lower.ends_with(".local")
        || lower.ends_with(".internal")
        || lower.ends_with(".intranet")
        || lower.ends_with(".localdomain")
        || lower == "0.0.0.0"
        || lower == "::"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires Obscura binary"]
    async fn test_obscura_bridge_axe() {
        let bridge = ObscuraBridge::new().await.unwrap();
        let result = bridge.run_axe("https://example.com").await;
        assert!(result.is_ok(), "Failed to run axe: {:?}", result.err());
        let ax = result.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&ax).expect("axe result must be parseable JSON");
        assert!(parsed.is_array(), "axe result must be a JSON array");
    }

    #[tokio::test]
    #[ignore = "requires Obscura binary"]
    async fn test_obscura_bridge_extract_page_context() {
        let bridge = ObscuraBridge::new().await.unwrap();
        let result = bridge.extract_page_context("https://example.com").await;
        assert!(result.is_ok(), "Failed to extract page context: {:?}", result.err());
        let context = result.unwrap();
        assert!(context.get("title").is_some(), "Missing title in page context");
    }

    #[tokio::test]
    #[ignore = "requires Obscura binary"]
    async fn test_obscura_bridge_axe_batch() {
        let bridge = ObscuraBridge::new().await.unwrap();
        let urls = vec![
            "https://example.com".to_string(),
            "https://example.org".to_string(),
        ];
        let results = bridge.run_axe_batch(&urls, 2).await;
        assert!(results.is_ok(), "Failed to run axe batch: {:?}", results.err());
        let results = results.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_private_network_detection() {
        assert!(is_private_network("192.168.1.1"));
        assert!(is_private_network("10.0.0.1"));
        assert!(is_private_network("172.16.0.1"));
        assert!(is_private_network("127.0.0.1"));
        assert!(is_private_network("localhost"));
        assert!(is_private_network("myhost.local"));
        assert!(is_private_network("100.64.0.1"));
        assert!(is_private_network("192.0.0.1"));
        assert!(is_private_network("198.51.100.1"));
        assert!(is_private_network("203.0.113.1"));

        assert!(!is_private_network("8.8.8.8"));
        assert!(!is_private_network("1.1.1.1"));
        assert!(!is_private_network("example.com"));
        assert!(!is_private_network("google.com"));
    }

    #[test]
    fn test_url_security_validation() {
        let config = AnalyzeConfig::default();

        // Public URLs should pass
        assert!(ObscuraBridge::validate_url_security("https://example.com", &config).is_ok());

        // File URLs should fail without flag
        assert!(ObscuraBridge::validate_url_security("file:///tmp/test.html", &config).is_err());

        // File URLs should pass with flag
        let mut config_file = config.clone();
        config_file.allow_file_access = true;
        assert!(ObscuraBridge::validate_url_security("file:///tmp/test.html", &config_file).is_ok());

        // Private IPs should fail without flag
        assert!(ObscuraBridge::validate_url_security("http://192.168.1.1/", &config).is_err());
        assert!(ObscuraBridge::validate_url_security("http://10.0.0.1/", &config).is_err());
        assert!(ObscuraBridge::validate_url_security("http://localhost/", &config).is_err());

        // Private IPs should pass with flag
        let mut config_net = config.clone();
        config_net.allow_private_network = true;
        assert!(ObscuraBridge::validate_url_security("http://192.168.1.1/", &config_net).is_ok());
        assert!(ObscuraBridge::validate_url_security("http://localhost/", &config_net).is_ok());
    }

    #[test]
    fn dns_resolution_cannot_bypass_private_policy() {
        // localhost resolves to loopback: denied by default, allowed with flag.
        let config = AnalyzeConfig::default();
        assert!(ObscuraBridge::validate_url_security("http://localhost/", &config).is_err());
        // Unresolvable hosts fail open on the DNS leg (literal check still applies).
        assert!(ObscuraBridge::validate_url_security(
            "https://this-host-does-not-exist.invalid/",
            &AnalyzeConfig::default()
        )
        .is_ok());
    }

    /// The `PageContext` worker payload must deserialize into
    /// `rgaa_holo::PageContext` — otherwise the orchestrator silently falls
    /// back to an empty context and NA detection runs on nothing.
    #[test]
    fn page_context_payload_matches_holo_shape() {
        let payload = serde_json::json!({
            "title": "Example",
            "lang": "fr",
            "headings": [{"level": 1, "text": "Bonjour"}],
            "images": [{"src": "/a.png", "alt": "A", "has_alt": true, "is_decorative": false}],
            "iframes": [{"src": null, "title": null, "has_title": false}],
            "links": [{"href": "/x", "text": "X", "has_text": true, "is_empty": false}],
            "forms": [{
                "id": null, "has_labels": true, "has_submit": false,
                "inputs": [{"input_type": "text", "has_label": true, "aria_label": null, "placeholder": null}]
            }],
            "media": [{"media_type": "video", "has_captions": false, "has_transcript": false, "has_controls": true}],
            "navigation": ["/x"]
        });
        let ctx: rgaa_holo::PageContext =
            serde_json::from_value(payload).expect("worker payload must match PageContext");
        assert_eq!(ctx.title.as_deref(), Some("Example"));
        assert_eq!(ctx.headings.len(), 1);
    }
}
