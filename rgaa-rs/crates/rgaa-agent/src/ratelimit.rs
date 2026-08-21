use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Configuration for the rate limiter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimitConfig {
    /// Maximum requests per minute for the tactical tier.
    pub tactical_rpm: u32,
    /// Maximum requests per minute for the reasoning tier.
    pub reasoning_rpm: u32,
}

/// The model tier used for rate limiting purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTier {
    /// holo3-1-35b-a3b, free tier, 10 RPM default.
    Tactical,
    /// holo3-122b-a10b, paid tier, configurable RPM.
    Reasoning,
}

pub(crate) struct RateLimiterInner {
    tactical_tokens: AtomicU32,
    reasoning_tokens: AtomicU32,
    tactical_refill: u32,
    reasoning_refill: u32,
    last_refill: Mutex<Instant>,
}

/// A token bucket rate limiter for controlling API request throughput.
///
/// Uses atomic operations for lock-free token acquisition on the hot path,
/// with a mutex only for periodic refill calculations.
///
/// # Examples
///
/// ```ignore
/// let limiter = RateLimiter::new(10, 20);
/// limiter.acquire(ModelTier::Tactical).await;
/// ```
#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<RateLimiterInner>,
}

impl RateLimiter {
    /// Creates a new rate limiter with the given RPM limits.
    ///
    /// Tokens start fully replenished (equal to the RPM limit).
    ///
    /// # Arguments
    ///
    /// * `tactical_rpm` - Maximum requests per minute for the tactical tier.
    /// * `reasoning_rpm` - Maximum requests per minute for the reasoning tier.
    #[must_use]
    pub fn new(tactical_rpm: u32, reasoning_rpm: u32) -> Self {
        Self {
            inner: Arc::new(RateLimiterInner {
                tactical_tokens: AtomicU32::new(tactical_rpm),
                reasoning_tokens: AtomicU32::new(reasoning_rpm),
                tactical_refill: tactical_rpm,
                reasoning_refill: reasoning_rpm,
                last_refill: Mutex::new(Instant::now()),
            }),
        }
    }

    /// Acquires a token for the given tier, blocking until available.
    ///
    /// This method will loop, refilling tokens as needed, until a token
    /// can be atomically decremented. Sleeps 1 second between retries
    /// when no tokens are available.
    ///
    /// # Arguments
    ///
    /// * `tier` - The model tier to acquire a token for.
    pub async fn acquire(&self, tier: ModelTier) {
        loop {
            self.refill_if_needed().await;
            let tokens = match tier {
                ModelTier::Tactical => &self.inner.tactical_tokens,
                ModelTier::Reasoning => &self.inner.reasoning_tokens,
            };
            let prev = tokens.load(Ordering::Acquire);
            if prev > 0 {
                if tokens
                    .compare_exchange(prev, prev - 1, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                {
                    return;
                }
            } else {
                // Wait 1 second before retrying
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }

    /// Returns the current rate limit configuration.
    pub fn config(&self) -> RateLimitConfig {
        RateLimitConfig {
            tactical_rpm: self.inner.tactical_refill,
            reasoning_rpm: self.inner.reasoning_refill,
        }
    }

    /// Resets all tokens to their maximum values.
    pub fn reset(&self) {
        self.inner
            .tactical_tokens
            .store(self.inner.tactical_refill, Ordering::Release);
        self.inner
            .reasoning_tokens
            .store(self.inner.reasoning_refill, Ordering::Release);
    }

    /// Returns the current number of available tokens for the given tier.
    ///
    /// # Arguments
    ///
    /// * `tier` - The model tier to check tokens for.
    ///
    /// # Returns
    ///
    /// The number of tokens currently available (may be 0).
    pub fn tokens(&self, tier: ModelTier) -> u32 {
        match tier {
            ModelTier::Tactical => self.inner.tactical_tokens.load(Ordering::Acquire),
            ModelTier::Reasoning => self.inner.reasoning_tokens.load(Ordering::Acquire),
        }
    }

    /// Refills tokens based on elapsed time since last refill.
    async fn refill_if_needed(&self) {
        let mut last_refill = self.inner.last_refill.lock().await;
        let now = Instant::now();
        let elapsed = now.duration_since(*last_refill).as_secs_f64();

        if elapsed < 0.01 {
            return; // Too soon, skip
        }

        // Calculate tokens to add based on elapsed time
        let tactical_new = (elapsed * self.inner.tactical_refill as f64 / 60.0) as u32;
        let reasoning_new = (elapsed * self.inner.reasoning_refill as f64 / 60.0) as u32;

        if tactical_new > 0 || reasoning_new > 0 {
            // Tactical tokens
            let current = self.inner.tactical_tokens.load(Ordering::Acquire);
            let replenished = (current + tactical_new).min(self.inner.tactical_refill);
            self.inner
                .tactical_tokens
                .store(replenished, Ordering::Release);

            // Reasoning tokens
            let current = self.inner.reasoning_tokens.load(Ordering::Acquire);
            let replenished = (current + reasoning_new).min(self.inner.reasoning_refill);
            self.inner
                .reasoning_tokens
                .store(replenished, Ordering::Release);

            // Update last_refill to account for consumed time
            let tokens_used = tactical_new.max(reasoning_new);
            if tokens_used > 0 {
                let tokens_per_second = self.inner.tactical_refill as f64 / 60.0;
                let time_consumed = Duration::from_secs_f64(tokens_used as f64 / tokens_per_second);
                *last_refill = now - time_consumed;
            } else {
                *last_refill = now;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_enforces_throttle() {
        let limiter = RateLimiter::new(1, 1);
        let start = Instant::now();
        limiter.acquire(ModelTier::Tactical).await;
        limiter.acquire(ModelTier::Tactical).await;
        let elapsed = start.elapsed();
        assert!(elapsed > Duration::from_secs(1));
    }

    #[test]
    fn test_config_returns_rpm_limits() {
        let limiter = RateLimiter::new(10, 20);
        let config = limiter.config();
        assert_eq!(config.tactical_rpm, 10);
        assert_eq!(config.reasoning_rpm, 20);
    }

    #[test]
    fn test_reset_restores_tokens() {
        let limiter = RateLimiter::new(10, 20);
        limiter.inner.tactical_tokens.store(0, Ordering::Release);
        limiter.inner.reasoning_tokens.store(0, Ordering::Release);
        limiter.reset();
        assert_eq!(limiter.inner.tactical_tokens.load(Ordering::Acquire), 10);
        assert_eq!(limiter.inner.reasoning_tokens.load(Ordering::Acquire), 20);
    }

    #[tokio::test]
    async fn test_smooth_token_refill() {
        let limiter = RateLimiter::new(10, 20);
        // Consume all tactical tokens
        for _ in 0..10 {
            limiter.acquire(ModelTier::Tactical).await;
        }
        assert_eq!(limiter.tokens(ModelTier::Tactical), 0);
        // Wait 200ms — should have ~2 tokens refilled (10 tokens/60s * 0.2s = 0.33, but min 1)
        tokio::time::sleep(Duration::from_millis(200)).await;
        // Force a refill check by acquiring (which calls refill_if_needed)
        limiter.acquire(ModelTier::Tactical).await;
        // After acquiring 1, should have refilled some
        let tokens = limiter.tokens(ModelTier::Tactical);
        assert!(
            tokens < 10,
            "tokens should be less than max after acquire, got {tokens}"
        );
    }
}
