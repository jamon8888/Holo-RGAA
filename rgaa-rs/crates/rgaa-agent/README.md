# rgaa-agent

Rig-based agentic evaluator for RGAA IA_ASSISTE criteria.

## Features
- Dual model routing (35b tactical / 122b reasoning)
- Token-bucket rate limiter per model tier
- Enriched prompts with criterion definitions and WCAG refs
- Confidence-based NeedsReview escalation
- Per-criterion evidence traces
- LanceDB-backed conversation memory and vector retrieval

## Usage
```rust
use rgaa_agent::{RgaaAgent, AgentConfig};

// Build configuration
let config = AgentConfig::from_env()?;

// Create the agent
let agent = RgaaAgent::new(&config).await?;

// Evaluate criteria
let results = agent.run_ia_assiste(&ia_criteria, &page_context).await;
```