//! OpenAI-compatible chat-completions transport shared by every backend.
//!
//! Holo3 and Ollama speak the same wire format (`POST /v1/chat/completions`,
//! `messages[]`, `choices[0].message.content`), so the request loop, retry,
//! backoff and response extraction live here once.

use base64::Engine;
use reqwest::Client;
use rgaa_core::RgaaError;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, info, warn};

pub(crate) const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 500;
const CIRCUIT_BREAKER_THRESHOLD: u32 = 5;

pub(crate) const SYSTEM_PROMPT: &str = "Tu es un expert en accessibilité web RGAA 4.1.2 (Référentiel Général d'Amélioration de l'Accessibilité). Tu évalues des critères d'accessibilité sur des pages web.

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
pub(crate) struct ChatMessage {
    pub role: String,
    pub content: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage>,
    temperature: f64,
    max_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ChatEnvelope {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct ChatChoiceMessage {
    content: Option<String>,
}

#[derive(Clone)]
pub(crate) struct ChatTransport {
    /// Short backend label used in logs and error messages (`"holo3"`, `"ollama"`).
    pub label: &'static str,
    pub endpoint: String,
    pub model: String,
    pub api_key: Option<String>,
    http_client: Client,
}

impl std::fmt::Debug for ChatTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatTransport")
            .field("label", &self.label)
            .field("endpoint", &self.endpoint)
            .field("model", &self.model)
            .field("api_key", &self.api_key.as_ref().map(|_| "[redacted]"))
            .finish()
    }
}

impl ChatTransport {
    pub(crate) fn new(
        label: &'static str,
        endpoint: impl Into<String>,
        model: impl Into<String>,
        api_key: Option<String>,
        timeout: Duration,
    ) -> Result<Self, RgaaError> {
        let http_client =
            Client::builder()
                .timeout(timeout)
                .build()
                .map_err(|e| RgaaError::Llm {
                    message: format!("{label}: HTTP client init failed: {e}"),
                    code: Some("HTTP_CLIENT_INIT".to_string()),
                })?;
        Ok(Self {
            label,
            endpoint: endpoint.into(),
            model: model.into(),
            api_key,
            http_client,
        })
    }

    pub(crate) fn text_messages(prompt: &str) -> Vec<ChatMessage> {
        vec![
            ChatMessage {
                role: "system".to_string(),
                content: serde_json::Value::String(SYSTEM_PROMPT.to_string()),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::Value::String(prompt.to_string()),
            },
        ]
    }

    pub(crate) fn multimodal_messages(
        prompt: &str,
        image_base64: Option<&str>,
    ) -> Result<Vec<ChatMessage>, RgaaError> {
        let Some(img) = image_base64 else {
            return Ok(Self::text_messages(prompt));
        };
        // Validate base64 without retaining the full decoded buffer; the image
        // is forwarded as base64 and decoded by the model server.
        let mut buf = vec![0u8; 64];
        base64::engine::general_purpose::STANDARD
            .decode_slice(img, &mut buf)
            .map_err(|e| RgaaError::Llm {
                message: format!("invalid base64 image data: {e}"),
                code: Some("INVALID_IMAGE".to_string()),
            })?;

        Ok(vec![
            ChatMessage {
                role: "system".to_string(),
                content: serde_json::Value::String(SYSTEM_PROMPT.to_string()),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!([
                    {"type": "text", "text": prompt},
                    {"type": "image_url", "image_url": {"url": format!("data:image/png;base64,{img}")}}
                ]),
            },
        ])
    }

    /// Sends the messages as a chat completion, retrying with exponential backoff
    /// on HTTP 429 and network errors, up to [`MAX_RETRIES`].
    pub(crate) async fn complete(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<HoloResponse, RgaaError> {
        let request = ChatRequest {
            model: &self.model,
            messages,
            temperature: 0.1,
            max_tokens: 512,
        };
        let label = self.label;

        let mut last_error = RgaaError::Llm {
            message: format!("{label}: no attempt made"),
            code: None,
        };
        let mut consecutive_failures = 0u32;

        for attempt in 1..=MAX_RETRIES {
            if consecutive_failures >= CIRCUIT_BREAKER_THRESHOLD {
                warn!(
                    backend = label,
                    consecutive_failures, "Circuit breaker open"
                );
                return Err(RgaaError::Llm {
                    message: format!(
                        "{label}: circuit breaker open: too many consecutive failures"
                    ),
                    code: Some("CIRCUIT_BREAKER_OPEN".to_string()),
                });
            }

            info!(backend = label, model = %self.model, attempt, max_retries = MAX_RETRIES, "Calling LLM backend");

            let mut req = self
                .http_client
                .post(&self.endpoint)
                .header("Content-Type", "application/json");
            if let Some(key) = &self.api_key {
                req = req.header("Authorization", format!("Bearer {key}"));
            }

            match req.json(&request).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        consecutive_failures = 0;
                        match response.text().await {
                            Ok(text) => {
                                if let Some(parsed) = extract_json(&text) {
                                    info!(backend = label, "Parsed LLM response");
                                    return Ok(parsed);
                                }
                                warn!(backend = label, "Failed to extract JSON from response");
                                last_error = RgaaError::Llm {
                                    message: format!("{label}: failed to parse response JSON"),
                                    code: Some("JSON_PARSE_ERROR".to_string()),
                                };
                                consecutive_failures += 1;
                            }
                            Err(e) => {
                                error!(backend = label, "Failed to read response body: {e}");
                                last_error = RgaaError::Llm {
                                    message: format!("{label}: response read error: {e}"),
                                    code: Some("RESPONSE_READ_ERROR".to_string()),
                                };
                                consecutive_failures += 1;
                            }
                        }
                    } else if status.as_u16() == 429 {
                        let retry_after = response
                            .headers()
                            .get("retry-after")
                            .and_then(|v| v.to_str().ok())
                            .and_then(|s| s.parse::<u64>().ok())
                            .unwrap_or(INITIAL_BACKOFF_MS / 1000);
                        let backoff_ms = INITIAL_BACKOFF_MS * 2u64.pow(attempt - 1);
                        let sleep_ms = backoff_ms + jitter_for(backoff_ms);
                        warn!(
                            backend = label,
                            attempt,
                            backoff_ms = sleep_ms,
                            retry_after,
                            "Rate limited, backing off"
                        );
                        tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
                        last_error = RgaaError::RateLimited { retry_after };
                        consecutive_failures += 1;
                    } else {
                        let body = response.text().await.unwrap_or_default();
                        error!(backend = label, status = status.as_u16(), body = %body, "API error");
                        last_error = RgaaError::Llm {
                            message: format!("{label}: API error {}: {body}", status.as_u16()),
                            code: Some(status.as_u16().to_string()),
                        };
                        consecutive_failures += 1;
                    }
                }
                Err(e) => {
                    error!(backend = label, "Request failed: {e}");
                    last_error = RgaaError::Llm {
                        message: format!("{label}: request error: {e}"),
                        code: Some("REQUEST_ERROR".to_string()),
                    };
                    consecutive_failures += 1;
                    if attempt < MAX_RETRIES {
                        let backoff = INITIAL_BACKOFF_MS * 2u64.pow(attempt - 1);
                        tokio::time::sleep(Duration::from_millis(backoff + jitter_for(backoff)))
                            .await;
                    }
                }
            }
        }

        Err(last_error)
    }
}

/// Extracts a [`HoloResponse`] from a raw response body.
///
/// Accepts either an OpenAI-style envelope (`choices[0].message.content`) or a
/// bare body, then tries: direct JSON, a ```` ```json ```` code block, and a
/// regex over the text for the three expected fields.
pub fn extract_json(text: &str) -> Option<HoloResponse> {
    if let Ok(envelope) = serde_json::from_str::<ChatEnvelope>(text) {
        if let Some(content) = envelope
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
        {
            if let Some(parsed) = extract_from_content(&content) {
                return Some(parsed);
            }
        }
    }
    extract_from_content(text)
}

fn extract_from_content(text: &str) -> Option<HoloResponse> {
    if let Ok(response) = serde_json::from_str::<HoloResponse>(text) {
        return Some(response);
    }
    if let Some(json_str) = extract_from_code_block(text) {
        if let Ok(response) = serde_json::from_str::<HoloResponse>(&json_str) {
            return Some(response);
        }
    }
    if let Some(json_str) = extract_with_regex(text) {
        if let Ok(response) = serde_json::from_str::<HoloResponse>(&json_str) {
            return Some(response);
        }
    }
    None
}

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

fn extract_with_regex(text: &str) -> Option<String> {
    let pattern = r#"\{[^{}]*"verdict"[^{}]*"confidence"[^{}]*"justification"[^{}]*\}"#;
    let re = regex_lite::Regex::new(pattern).ok()?;
    re.find(text).map(|m| m.as_str().to_string())
}

/// Cheap, dependency-free jitter (0..=backoff/2) to spread retries and avoid a
/// thundering herd when many evaluations run concurrently.
fn jitter_for(backoff: u64) -> u64 {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    seed % (backoff / 2 + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_json_direct() {
        let r = extract_json(r#"{"verdict": "pass", "confidence": 0.95, "justification": "Test"}"#)
            .unwrap();
        assert_eq!(r.verdict, "pass");
        assert_eq!(r.confidence, 0.95);
    }

    #[test]
    fn extract_json_from_code_block() {
        let text = "Here is the result:\n```json\n{\"verdict\": \"fail\", \"confidence\": 0.8, \"justification\": \"Missing alt text\"}\n```\n";
        assert_eq!(extract_json(text).unwrap().verdict, "fail");
    }

    #[test]
    fn extract_json_from_regex() {
        let text = "The verdict is {\"verdict\": \"na\", \"confidence\": 1.0, \"justification\": \"N/A\"} for this criterion.";
        assert_eq!(extract_json(text).unwrap().verdict, "na");
    }

    #[test]
    fn extract_json_from_openai_envelope() {
        let body = r#"{"id":"x","choices":[{"index":0,"message":{"role":"assistant","content":"```json\n{\"verdict\":\"fail\",\"confidence\":0.7,\"justification\":\"alt manquant\"}\n```"}}]}"#;
        let r = extract_json(body).unwrap();
        assert_eq!(r.verdict, "fail");
        assert_eq!(r.justification, "alt manquant");
    }

    #[test]
    fn extract_json_invalid() {
        assert!(extract_json("No JSON here").is_none());
        assert!(extract_json(r#"{"choices":[{"message":{"content":"nope"}}]}"#).is_none());
    }

    #[test]
    fn multimodal_messages_reject_bad_base64() {
        let err = ChatTransport::multimodal_messages("p", Some("not-valid-base64!!!")).unwrap_err();
        assert!(err.to_string().contains("base64"));
    }

    #[test]
    fn multimodal_messages_without_image_are_text_only() {
        let m = ChatTransport::multimodal_messages("p", None).unwrap();
        assert_eq!(m.len(), 2);
        assert!(m[1].content.is_string());
    }
}
