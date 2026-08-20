use crate::criteria_defs::VISUAL_CRITERIA;
use crate::ratelimit::RateLimiter;
use rgaa_holo::HoloClient;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelInfo {
    pub id: &'static str,
    pub tier: SelectedTier,
}

#[derive(Clone)]
pub struct ModelRouter {
    tactical_client: HoloClient,
    reasoning_client: HoloClient,
    rate_limiter: RateLimiter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectedTier {
    Tactical,
    Reasoning,
}

impl SelectedTier {
    pub fn is_reasoning(&self) -> bool {
        *self == SelectedTier::Reasoning
    }

    pub fn is_tactical(&self) -> bool {
        *self == SelectedTier::Tactical
    }
}

impl ModelRouter {
    pub fn new(
        tactical_client: HoloClient,
        reasoning_client: HoloClient,
        rate_limiter: RateLimiter,
    ) -> Self {
        Self {
            tactical_client,
            reasoning_client,
            rate_limiter,
        }
    }

    /// Create a placeholder router for testing without API keys.
    pub fn new_placeholder() -> Self {
        let dummy_key = "test-key".to_string();
        Self::new(
            HoloClient::new(dummy_key.clone()),
            HoloClient::new(dummy_key),
            RateLimiter::new(10, 20),
        )
    }

    pub fn list_available_models(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "holo3-1-35b-a3b",
                tier: SelectedTier::Tactical,
            },
            ModelInfo {
                id: "holo3-122b-a10b",
                tier: SelectedTier::Reasoning,
            },
        ]
    }

    pub fn route_for(&self, criterion_id: &str) -> SelectedTier {
        if VISUAL_CRITERIA.contains(&criterion_id)
            || criterion_id.starts_with("11.")
            || criterion_id == "12.8"
        {
            SelectedTier::Reasoning
        } else {
            SelectedTier::Tactical
        }
    }

    pub fn rate_limiter(&self) -> &RateLimiter {
        &self.rate_limiter
    }
}
