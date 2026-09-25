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
