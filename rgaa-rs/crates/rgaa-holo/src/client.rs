use base64::Engine;
use reqwest::Client;
use rgaa_core::RgaaError;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, info, warn};

const API_URL: &str = "https://api.hcompany.ai/v1/chat/completions";
const MODEL: &str = "holo3-1-35b-a3b";
const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 500;

const SYSTEM_PROMPT: &str = "Tu es un expert en accessibilité web RGAA 4.1.2 (Référentiel Général d'Amélioration de l'Accessibilité). Tu évalues des critères d'accessibilité sur des pages web.

Tu dois retourner un JSON avec les champs suivants :
- \"verdict\": \"pass\", \"fail\", ou \"na\" (non applicable)
- \"confidence\": un nombre entre 0.0 et 1.0 indiquant ton niveau de confiance
- \"justification\": une explication détaillée en français du raisonnement

Ne retourne QUE le JSON, sans texte additionnel.";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoloResponse {
    pub verdict: String,
    pub confidence: f64,
    pub justification: String,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f64,
    max_tokens: u32,
}

#[derive(Clone)]
pub struct HoloClient {
    api_key: String,
    base_url: String,
    http_client: Client,
}

impl std::fmt::Debug for HoloClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HoloClient")
            .field("api_key", &"[redacted]")
            .field("base_url", &self.base_url)
            .field("http_client", &"<reqwest::Client>")
            .finish()
    }
}

impl HoloClient {
    /// Creates a new HoloClient with the given API key.
    ///
    /// # Errors
    ///
    /// Returns `Err(RgaaError::Holo3)` if the HTTP client cannot be built
    /// (e.g., TLS initialization failure).
    pub fn new(api_key: String) -> Result<Self, RgaaError> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| RgaaError::Holo3(e.to_string()))?;

        Ok(Self {
            api_key,
            base_url: API_URL.to_string(),
            http_client,
        })
    }

    /// Override the API base URL. Primarily used by tests against a mock server.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Sends a text-only evaluation prompt to the Holo3 API.
    ///
    /// Wraps the prompt with the system prompt and sends it as a chat completion.
    /// Retries up to `MAX_RETRIES` times with exponential backoff on transient
    /// failures (HTTP 429, network errors).
    ///
    /// # Arguments
    ///
    /// * `prompt` - The evaluation prompt describing the accessibility criterion.
    ///
    /// # Returns
    ///
    /// A parsed `HoloResponse` containing the verdict, confidence, and justification.
    ///
    /// # Errors
    ///
    /// Returns `Err(String)` if all retry attempts fail due to network errors,
    /// API errors, or invalid response parsing.
    pub async fn evaluate(&self, prompt: &str) -> Result<HoloResponse, RgaaError> {
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: serde_json::Value::String(SYSTEM_PROMPT.to_string()),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::Value::String(prompt.to_string()),
            },
        ];
        self.evaluate_with_messages(messages).await
    }

    /// Evaluate a prompt with an optional image (base64 PNG).
    /// When image is Some, sends a multimodal content array.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The evaluation prompt describing the accessibility criterion.
    /// * `image_base64` - Optional base64-encoded PNG image for visual evaluation.
    ///
    /// # Returns
    ///
    /// A parsed `HoloResponse` containing the verdict, confidence, and justification.
    ///
    /// # Errors
    ///
    /// Returns `Err(String)` if:
    /// - The provided `image_base64` is not valid base64 data.
    /// - All retry attempts fail due to network errors, API errors, or invalid response parsing.
    pub async fn evaluate_multimodal(
        &self,
        prompt: &str,
        image_base64: Option<&str>,
    ) -> Result<HoloResponse, RgaaError> {
        let mut messages = vec![ChatMessage {
            role: "system".to_string(),
            content: serde_json::Value::String(SYSTEM_PROMPT.to_string()),
        }];

        if let Some(img) = image_base64 {
            // Validate base64 without retaining the full decoded buffer.
            // Use a small reusable buffer to check validity; the actual image
            // bytes are sent as base64 in the request and decoded by the API.
            let mut buf = vec![0u8; 64];
            base64::engine::general_purpose::STANDARD
                .decode_slice(img, &mut buf)
                .map_err(|e| RgaaError::Holo3(format!("invalid base64 image data: {e}")))?;

            let content = serde_json::json!([
                {"type": "text", "text": prompt},
                {"type": "image_url", "image_url": {"url": format!("data:image/png;base64,{}", img)}}
            ]);
            messages.push(ChatMessage {
                role: "user".to_string(),
                content,
            });
        } else {
            messages.push(ChatMessage {
                role: "user".to_string(),
                content: serde_json::Value::String(prompt.to_string()),
            });
        }

        self.evaluate_with_messages(messages).await
    }

    async fn evaluate_with_messages(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<HoloResponse, RgaaError> {
        let request = ChatRequest {
            model: MODEL.to_string(),
            messages,
            temperature: 0.1,
            max_tokens: 512,
        };

        let mut last_error = String::new();

        for attempt in 1..=MAX_RETRIES {
            info!(attempt, max_retries = MAX_RETRIES, "Calling Holo3 API");

            match self
                .http_client
                .post(&self.base_url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await
            {
                Ok(response) => {
                    let status = response.status();

                    if status.is_success() {
                        match response.text().await {
                            Ok(text) => {
                                if let Some(parsed) = Self::extract_json(&text) {
                                    info!("Successfully parsed Holo3 response");
                                    return Ok(parsed);
                                } else {
                                    warn!("Failed to extract JSON from response");
                                    last_error = "Failed to parse response JSON".to_string();
                                }
                            }
                            Err(e) => {
                                error!("Failed to read response body: {}", e);
                                last_error = format!("Response read error: {}", e);
                            }
                        }
                    } else if status.as_u16() == 429 {
                        let backoff = INITIAL_BACKOFF_MS * 2u64.pow(attempt - 1);
                        let sleep_ms = backoff + Self::jitter_for(backoff);
                        warn!(attempt, backoff_ms = sleep_ms, "Rate limited, backing off");
                        tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
                        last_error = "Rate limited (429)".to_string();
                    } else {
                        let body = response.text().await.unwrap_or_default();
                        error!(
                            status = status.as_u16(),
                            body = %body,
                            "API error"
                        );
                        last_error = format!("API error {}: {}", status.as_u16(), body);
                    }
                }
                Err(e) => {
                    error!("Request failed: {}", e);
                    last_error = format!("Request error: {}", e);

                    if attempt < MAX_RETRIES {
                        let backoff = INITIAL_BACKOFF_MS * 2u64.pow(attempt - 1);
                        tokio::time::sleep(Duration::from_millis(
                            backoff + Self::jitter_for(backoff),
                        ))
                        .await;
                    }
                }
            }
        }

        Err(RgaaError::Holo3(format!(
            "Failed after {} attempts. Last error: {}",
            MAX_RETRIES, last_error
        )))
    }

    /// Attempts to extract a `HoloResponse` from raw text.
    ///
    /// Tries three strategies in order:
    /// 1. Direct JSON parsing of the entire text.
    /// 2. Extraction from a markdown code block (` ```json ... ``` `).
    /// 3. Regex extraction of a JSON object containing the expected fields.
    ///
    /// # Arguments
    ///
    /// * `text` - The raw response text from the API.
    ///
    /// # Returns
    ///
    /// `Some(HoloResponse)` if any extraction strategy succeeds, `None` otherwise.
    pub fn extract_json(text: &str) -> Option<HoloResponse> {
        if let Ok(response) = serde_json::from_str::<HoloResponse>(text) {
            return Some(response);
        }

        if let Some(json_str) = Self::extract_from_code_block(text) {
            if let Ok(response) = serde_json::from_str::<HoloResponse>(&json_str) {
                return Some(response);
            }
        }

        if let Some(json_str) = Self::extract_with_regex(text) {
            if let Ok(response) = serde_json::from_str::<HoloResponse>(&json_str) {
                return Some(response);
            }
        }

        None
    }

    /// Cheap, dependency-free jitter (0..=backoff/2) to spread 429 retries and
    /// avoid a thundering herd when many evaluations run concurrently.
    fn jitter_for(backoff: u64) -> u64 {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64)
            .unwrap_or(0);
        seed % (backoff / 2 + 1)
    }

    /// Extracts JSON content from a markdown code block.
    fn extract_from_code_block(text: &str) -> Option<String> {
        let patterns = ["```json\n", "```\n", "```json\r\n", "```\r\n"];

        for start_pattern in &patterns {
            if let Some(start) = text.find(start_pattern) {
                let json_start = start + start_pattern.len();
                if let Some(end) = text[json_start..].find("```") {
                    return Some(text[json_start..json_start + end].trim().to_string());
                }
            }
        }

        None
    }

    /// Extracts a JSON object containing the expected fields using regex.
    fn extract_with_regex(text: &str) -> Option<String> {
        let pattern = r#"\{[^{}]*"verdict"[^{}]*"confidence"[^{}]*"justification"[^{}]*\}"#;
        let re = regex_lite::Regex::new(pattern).ok()?;
        re.find(text).map(|m| m.as_str().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_direct() {
        let json = r#"{"verdict": "pass", "confidence": 0.95, "justification": "Test"}"#;
        let result = HoloClient::extract_json(json);
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.verdict, "pass");
        assert_eq!(r.confidence, 0.95);
    }

    #[test]
    fn test_extract_json_from_code_block() {
        let text = r#"Here is the result:
```json
{"verdict": "fail", "confidence": 0.8, "justification": "Missing alt text"}
```
"#;
        let result = HoloClient::extract_json(text);
        assert!(result.is_some());
        assert_eq!(result.unwrap().verdict, "fail");
    }

    #[test]
    fn test_extract_json_from_regex() {
        let text = "The verdict is {\"verdict\": \"na\", \"confidence\": 1.0, \"justification\": \"N/A\"} for this criterion.";
        let result = HoloClient::extract_json(text);
        assert!(result.is_some());
        assert_eq!(result.unwrap().verdict, "na");
    }

    #[test]
    fn test_extract_json_invalid() {
        let text = "No JSON here";
        let result = HoloClient::extract_json(text);
        assert!(result.is_none());
    }

    /// Minimal HTTP server that answers every request with a fixed Holo3-style
    /// JSON body. Used to validate parsing and concurrent execution without
    /// touching the real API.
    fn spawn_mock_server(
        body: &'static str,
    ) -> (String, std::sync::Arc<std::thread::JoinHandle<()>>) {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(mut s) => {
                        std::thread::spawn(move || {
                            let mut buf = [0u8; 4096];
                            let _ = s.read(&mut buf);
                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                body.len(),
                                body
                            );
                            let _ = s.write_all(response.as_bytes());
                            let _ = s.flush();
                        });
                    }
                    Err(_) => break,
                }
            }
        });
        (addr.to_string(), std::sync::Arc::new(handle))
    }

    #[tokio::test]
    async fn test_evaluate_parses_via_mock_server() {
        let (addr, handle) =
            spawn_mock_server(r#"{"verdict":"pass","confidence":0.9,"justification":"ok"}"#);
        let client = HoloClient::new("test-key".to_string())
            .unwrap()
            .with_base_url(format!("http://{addr}"));

        let res = client.evaluate("prompt").await;
        assert!(res.is_ok(), "expected Ok, got {:?}", res.err());
        let r = res.unwrap();
        assert_eq!(r.verdict, "pass");
        assert_eq!(r.confidence, 0.9);
        drop(handle);
    }

    #[tokio::test]
    async fn test_evaluate_multimodal_text_only() {
        let (addr, handle) =
            spawn_mock_server(r#"{"verdict":"pass","confidence":0.9,"justification":"ok"}"#);
        let client = HoloClient::new("test-key".to_string())
            .unwrap()
            .with_base_url(format!("http://{addr}"));

        let res = client.evaluate_multimodal("prompt", None).await;
        assert!(res.is_ok(), "expected Ok, got {:?}", res.err());
        let r = res.unwrap();
        assert_eq!(r.verdict, "pass");
        drop(handle);
    }

    #[tokio::test]
    async fn test_evaluate_multimodal_invalid_base64() {
        let client = HoloClient::new("test-key".to_string()).unwrap();
        let result = client
            .evaluate_multimodal("test prompt", Some("not-valid-base64!!!"))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("base64"));
    }

    #[tokio::test]
    async fn test_evaluate_multimodal_with_image() {
        let (addr, handle) =
            spawn_mock_server(r#"{"verdict":"fail","confidence":0.85,"justification":"no alt"}"#);
        let client = HoloClient::new("test-key".to_string())
            .unwrap()
            .with_base_url(format!("http://{addr}"));

        let fake_b64 = "iVBORw0KGgoAAAANSUhEUg==";
        let res = client
            .evaluate_multimodal("describe this screenshot", Some(fake_b64))
            .await;
        assert!(res.is_ok(), "expected Ok, got {:?}", res.err());
        let r = res.unwrap();
        assert_eq!(r.verdict, "fail");
        assert_eq!(r.confidence, 0.85);
        drop(handle);
    }

    #[tokio::test]
    async fn test_evaluate_concurrent_send() {
        let (addr, handle) =
            spawn_mock_server(r#"{"verdict":"na","confidence":1.0,"justification":"n/a"}"#);
        let client = std::sync::Arc::new(
            HoloClient::new("test-key".to_string())
                .unwrap()
                .with_base_url(format!("http://{addr}")),
        );

        let start = std::time::Instant::now();
        let mut set = tokio::task::JoinSet::new();
        for i in 0..10u32 {
            let c = std::sync::Arc::clone(&client);
            set.spawn(async move {
                let r = c.evaluate(&format!("prompt-{i}")).await;
                (i, r)
            });
        }

        let mut ok = 0;
        while let Some(joined) = set.join_next().await {
            let (_i, r) = joined.expect("Holo3 task panicked");
            if r.is_ok() {
                ok += 1;
            }
        }
        let elapsed = start.elapsed();

        assert_eq!(ok, 10, "all concurrent calls should succeed");
        assert!(
            elapsed.as_secs() < 10,
            "concurrent calls unexpectedly slow: {elapsed:?}"
        );
        drop(handle);
    }
}
