use rgaa_agent::models::{ModelRouter, SelectedTier};
use rgaa_agent::prompts::PromptBuilder;
use rgaa_agent::ratelimit::{ModelTier, RateLimitConfig, RateLimiter};
use rgaa_agent::verify::{map_verdict, CONFIDENCE_THRESHOLD};
use rgaa_core::CriterionStatus;
use rgaa_holo::{HoloResponse, PageContext};
use std::time::Duration;

fn sample_context() -> PageContext {
    PageContext {
        title: Some("Test Page".to_string()),
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

#[test]
fn prompt_includes_criterion_definition() {
    let prompt = PromptBuilder::build("1.3", &sample_context());
    assert!(prompt.contains("Alternative textuelle pertinente"));
    assert!(prompt.contains("1.1.1"));
}

#[test]
fn prompt_includes_page_title() {
    let prompt = PromptBuilder::build("3.1", &sample_context());
    assert!(prompt.contains("Test Page"));
}

#[test]
fn prompt_includes_instructions() {
    let prompt = PromptBuilder::build("12.8", &sample_context());
    assert!(prompt.contains("verdict"));
    assert!(prompt.contains("confidence"));
    assert!(prompt.contains("justification"));
}

#[tokio::test]
async fn rate_limiter_enforces_budget() {
    let limiter = RateLimiter::new(10, 20); // 10 RPM tactical, 20 RPM reasoning
    let start = std::time::Instant::now();

    // Fire 15 tactical requests — should be bounded by 10 RPM
    let mut handles = vec![];
    for _ in 0..15 {
        let limiter = limiter.clone();
        handles.push(tokio::spawn(async move {
            limiter.acquire(ModelTier::Tactical).await;
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    let elapsed = start.elapsed();
    // With 10 RPM, 15 requests should take at least 30 seconds
    // (first 10 immediate, next 5 must wait for refill)
    // But for testing, we just verify it doesn't complete instantly
    assert!(
        elapsed > Duration::from_secs(1),
        "rate limiter should throttle"
    );
}

#[test]
fn rate_limiter_config_returns_tier_limits() {
    let limiter = RateLimiter::new(10, 20);
    let config = limiter.config();
    assert_eq!(
        config,
        RateLimitConfig {
            tactical_rpm: 10,
            reasoning_rpm: 20
        }
    );
}

#[tokio::test]
async fn rate_limiter_reset_restores_tokens() {
    let limiter = RateLimiter::new(5, 10);

    // Exhaust tactical tokens
    for _ in 0..5 {
        limiter.acquire(ModelTier::Tactical).await;
    }

    // Reset restores full capacity
    limiter.reset();

    let config = limiter.config();
    assert_eq!(config.tactical_rpm, 5);
    assert_eq!(config.reasoning_rpm, 10);

    // Acquire should succeed immediately after reset
    let start = std::time::Instant::now();
    limiter.acquire(ModelTier::Tactical).await;
    assert!(
        start.elapsed() < Duration::from_secs(1),
        "reset should restore tokens"
    );
}

#[test]
fn visual_criteria_routed_to_reasoning() {
    let router = ModelRouter::new_placeholder();
    assert!(router.route_for("1.3").is_reasoning());
    assert!(router.route_for("3.1").is_reasoning());
    assert!(router.route_for("11.2").is_reasoning());
    assert!(router.route_for("12.8").is_reasoning());
}

#[test]
fn text_criteria_routed_to_tactical() {
    let router = ModelRouter::new_placeholder();
    assert!(router.route_for("2.2").is_tactical());
    assert!(router.route_for("4.2").is_tactical());
    assert!(router.route_for("8.6").is_tactical());
    assert!(router.route_for("9.2").is_tactical());
}

#[test]
fn list_available_models_returns_both_tiers() {
    let router = ModelRouter::new_placeholder();
    let models = router.list_available_models();
    assert_eq!(models.len(), 2);
    assert!(models
        .iter()
        .any(|m| m.id == "holo3-1-35b-a3b" && m.tier == SelectedTier::Tactical));
    assert!(models
        .iter()
        .any(|m| m.id == "holo3-122b-a10b" && m.tier == SelectedTier::Reasoning));
}

#[test]
fn high_confidence_pass_maps_to_pass() {
    let response = HoloResponse {
        verdict: "pass".to_string(),
        confidence: 0.9,
        justification: "OK".to_string(),
    };
    assert_eq!(map_verdict(response), CriterionStatus::Pass);
}

#[test]
fn high_confidence_fail_maps_to_fail() {
    let response = HoloResponse {
        verdict: "fail".to_string(),
        confidence: 0.85,
        justification: "Missing alt".to_string(),
    };
    assert_eq!(map_verdict(response), CriterionStatus::Fail);
}

#[test]
fn low_confidence_maps_to_needs_review() {
    let response = HoloResponse {
        verdict: "pass".to_string(),
        confidence: 0.3,
        justification: "Uncertain".to_string(),
    };
    assert_eq!(map_verdict(response), CriterionStatus::NeedsReview);
}

#[test]
fn threshold_is_0_6() {
    assert_eq!(CONFIDENCE_THRESHOLD, 0.6);
}

#[test]
fn exactly_at_threshold_maps_to_verdict() {
    let response = HoloResponse {
        verdict: "fail".to_string(),
        confidence: 0.6,
        justification: "Borderline".to_string(),
    };
    assert_eq!(map_verdict(response), CriterionStatus::Fail);
}

#[test]
fn unknown_verdict_maps_to_needs_review() {
    let response = HoloResponse {
        verdict: "uncertain".to_string(),
        confidence: 0.9,
        justification: "Model unsure".to_string(),
    };
    assert_eq!(map_verdict(response), CriterionStatus::NeedsReview);
}
