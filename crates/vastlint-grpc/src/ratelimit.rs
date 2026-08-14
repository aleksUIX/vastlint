//! Per-caller rate limiting, as a token bucket.
//!
//! Distinct from the concurrency limiter, and both are needed. The concurrency
//! limiter protects the server from the aggregate: it does not care who is
//! calling, only that the box is full. The rate limiter protects callers from
//! each other: one client in a retry storm can hold the concurrency limit shut
//! for everyone else without ever exceeding it, because it refills the slot the
//! instant it frees.
//!
//! ## What the caller identity is, and is not
//!
//! Identity comes from a request header. That is a fairness mechanism, not
//! authentication: a caller can claim to be anyone. It is still useful, because
//! the failure being prevented is an honest client misbehaving, not a
//! determined attacker. Authenticating the identity is a separate piece of work
//! and pretending a header does it would be worse than saying so.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use tonic::Status;
use tower::{Layer, Service};

use crate::config::RateLimitConfig;
use crate::metrics;

/// Identity used when a request carries no caller header.
///
/// All anonymous callers share one bucket. That is deliberate: giving each
/// unidentified request its own allowance would make omitting the header the
/// cheapest way to bypass the limit.
const ANONYMOUS: &str = "anonymous";

/// How long an idle bucket is kept before being reclaimed.
///
/// Without this the map grows once per distinct caller string, forever, which
/// turns the rate limiter into a memory leak that a caller controls by varying
/// a header.
const BUCKET_TTL: Duration = Duration::from_secs(600);

/// A single caller's allowance.
#[derive(Debug)]
struct Bucket {
    /// Scaled by `SCALE` so refill can be fractional without floating point.
    tokens: u64,
    last_refill: Instant,
    last_used: Instant,
}

/// Fixed-point scale for token accounting.
///
/// Integer tokens would round a 3 requests-per-second limit's refill to zero on
/// any interval shorter than 333ms, so a caller polling every 100ms would never
/// be refilled at all.
const SCALE: u64 = 1_000_000;

/// Token buckets, keyed by caller.
#[derive(Debug)]
pub struct RateLimiter {
    config: RateLimitConfig,
    buckets: Mutex<HashMap<String, Bucket>>,
    /// Requests rejected, for metrics and for tests.
    rejected: AtomicU64,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            buckets: Mutex::new(HashMap::new()),
            rejected: AtomicU64::new(0),
        }
    }

    pub fn enabled(&self) -> bool {
        self.config.per_second > 0
    }

    pub fn rejected(&self) -> u64 {
        self.rejected.load(Ordering::Relaxed)
    }

    /// Takes one token for `caller`, or reports that the allowance is spent.
    pub fn try_acquire(&self, caller: &str) -> bool {
        self.try_acquire_at(caller, Instant::now())
    }

    /// Time is a parameter so the refill behaviour can be tested without
    /// sleeping. A rate limiter tested by sleeping is a slow test suite and a
    /// flaky one.
    fn try_acquire_at(&self, caller: &str, now: Instant) -> bool {
        if !self.enabled() {
            return true;
        }

        let burst = self.config.burst.max(1) as u64 * SCALE;
        let refill_per_second = self.config.per_second as u64 * SCALE;

        let mut buckets = self.buckets.lock().unwrap_or_else(|poisoned| {
            // A panic while holding the lock leaves the map intact: every
            // operation on it is a single mutation. Refusing traffic because a
            // previous request panicked would turn one bug into an outage.
            poisoned.into_inner()
        });

        // Reclaim idle buckets while the lock is already held, rather than
        // running a background task to do it.
        if buckets.len() > 1_000 {
            buckets.retain(|_, bucket| now.duration_since(bucket.last_used) < BUCKET_TTL);
        }

        let bucket = buckets.entry(caller.to_string()).or_insert(Bucket {
            tokens: burst,
            last_refill: now,
            last_used: now,
        });

        let elapsed = now.saturating_duration_since(bucket.last_refill);
        let refill = (elapsed.as_secs_f64() * refill_per_second as f64) as u64;
        bucket.tokens = bucket.tokens.saturating_add(refill).min(burst);
        bucket.last_refill = now;
        bucket.last_used = now;

        if bucket.tokens >= SCALE {
            bucket.tokens -= SCALE;
            true
        } else {
            self.rejected.fetch_add(1, Ordering::Relaxed);
            false
        }
    }
}

/// Tower layer applying per-caller rate limits.
#[derive(Debug, Clone)]
pub struct RateLimitLayer {
    limiter: Arc<RateLimiter>,
    caller_header: Arc<str>,
}

impl RateLimitLayer {
    pub fn new(limiter: Arc<RateLimiter>, caller_header: impl Into<Arc<str>>) -> Self {
        Self {
            limiter,
            caller_header: caller_header.into(),
        }
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimited<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimited {
            inner,
            limiter: Arc::clone(&self.limiter),
            caller_header: Arc::clone(&self.caller_header),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RateLimited<S> {
    inner: S,
    limiter: Arc<RateLimiter>,
    caller_header: Arc<str>,
}

impl<S, ReqBody, ResBody> Service<http::Request<ReqBody>> for RateLimited<S>
where
    S: Service<http::Request<ReqBody>, Response = http::Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
    ResBody: Default + Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: http::Request<ReqBody>) -> Self::Future {
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        if !self.limiter.enabled() {
            return Box::pin(async move { inner.call(request).await });
        }

        let caller = request
            .headers()
            .get(self.caller_header.as_ref())
            .and_then(|value| value.to_str().ok())
            .filter(|value| !value.is_empty())
            .unwrap_or(ANONYMOUS);

        if !self.limiter.try_acquire(caller) {
            metrics::record_rate_limited();
            // RESOURCE_EXHAUSTED, the same code as a concurrency shed. gRPC has
            // no separate rate-limit status, and callers should treat both the
            // same way: back off and retry later.
            let response =
                Status::resource_exhausted("per-caller rate limit exceeded; retry with backoff")
                    .into_http();
            return Box::pin(async move { Ok(response) });
        }

        Box::pin(async move { inner.call(request).await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limiter(per_second: u32, burst: u32) -> RateLimiter {
        RateLimiter::new(RateLimitConfig { per_second, burst })
    }

    #[test]
    fn a_disabled_limiter_allows_everything() {
        let limiter = limiter(0, 0);
        assert!(!limiter.enabled());
        for _ in 0..10_000 {
            assert!(limiter.try_acquire("someone"));
        }
        assert_eq!(limiter.rejected(), 0);
    }

    #[test]
    fn a_caller_may_spend_its_burst_immediately() {
        let limiter = limiter(10, 5);
        let now = Instant::now();

        for i in 0..5 {
            assert!(limiter.try_acquire_at("a", now), "request {i} within burst");
        }
        assert!(!limiter.try_acquire_at("a", now), "burst is spent");
    }

    #[test]
    fn tokens_refill_over_time() {
        let limiter = limiter(10, 2);
        let start = Instant::now();

        assert!(limiter.try_acquire_at("a", start));
        assert!(limiter.try_acquire_at("a", start));
        assert!(!limiter.try_acquire_at("a", start));

        // 10 per second means one token every 100ms.
        assert!(limiter.try_acquire_at("a", start + Duration::from_millis(100)));
    }

    /// Integer token accounting would round this refill to zero and starve the
    /// caller permanently.
    #[test]
    fn slow_rates_still_refill_on_short_intervals() {
        let limiter = limiter(3, 1);
        let start = Instant::now();

        assert!(limiter.try_acquire_at("a", start));
        assert!(!limiter.try_acquire_at("a", start));

        // Four 100ms steps at 3/s is 1.2 tokens, so the fifth call succeeds
        // even though no single step earns a whole token.
        let mut allowed = false;
        for step in 1..=4 {
            allowed |= limiter.try_acquire_at("a", start + Duration::from_millis(100 * step));
        }
        assert!(allowed, "fractional refill must accumulate");
    }

    #[test]
    fn the_bucket_never_exceeds_its_burst() {
        let limiter = limiter(100, 3);
        let start = Instant::now();

        // An hour of idling must not bank an hour of tokens.
        let much_later = start + Duration::from_secs(3600);
        for i in 0..3 {
            assert!(limiter.try_acquire_at("a", much_later), "banked token {i}");
        }
        assert!(!limiter.try_acquire_at("a", much_later), "burst is the cap");
    }

    #[test]
    fn callers_do_not_share_an_allowance() {
        let limiter = limiter(10, 2);
        let now = Instant::now();

        assert!(limiter.try_acquire_at("noisy", now));
        assert!(limiter.try_acquire_at("noisy", now));
        assert!(!limiter.try_acquire_at("noisy", now), "noisy is spent");

        assert!(
            limiter.try_acquire_at("quiet", now),
            "one caller must not spend another's allowance"
        );
    }

    #[test]
    fn rejections_are_counted() {
        let limiter = limiter(1, 1);
        let now = Instant::now();

        assert!(limiter.try_acquire_at("a", now));
        assert!(!limiter.try_acquire_at("a", now));
        assert!(!limiter.try_acquire_at("a", now));

        assert_eq!(limiter.rejected(), 2);
    }

    /// A zero burst would reject every request forever, including the first.
    #[test]
    fn a_zero_burst_is_treated_as_one() {
        let limiter = limiter(5, 0);
        assert!(limiter.try_acquire_at("a", Instant::now()));
    }

    #[test]
    fn idle_buckets_are_reclaimed() {
        let limiter = limiter(10, 10);
        let start = Instant::now();

        for i in 0..1_100 {
            limiter.try_acquire_at(&format!("caller-{i}"), start);
        }
        assert!(limiter.buckets.lock().unwrap().len() > 1_000);

        // One more request, long after the rest went idle.
        limiter.try_acquire_at("late", start + BUCKET_TTL + Duration::from_secs(1));

        let remaining = limiter.buckets.lock().unwrap().len();
        assert!(
            remaining < 10,
            "expired buckets should be reclaimed, {remaining} remain"
        );
    }
}
