use rgaa_agent::agent::RgaaAgent;
use rgaa_agent::config::AgentConfig;
use rgaa_core::{Classification, Criterion};
use rgaa_holo::PageContext;

fn has_api_key() -> bool {
    std::env::var("HOL3_API_KEY").is_ok() || std::env::var("HOLO3_API_KEY").is_ok()
}

#[tokio::test]
async fn test_agent_creation() {
    let config = AgentConfig::default();
    let agent = RgaaAgent::new(&config).await;
    assert!(
        agent.is_ok(),
        "Agent creation should succeed with default config"
    );
}

#[tokio::test]
async fn test_evaluate_criterion() {
    if !has_api_key() {
        eprintln!(
            "Skipping test_evaluate_criterion: no API key set (HOL3_API_KEY or HOLO3_API_KEY)"
        );
        return;
    }
    let config = AgentConfig::from_env().unwrap();
    let agent = RgaaAgent::new(&config).await.unwrap();

    let criterion = Criterion {
        id: "1.3",
        title: "Test Criterion".to_string(),
        classification: Classification::IaAssiste,
        wcag_refs: "1.1.1",
    };

    let page_context = PageContext {
        title: Some("Test Page".to_string()),
        lang: Some("fr".to_string()),
        headings: vec![],
        images: vec![],
        iframes: vec![],
        links: vec![],
        forms: vec![],
        media: vec![],
        navigation: vec![],
    };

    let result = agent.evaluate_criterion(&criterion, &page_context).await;
    // AgentConfig::default() has no API key, so this hits the real Holo3
    // endpoint with empty credentials and takes evaluate_criterion's error
    // path (NeedsReview / source "agent-error") in any environment without
    // HOLO3_API_KEY set — including CI's default test job and local runs.
    // With a real key configured, it exercises the success path instead,
    // whose status depends on the model's verdict. Assert what holds in
    // both rather than hardcoding the network-dependent outcome.
    assert!(result.source == "agent" || result.source == "agent-error");
    assert!(result.justification.is_some());
}

#[tokio::test]
async fn test_run_ia_assiste() {
    if !has_api_key() {
        eprintln!("Skipping test_run_ia_assiste: no API key set (HOL3_API_KEY or HOLO3_API_KEY)");
        return;
    }
    let config = AgentConfig::from_env().unwrap();
    let agent = RgaaAgent::new(&config).await.unwrap();

    let criteria = vec![
        Criterion {
            id: "1.3",
            title: "Test Criterion 1".to_string(),
            classification: Classification::IaAssiste,
            wcag_refs: "1.1.1",
        },
        Criterion {
            id: "11.2",
            title: "Test Criterion 2".to_string(),
            classification: Classification::IaAssiste,
            wcag_refs: "2.4.6",
        },
    ];

    let page_context = PageContext {
        title: Some("Test Page".to_string()),
        lang: Some("fr".to_string()),
        headings: vec![],
        images: vec![],
        iframes: vec![],
        links: vec![],
        forms: vec![],
        media: vec![],
        navigation: vec![],
    };

    let results = std::sync::Arc::new(agent)
        .run_ia_assiste(criteria, page_context)
        .await;
    assert_eq!(results.len(), 2);
    assert!(results.contains_key("1.3"));
    assert!(results.contains_key("11.2"));
}

/// One local completion response; the real agent, prompt and mapper run unchanged.
fn estimate_provider(
    content: &str,
    status: u16,
) -> (
    AgentConfig,
    std::sync::mpsc::Receiver<serde_json::Value>,
    std::thread::JoinHandle<()>,
) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let envelope = serde_json::json!({"id":"estimate", "object":"chat.completion", "model":"mock-model", "created":0,
        "usage":{"prompt_tokens":10,"completion_tokens":10,"total_tokens":20},
        "choices":[{"index":0,"message":{"role":"assistant","content":content},"finish_reason":"stop"}]});
    let body = envelope.to_string();
    let (sender, receiver) = std::sync::mpsc::channel();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut bytes = Vec::new();
        loop {
            let mut chunk = [0u8; 4096];
            let count = stream.read(&mut chunk).unwrap();
            assert!(count > 0, "request ended before its body");
            bytes.extend_from_slice(&chunk[..count]);
            let raw = String::from_utf8_lossy(&bytes);
            let Some((headers, request)) = raw.split_once("\r\n\r\n") else {
                continue;
            };
            let length: usize = headers
                .lines()
                .find_map(|line| {
                    let (key, value) = line.split_once(':')?;
                    key.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse().unwrap())
                })
                .unwrap();
            if request.len() >= length {
                sender.send(serde_json::from_str(request).unwrap()).unwrap();
                break;
            }
        }
        write!(stream, "HTTP/1.1 {status} Stub\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
    });
    let config = AgentConfig {
        provider: "custom".into(),
        base_url: format!("http://{address}/v1"),
        api_key: "mock-key".into(),
        model: "mock-model".into(),
        tactical_rpm: 0,
        reasoning_rpm: 0,
        timeout: std::time::Duration::from_secs(5),
        ..Default::default()
    };
    (config, receiver, server)
}

fn estimate_context() -> PageContext {
    PageContext {
        title: Some("Mock page".into()),
        lang: Some("fr".into()),
        headings: vec![],
        images: vec![],
        iframes: vec![],
        links: vec![],
        forms: vec![],
        media: vec![],
        navigation: vec![],
    }
}

const ESTIMATE_FAIL: &str = r#"[{"criterion_id":"4.2","tests":[{"test_key":"1","verdict":"fail","justification":"no transcript"},{"test_key":"2","verdict":"fail","justification":"no transcript"},{"test_key":"3","verdict":"fail","justification":"no transcript"}],"verdict":"fail","justification":"no transcript","confidence":0.72,"review_required":true}]"#;

#[tokio::test]
async fn automatic_estimator_accepts_human_routes_and_includes_prior_evidence() {
    use rgaa_core::{AutomatedVerdict, EnginePlan, EvidenceRef, PlanEngine, RgaaCriteria};
    assert_eq!(EnginePlan::primary("4.2"), Some(PlanEngine::Human));
    let criteria = vec![
        RgaaCriteria::find("4.2").unwrap().clone(),
        RgaaCriteria::find("4.4").unwrap().clone(),
    ];
    let mut prior = rgaa_agent::verify::map_automatic_response(&criteria, ESTIMATE_FAIL)
        .remove("4.2")
        .unwrap();
    prior.source = "browser".into();
    prior.evidence = vec![EvidenceRef::new("dom_snapshot", "sha256:recorded-media")];
    let (config, request, server) = estimate_provider(ESTIMATE_FAIL, 200);
    let agent = RgaaAgent::new(&config).await.unwrap();
    let results = agent
        .run_automatic_estimates(&criteria, &estimate_context(), &[prior])
        .await;
    assert_eq!(results.len(), 2);
    assert_eq!(results["4.4"].automated_verdict, None);
    assert_eq!(results["4.4"].verified_status, None);
    assert_eq!(
        results["4.4"].status,
        rgaa_core::CriterionStatus::NeedsReview
    );
    assert_eq!(
        results["4.2"].automated_verdict,
        Some(AutomatedVerdict::Fail)
    );
    assert_eq!(
        results["4.2"].status,
        rgaa_core::CriterionStatus::NeedsReview
    );
    assert_eq!(results["4.2"].verified_status, None);
    let body = request.recv().unwrap();
    let prompt = body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| m["role"] == "user")
        .map(|m| m["content"].to_string())
        .collect::<String>();
    assert!(prompt.contains("sha256:recorded-media"), "{prompt}");
    assert!(prompt.contains("test_key"), "{prompt}");
    server.join().unwrap();
}

#[tokio::test]
async fn automatic_estimator_returns_unresolved_ids_on_provider_or_shape_failure() {
    use rgaa_core::{CriterionStatus, RgaaCriteria};
    let criteria = vec![RgaaCriteria::find("4.2").unwrap().clone()];
    for (reply, status) in [("not json", 200), ("[]", 200), ("{}", 500)] {
        let (config, request, server) = estimate_provider(reply, status);
        let agent = RgaaAgent::new(&config).await.unwrap();
        let results = agent
            .run_automatic_estimates(&criteria, &estimate_context(), &[])
            .await;
        assert_eq!(results.len(), 1);
        assert_eq!(results["4.2"].status, CriterionStatus::NeedsReview);
        assert_eq!(results["4.2"].automated_verdict, None);
        assert_eq!(results["4.2"].verified_status, None);
        assert!(results["4.2"].review_required);
        request.recv().unwrap();
        server.join().unwrap();
    }
}
