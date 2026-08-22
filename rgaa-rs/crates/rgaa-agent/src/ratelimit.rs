use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;

/// Model tiers with distinct rate limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTier {
    /// Fast, cheap model (used for most steps).
    Tactical,
    /// Slower reasoning model (used for hard criteria).
    Reasoning,
}

struct Inner {
    tactical_tokens: AtomicU32,
    tactical_refill: u32,
    tactical_capacity: u32,
    tactical_last_refill: Mutex<Instant>,
    reasoning_tokens: AtomicU32,
    reasoning_refill: u32,
    reasoning_capacity: u32,
    reasoning_last_refill: Mutex<Instant>,
}

/// Token-bucket rate limiter with independent buckets per [`ModelTier`].
///
/// Each bucket refills on its own clock, so a burst of tactical calls cannot
/// starve the reasoning bucket (and vice versa).
pub struct Ratelimiter {
    inner: Arc<Inner>,
}

impl Ratelimiter {
    pub fn new(tactical_rpm: u32, reasoning_rpm: u32) -> Self {
        let now = Instant::now();
        Self {
            inner: Arc::new(Inner {
                tactical_tokens: AtomicU32::new(tactical_rpm),
                tactical_refill: tactical_rpm / 60,
                tactical_capacity: tactical_rpm,
                tactical_last_refill: Mutex::new(now),
                reasoning_tokens: AtomicU32::new(reasoning_rpm),
                reasoning_refill: reasoning_rpm / 60,
                reasoning_capacity: reasoning_rpm,
                reasoning_last_refill: Mutex::new(now),
            }),
        }
    }

    /// Acquires a token for `tier`, blocking (with bounded sleeps) until one is
    /// available.
    pub async fn acquire(&self, tier: ModelTier) {
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

    /// Refills the bucket belonging to `tier` based on elapsed real time.
    async fn refill(&self, tier: ModelTier) {
        let (tokens, refill, capacity, last) = match tier {
            ModelTier::Tactical => (
                &self.inner.tactical_tokens,
                self.inner.tactical_refill,
                self.inner.tactical_capacity,
                &self.inner.tactical_last_refill,
            ),
            ModelTier::Reasoning => (
                &self.inner.reasoning_tokens,
                self.inner.reasoning_refill,
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
        if refill == 0 {
            *last_guard = now;
            return;
        }

        let gained = (elapsed * refill as f64) as u32;
        let current = tokens.load(Ordering::Relaxed);
        let updated = (current + gained).min(capacity);
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
}
