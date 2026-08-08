use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, info, warn};

const API_URL: &str = "https://api.hcompany.ai/v1/chat/completions";
const MODEL: &str = "holo3-1-35b-a3b";
const MAX_RETRIES: u32 = 5;
const INITIAL_BACKOFF_MS: u64 = 1000;

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
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f64,
    max_tokens: u32,
}

pub struct HoloClient {
    api_key: String,
    http_client: Client,
}

impl HoloClient {
    pub fn new(api_key: String) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            api_key,
            http_client,
        }
    }

    pub async fn evaluate(&self, prompt: &str) -> Result<HoloResponse, String> {
        let request = ChatRequest {
            model: MODEL.to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: SYSTEM_PROMPT.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: prompt.to_string(),
                },
            ],
            temperature: 0.1,
            max_tokens: 2048,
        };

        let mut last_error = String::new();

        for attempt in 1..=MAX_RETRIES {
            info!(
                attempt,
                max_retries = MAX_RETRIES,
                "Calling Holo3 API"
            );

            match self
                .http_client
                .post(API_URL)
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
                        warn!(
                            attempt,
                            backoff_ms = backoff,
                            "Rate limited, backing off"
                        );
                        tokio::time::sleep(Duration::from_millis(backoff)).await;
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
                        tokio::time::sleep(Duration::from_millis(backoff)).await;
                    }
                }
            }
        }

        Err(format!(
            "Failed after {} attempts. Last error: {}",
            MAX_RETRIES, last_error
        ))
    }

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

    fn extract_from_code_block(text: &str) -> Option<String> {
        let patterns = [
            "```json\n",
            "```\n",
            "```json\r\n",
            "```\r\n",
        ];

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
}
