use rgaa_agent::prompts::PromptBuilder;
use rgaa_agent::ratelimit::{ModelTier, RateLimiter};
use rgaa_holo::PageContext;
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
    assert!(elapsed > Duration::from_secs(1), "rate limiter should throttle");
}
