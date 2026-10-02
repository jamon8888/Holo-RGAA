//! Pins the fix for #186/#193: both LLM code paths read one configuration
//! and put the same completion parameters on the wire.
//!
//! Before this, the `rig` agent sent neither `temperature` nor `max_tokens`
//! (so the provider's own defaults applied, and two runs of a bake-off were
//! not comparable) while `rgaa-holo`'s transport sent a hardcoded 0.1/512 of
//! its own. The assertions below compare the two request bodies directly, so
//! the split-brain cannot come back unnoticed.

use rgaa_agent::agent::RgaaAgent;
use rgaa_agent::config::AgentConfig;
use rgaa_agent::ratelimit::ModelTier;
use rgaa_core::{Classification, Criterion, LlmSettings};
use rgaa_holo::{ChatBackend, LlmBackend, PageContext};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

/// An OpenAI-shaped reply carrying the verdict JSON both paths expect.
const ENVELOPE: &str = r#"{"id":"c1","object":"chat.completion","choices":[{"index":0,"message":{"role":"assistant","content":"{\"verdict\":\"pass\",\"confidence\":0.9,\"justification\":\"ok\"}"},"finish_reason":"stop"}]}"#;

struct Stub {
    base_url: String,
    bodies: Arc<Mutex<Vec<serde_json::Value>>>,
}

impl Stub {
    /// Request bodies received so far, parsed as JSON.
    fn bodies(&self) -> Vec<serde_json::Value> {
        self.bodies.lock().expect("stub mutex poisoned").clone()
    }

    fn first_body(&self) -> serde_json::Value {
        self.bodies()
            .into_iter()
            .next()
            .expect("stub received no request")
    }
}

/// Minimal HTTP server answering every request with [`ENVELOPE`] and
/// recording the JSON body, so a test can assert what went on the wire
/// without reaching a real endpoint.
fn spawn_stub() -> Stub {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind stub");
    let addr = listener.local_addr().expect("stub addr");
    let bodies = Arc::new(Mutex::new(Vec::new()));
    let recorder = Arc::clone(&bodies);

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut s) = stream else { break };
            let recorder = Arc::clone(&recorder);
            std::thread::spawn(move || {
                let mut buf = vec![0u8; 256 * 1024];
                let mut filled = 0usize;
                // Read until the body announced by Content-Length has
                // arrived: a large prompt does not fit one TCP segment.
                loop {
                    let n = s.read(&mut buf[filled..]).unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    filled += n;
                    let raw = String::from_utf8_lossy(&buf[..filled]).into_owned();
                    let Some((head, body)) = raw.split_once("\r\n\r\n") else {
                        continue;
                    };
                    let want: usize = head
                        .lines()
                        .find_map(|l| {
                            let (k, v) = l.split_once(':')?;
                            k.eq_ignore_ascii_case("content-length")
                                .then(|| v.trim().parse().ok())?
                        })
                        .unwrap_or(0);
                    if body.len() >= want {
                        if let Ok(json) = serde_json::from_str(body) {
                            recorder.lock().expect("stub mutex poisoned").push(json);
                        }
                        break;
                    }
                    if filled == buf.len() {
                        break;
                    }
                }
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{ENVELOPE}",
                    ENVELOPE.len()
                );
                let _ = s.write_all(response.as_bytes());
                let _ = s.flush();
            });
        }
    });

    Stub {
        base_url: format!("http://{addr}/v1"),
        bodies,
    }
}

/// `LlmSettings` for a self-hosted route pointed at `base_url`, plus any
/// extra variables.
fn settings(base_url: &str, extra: &[(&str, &str)]) -> LlmSettings {
    let mut pairs: Vec<(String, String)> = vec![
        ("RGAA_LLM_PROVIDER".into(), "vllm".into()),
        ("RGAA_LLM_BASE_URL".into(), base_url.into()),
        ("RGAA_LLM_MODEL".into(), "qwen3-8b".into()),
    ];
    pairs.extend(
        extra
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string())),
    );
    LlmSettings::from_env_with(move |k| {
        pairs
            .iter()
            .find(|(key, _)| key == k)
            .map(|(_, v)| v.clone())
    })
    .expect("settings resolve")
}

fn criterion() -> Criterion {
    Criterion {
        id: "1.3",
        title: "Image porteuse d'information".to_string(),
        classification: Classification::IaAssiste,
        wcag_refs: "1.1.1",
    }
}

fn page_context() -> PageContext {
    PageContext {
        title: Some("Accueil".to_string()),
        lang: Some("fr".to_string()),
        headings: vec![],
        images: vec![],
        iframes: vec![],
        links: vec![],
        forms: vec![],
        media: vec![],
        navigation: vec![],
    }
}

/// Body the `rig` agent path puts on the wire for one criterion.
async fn agent_body(stub: &Stub, extra: &[(&str, &str)]) -> serde_json::Value {
    let config = AgentConfig::from_llm_settings(settings(&stub.base_url, extra));
    let agent = RgaaAgent::new(&config).await.expect("agent builds");
    // The verdict itself is irrelevant here; the request body is the subject.
    let _ = agent
        .evaluate_criterion(&criterion(), &page_context())
        .await;
    stub.first_body()
}

/// Body the `rgaa-holo` transport path puts on the wire for one prompt.
async fn transport_body(stub: &Stub, extra: &[(&str, &str)]) -> serde_json::Value {
    let backend = ChatBackend::new(&settings(&stub.base_url, extra)).expect("backend builds");
    backend.evaluate("prompt").await.expect("transport call");
    stub.first_body()
}

#[tokio::test]
async fn both_paths_send_the_same_explicit_sampling_parameters() {
    let extra = [
        ("RGAA_LLM_TEMPERATURE", "0.05"),
        ("RGAA_LLM_MAX_TOKENS", "1234"),
    ];

    let agent_stub = spawn_stub();
    let agent = agent_body(&agent_stub, &extra).await;
    let transport_stub = spawn_stub();
    let transport = transport_body(&transport_stub, &extra).await;

    for body in [&agent, &transport] {
        assert_eq!(body["temperature"], 0.05, "{body}");
        assert_eq!(body["max_tokens"], 1234, "{body}");
        assert_eq!(body["model"], "qwen3-8b", "{body}");
    }
    assert_eq!(agent["temperature"], transport["temperature"]);
    assert_eq!(agent["max_tokens"], transport["max_tokens"]);
}

#[tokio::test]
async fn both_paths_default_to_the_shared_constants() {
    let agent_stub = spawn_stub();
    let agent = agent_body(&agent_stub, &[]).await;
    let transport_stub = spawn_stub();
    let transport = transport_body(&transport_stub, &[]).await;

    for body in [&agent, &transport] {
        assert_eq!(
            body["temperature"],
            rgaa_core::DEFAULT_TEMPERATURE,
            "{body}"
        );
        assert_eq!(body["max_tokens"], rgaa_core::DEFAULT_MAX_TOKENS, "{body}");
    }
}

#[tokio::test]
async fn both_paths_switch_thinking_off_against_a_self_hosted_endpoint() {
    let agent_stub = spawn_stub();
    let agent = agent_body(&agent_stub, &[]).await;
    let transport_stub = spawn_stub();
    let transport = transport_body(&transport_stub, &[]).await;

    for body in [&agent, &transport] {
        assert_eq!(
            body["chat_template_kwargs"]["enable_thinking"], false,
            "{body}"
        );
        assert_eq!(body["think"], false, "{body}");
        assert_eq!(body["enable_thinking"], false, "{body}");
    }
}

#[tokio::test]
async fn only_the_transport_path_constrains_the_response_format() {
    // Deliberate asymmetry, documented in `RgaaAgent::new`: the agent serves
    // a batch prompt (an array of verdicts) and a single-criterion prompt
    // from the same fixed body, so a single-verdict schema would make the
    // batch prompt unanswerable.
    let extra = [("RGAA_LLM_RESPONSE_FORMAT", "json_schema")];

    let agent_stub = spawn_stub();
    let agent = agent_body(&agent_stub, &extra).await;
    assert!(agent.get("response_format").is_none(), "{agent}");

    let transport_stub = spawn_stub();
    let transport = transport_body(&transport_stub, &extra).await;
    assert_eq!(transport["response_format"]["type"], "json_schema");
}

#[test]
fn a_self_hosted_route_drops_the_hosted_rate_limits_and_timeout() {
    let local = AgentConfig::from_llm_settings(settings("http://gpu-box:8000/v1", &[]));
    // 0 rpm is "unlimited" to the rate limiter: a box the operator owns is
    // not billed per request and must not be throttled to 10/20 rpm.
    assert_eq!(local.tactical_rpm, 0);
    assert_eq!(local.reasoning_rpm, 0);
    // …and unlimited must not collapse concurrency to one call at a time.
    assert!(
        local.agent_concurrency > 1,
        "unthrottled route ran at concurrency {}",
        local.agent_concurrency
    );
    assert_eq!(local.timeout.as_secs(), 600);

    let hosted = AgentConfig::from_llm_settings(
        LlmSettings::from_env_with(|k| {
            match k {
                "HOLO3_API_KEY" => Some("k"),
                "RGAA_LLM_MODEL" => Some("holo3-1-35b-a3b"),
                _ => None,
            }
            .map(str::to_string)
        })
        .expect("hosted settings"),
    );
    assert_eq!(hosted.tactical_rpm, 10);
    assert_eq!(hosted.reasoning_rpm, 20);
    assert_eq!(hosted.timeout.as_secs(), 30);
}

#[tokio::test]
async fn the_agent_reports_the_parameters_it_was_built_with() {
    let config = AgentConfig::from_llm_settings(settings(
        "http://gpu-box:8000/v1",
        &[("RGAA_LLM_MAX_TOKENS", "999")],
    ));
    let agent = RgaaAgent::new(&config).await.expect("agent builds");

    let p = agent.provenance(ModelTier::Tactical);
    assert_eq!(p.provider, "vllm");
    assert_eq!(p.model, "qwen3-8b");
    assert_eq!(p.endpoint, "http://gpu-box:8000/v1/chat/completions");
    assert_eq!(p.max_tokens, 999);
    assert_eq!(p.temperature, rgaa_core::DEFAULT_TEMPERATURE);
    assert_eq!(p.enable_thinking, Some(false));
    assert!(p.summary().contains("thinking=off"), "{}", p.summary());
    // Both tiers share the route, so both record the same endpoint.
    assert_eq!(agent.provenance(ModelTier::Reasoning).endpoint, p.endpoint);
}
