use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimitConfig {
    pub tactical_rpm: u32,
    pub reasoning_rpm: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTier {
    Tactical,  // holo3-1-35b-a3b, free, 10 RPM
    Reasoning, // holo3-122b-a10b, paid, configurable RPM
}

pub struct RateLimiterInner {
    tactical_tokens: AtomicU32,
    reasoning_tokens: AtomicU32,
    tactical_refill: u32,
    reasoning_refill: u32,
    last_refill: Mutex<Instant>,
}

#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<RateLimiterInner>,
}

impl RateLimiter {
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

    pub fn config(&self) -> RateLimitConfig {
        RateLimitConfig {
            tactical_rpm: self.inner.tactical_refill,
            reasoning_rpm: self.inner.reasoning_refill,
        }
    }

    pub fn reset(&self) {
        self.inner
            .tactical_tokens
            .store(self.inner.tactical_refill, Ordering::Release);
        self.inner
            .reasoning_tokens
            .store(self.inner.reasoning_refill, Ordering::Release);
    }

    async fn refill_if_needed(&self) {
        let mut last_refill = self.inner.last_refill.lock().await;
        let now = Instant::now();
        let elapsed = now.duration_since(*last_refill);
        if elapsed >= Duration::from_secs(60) {
            self.inner
                .tactical_tokens
                .store(self.inner.tactical_refill, Ordering::Release);
            self.inner
                .reasoning_tokens
                .store(self.inner.reasoning_refill, Ordering::Release);
            *last_refill = now;
        }
    }
}
