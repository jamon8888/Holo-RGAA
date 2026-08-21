# rgaa-agent

Rig-based agentic evaluator for RGAA IA_ASSISTE criteria.

## Features
- Dual model routing (35b tactical / 122b reasoning)
- Token-bucket rate limiter per model tier
- Enriched prompts with criterion definitions and WCAG refs
- Confidence-based NeedsReview escalation
- Per-criterion evidence traces

## Usage
```rust
use rgaa_agent::agent::{RgaaAgent, AgentBuilder, create_simple_agent};
use rgaa_agent::models::ModelRouter;
use rgaa_agent::ratelimit::RateLimiter;

// Simple construction
let agent = create_simple_agent(api_key);

// Builder pattern
let agent = AgentBuilder::new()
    .model("holo3-122b-a10b")
    .max_concurrent(5)
    .build();

// Full control
let router = ModelRouter::new(tactical_client, reasoning_client, rate_limiter);
let agent = RgaaAgent::new(router);
let results = agent.run_ia_assiste(&criteria, &page_context, None).await;
```
