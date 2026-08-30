use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use thiserror::Error;
use tokio::sync::Mutex;

/// Errors that can occur during rate limiting.
#[derive(Debug, Error)]
pub enum RateLimitError {
    #[error("rate limit exceeded for {tier} tier: {rpm} requests/minute, try again in {retry_after_secs:.1}s")]
    LimitExceeded {
        tier: &'static str,
        rpm: u32,
        retry_after_secs: f64,
    },
}

/// Configuration snapshot for a [`Ratelimiter`] instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimitConfig {
    /// Tactical tier RPM.
    pub tactical_rpm: u32,
    /// Reasoning tier RPM.
    pub reasoning_rpm: u32,
}

/// Model tiers with distinct rate limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTier {
    /// Fast, cheap model (used for most steps).
    Tactical,
    /// Slower reasoning model (used for hard criteria).
    Reasoning,
}

impl ModelTier {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Tactical => "tactical",
            Self::Reasoning => "reasoning",
        }
    }
}

struct Inner {
    tactical_tokens: AtomicU32,
    tactical_rpm: u32,
    tactical_capacity: u32,
    tactical_last_refill: Mutex<Instant>,
    reasoning_tokens: AtomicU32,
    reasoning_rpm: u32,
    reasoning_capacity: u32,
    reasoning_last_refill: Mutex<Instant>,
}

/// Token-bucket rate limiter with independent buckets per [`ModelTier`].
///
/// Each bucket refills on its own clock using fractional token replenishment
/// based on elapsed time and configured RPM. Zero RPM means "unlimited" (no blocking).
#[derive(Clone)]
pub struct Ratelimiter {
    inner: Arc<Inner>,
}

impl Ratelimiter {
    pub fn new(tactical_rpm: u32, reasoning_rpm: u32) -> Self {
        let now = Instant::now();
        Self {
            inner: Arc::new(Inner {
                tactical_tokens: AtomicU32::new(tactical_rpm),
                tactical_rpm,
                tactical_capacity: tactical_rpm,
                tactical_last_refill: Mutex::new(now),
                reasoning_tokens: AtomicU32::new(reasoning_rpm),
                reasoning_rpm,
                reasoning_capacity: reasoning_rpm,
                reasoning_last_refill: Mutex::new(now),
            }),
        }
    }

    /// Returns a snapshot of the configured RPM limits.
    pub fn config(&self) -> RateLimitConfig {
        RateLimitConfig {
            tactical_rpm: self.inner.tactical_rpm,
            reasoning_rpm: self.inner.reasoning_rpm,
        }
    }

    /// Resets both token buckets to full capacity.
    pub fn reset(&self) {
        self.inner
            .tactical_tokens
            .store(self.inner.tactical_capacity, Ordering::Relaxed);
        self.inner
            .reasoning_tokens
            .store(self.inner.reasoning_capacity, Ordering::Relaxed);
    }

    /// Acquires a token for `tier`, blocking (with bounded sleeps) until one is
    /// available. If the tier's RPM is 0, returns immediately (unlimited).
    pub async fn acquire(&self, tier: ModelTier) {
        let rpm = match tier {
            ModelTier::Tactical => self.inner.tactical_rpm,
            ModelTier::Reasoning => self.inner.reasoning_rpm,
        };

        // Zero RPM = unlimited, no blocking
        if rpm == 0 {
            return;
        }

        loop {
            self.refill(tier).await;
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
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }
    }

    /// Attempts to acquire a token for `tier` without blocking.
    ///
    /// Returns `Ok(())` if a token was acquired. Returns `Err(RateLimitError)` if no
    /// tokens are available, containing the time until the next token is available.
    pub async fn try_acquire(&self, tier: ModelTier) -> Result<(), RateLimitError> {
        let (rpm, tokens, _capacity) = match tier {
            ModelTier::Tactical => (
                self.inner.tactical_rpm,
                &self.inner.tactical_tokens,
                self.inner.tactical_capacity,
            ),
            ModelTier::Reasoning => (
                self.inner.reasoning_rpm,
                &self.inner.reasoning_tokens,
                self.inner.reasoning_capacity,
            ),
        };

        // Zero RPM = unlimited
        if rpm == 0 {
            return Ok(());
        }

        self.refill(tier).await;

        let prev = tokens.load(Ordering::Acquire);
        if prev > 0 {
            tokens
                .compare_exchange(prev, prev - 1, Ordering::AcqRel, Ordering::Acquire)
                .map_err(|_| {
                    let retry_after = 60.0 / rpm as f64;
                    RateLimitError::LimitExceeded {
                        tier: tier.label(),
                        rpm,
                        retry_after_secs: retry_after,
                    }
                })?;
            Ok(())
        } else {
            let retry_after = 60.0 / rpm as f64;
            Err(RateLimitError::LimitExceeded {
                tier: tier.label(),
                rpm,
                retry_after_secs: retry_after,
            })
        }
    }

    /// Refills the bucket belonging to `tier` based on elapsed real time.
    /// Uses fractional replenishment: gained = elapsed_seconds * RPM / 60.
    /// Does not advance the refill timestamp if RPM is 0 (preserves accumulated time).
    async fn refill(&self, tier: ModelTier) {
        let (tokens, rpm, capacity, last) = match tier {
            ModelTier::Tactical => (
                &self.inner.tactical_tokens,
                self.inner.tactical_rpm,
                self.inner.tactical_capacity,
                &self.inner.tactical_last_refill,
            ),
            ModelTier::Reasoning => (
                &self.inner.reasoning_tokens,
                self.inner.reasoning_rpm,
                self.inner.reasoning_capacity,
                &self.inner.reasoning_last_refill,
            ),
        };

        let mut last_guard = last.lock().await;
        let now = Instant::now();
        let elapsed = now.duration_since(*last_guard).as_secs_f64();
        if elapsed < 0.01 {
            return;
        }
        if rpm == 0 {
            // Zero RPM = unlimited; don't advance last_refill so accumulated
            // elapsed time is preserved if RPM later becomes non-zero.
            return;
        }

        let gained = (elapsed * rpm as f64 / 60.0) as u32;
        let current = tokens.load(Ordering::Relaxed);
        let updated = current.saturating_add(gained).min(capacity);
        tokens.store(updated, Ordering::Relaxed);
        *last_guard = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn acquire_does_not_panic_with_zero_rpm() {
        let limiter = Ratelimiter::new(0, 0);
        tokio::time::timeout(Duration::from_secs(1), limiter.acquire(ModelTier::Tactical))
            .await
            .expect("acquire must not deadlock on zero RPM");
    }

    #[tokio::test]
    async fn try_acquire_succeeds_when_tokens_available() {
        let limiter = Ratelimiter::new(60, 30);
        limiter.reset();
        let result = limiter.try_acquire(ModelTier::Tactical).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn try_acquire_returns_error_when_exhausted() {
        let limiter = Ratelimiter::new(1, 1);
        limiter.reset();
        limiter.try_acquire(ModelTier::Tactical).await.unwrap();
        let result = limiter.try_acquire(ModelTier::Tactical).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, RateLimitError::LimitExceeded { tier: "tactical", .. }));
    }

    #[tokio::test]
    async fn try_acquire_unlimited_with_zero_rpm() {
        let limiter = Ratelimiter::new(0, 0);
        let result = limiter.try_acquire(ModelTier::Tactical).await;
        assert!(result.is_ok());
    }
}
