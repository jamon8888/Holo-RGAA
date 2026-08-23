use rgaa_agent::agent::RgaaAgent;
use rgaa_agent::config::AgentConfig;
use rgaa_core::{Classification, Criterion};
use rgaa_holo::PageContext;

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
    let config = AgentConfig::default();
    let agent = RgaaAgent::new(&config).await.unwrap();

    let criterion = Criterion {
        id: "1.3",
        title: "Test Criterion",
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
    assert_eq!(result.status, rgaa_core::CriterionStatus::NeedsReview);
    assert_eq!(result.source, "agent");
    assert!(result.justification.is_some());
}

#[tokio::test]
async fn test_run_ia_assiste() {
    let config = AgentConfig::default();
    let agent = RgaaAgent::new(&config).await.unwrap();

    let criteria = vec![
        Criterion {
            id: "1.3",
            title: "Test Criterion 1",
            classification: Classification::IaAssiste,
            wcag_refs: "1.1.1",
        },
        Criterion {
            id: "11.2",
            title: "Test Criterion 2",
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

    let results = agent.run_ia_assiste(&criteria, &page_context).await;
    assert_eq!(results.len(), 2);
    assert!(results.contains_key("1.3"));
    assert!(results.contains_key("11.2"));
}
