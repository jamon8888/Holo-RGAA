use crate::criteria_defs::VISUAL_CRITERIA;
use crate::ratelimit::RateLimiter;
use rgaa_holo::HoloClient;

/// Information about an available model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelInfo {
    /// The model identifier string (e.g., "holo3-1-35b-a3b").
    pub id: &'static str,
    /// The tier this model belongs to.
    pub tier: SelectedTier,
}

/// Routes criteria to appropriate model tiers and manages rate limiting.
#[derive(Clone)]
pub struct ModelRouter {
    #[allow(dead_code)]
    tactical_client: HoloClient,
    #[allow(dead_code)]
    reasoning_client: HoloClient,
    rate_limiter: RateLimiter,
}

/// The model tier used for evaluating a criterion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectedTier {
    /// Fast, lower-cost model for standard criteria.
    Tactical,
    /// Higher-capability model for complex/visual criteria.
    Reasoning,
}

impl SelectedTier {
    /// Returns `true` if this tier is the reasoning tier.
    pub fn is_reasoning(&self) -> bool {
        *self == SelectedTier::Reasoning
    }

    /// Returns `true` if this tier is the tactical tier.
    pub fn is_tactical(&self) -> bool {
        *self == SelectedTier::Tactical
    }
}

impl ModelRouter {
    /// Creates a new `ModelRouter` with the given clients and rate limiter.
    ///
    /// # Arguments
    ///
    /// * `tactical_client` - The HoloClient for the tactical model tier.
    /// * `reasoning_client` - The HoloClient for the reasoning model tier.
    /// * `rate_limiter` - The rate limiter controlling request throughput.
    #[must_use]
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
    ///
    /// Uses dummy API keys and a non-routable base URL so HTTP calls fail fast.
    /// Suitable only for unit tests and integration tests.
    #[must_use]
    pub fn new_placeholder() -> Self {
        let dummy_key = "test-key".to_string();
        let dummy_url = "http://127.0.0.1:1".to_string();
        Self::new(
            HoloClient::new(dummy_key.clone()).with_base_url(&dummy_url),
            HoloClient::new(dummy_key).with_base_url(dummy_url),
            RateLimiter::new(10, 20),
        )
    }

    /// Returns a list of all available models and their tiers.
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

    /// Determines which model tier should evaluate the given criterion.
    ///
    /// Visual criteria and criteria in the 11.x range are routed to the
    /// reasoning tier; all others use the tactical tier.
    ///
    /// # Arguments
    ///
    /// * `criterion_id` - The RGAA criterion identifier (e.g., "1.3", "11.2").
    ///
    /// # Returns
    ///
    /// The `SelectedTier` appropriate for the given criterion.
    #[must_use]
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

    /// Returns a reference to the HoloClient for the given tier.
    #[must_use]
    pub fn client_for_tier(&self, tier: SelectedTier) -> &HoloClient {
        match tier {
            SelectedTier::Tactical => &self.tactical_client,
            SelectedTier::Reasoning => &self.reasoning_client,
        }
    }

    /// Returns a reference to the rate limiter.
    pub fn rate_limiter(&self) -> &RateLimiter {
        &self.rate_limiter
    }
}
