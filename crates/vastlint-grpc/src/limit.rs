//! Adaptive concurrency limiting and load shedding.
//!
//! A static thread pool or a fixed concurrency cap is the wrong answer under
//! variable load: the right number depends on the document mix, the machine,
//! and what else is running on it, and every one of those changes without
//! anyone editing a config file. The limit here is discovered from observed
//! latency instead, using additive-increase multiplicative-decrease, in the
//! spirit of Netflix's `concurrency-limits`.
//!
//! Above the limit the server sheds with `RESOURCE_EXHAUSTED` rather than
//! queueing. That is the important half. A validator that queues past its
//! caller's deadline is worse than one that refuses immediately, because the
//! caller has already lost the auction and the work still consumed capacity
//! that a servable request needed.
//!
//! ## Why a counter and not a semaphore
//!
//! The obvious implementation is a `Semaphore` with `try_acquire`. It is not
//! used, because a semaphore's structure exists to make waiters wait, and this
//! layer never wants a waiter: a request that cannot run right now is refused,
//! not parked. Resizing a semaphore's permit count at runtime is also fiddly in
//! a way that an atomic counter is not, and the counter's one weakness, a small
//! race where two requests can both observe room for one, is harmless. It
//! admits an occasional extra request, which is exactly what the next latency
//! sample will correct.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use tonic::Status;
use tower::{Layer, Service};

use crate::config::LimitConfig;
use crate::metrics;

/// Paths that are never limited or shed.
///
/// Shedding a health check is actively harmful: a load balancer would pull the
/// instance out of rotation precisely when it is busy but still serving, which
/// moves its traffic onto the remaining instances and spreads the overload. The
/// same argument applies to reflection, which is how tooling discovers the
/// server at all.
const EXEMPT_PREFIXES: [&str; 3] = [
    "/grpc.health.v1.",
    "/grpc.reflection.v1.",
    "/grpc.reflection.v1alpha.",
];

/// The AIMD controller and its in-flight count.
#[derive(Debug)]
pub struct AdaptiveLimiter {
    config: LimitConfig,
    /// Current concurrency limit. Moves between `config.min` and `config.max`.
    limit: AtomicUsize,
    /// Requests currently admitted and not yet finished.
    inflight: AtomicUsize,
}

impl AdaptiveLimiter {
    pub fn new(config: LimitConfig) -> Self {
        let initial = config.initial.clamp(config.min, config.max);
        Self {
            limit: AtomicUsize::new(initial),
            inflight: AtomicUsize::new(0),
            config,
        }
    }

    pub fn limit(&self) -> usize {
        self.limit.load(Ordering::Relaxed)
    }

    pub fn inflight(&self) -> usize {
        self.inflight.load(Ordering::Relaxed)
    }

    /// Admits a request, or returns `None` when the server is at its limit.
    ///
    /// The returned guard records the outcome when dropped, so a request that
    /// panics still releases its slot and still feeds the controller. Losing
    /// in-flight accounting on the error path is how a limiter ratchets itself
    /// down to the floor and stays there.
    fn try_admit(self: &Arc<Self>) -> Option<Admitted> {
        if !self.config.enabled {
            return Some(Admitted {
                limiter: Arc::clone(self),
                started: Instant::now(),
                counted: false,
            });
        }

        let limit = self.limit.load(Ordering::Relaxed);
        let inflight = self.inflight.fetch_add(1, Ordering::AcqRel);

        if inflight >= limit {
            self.inflight.fetch_sub(1, Ordering::AcqRel);
            return None;
        }

        Some(Admitted {
            limiter: Arc::clone(self),
            started: Instant::now(),
            counted: true,
        })
    }

    /// Feeds one completed request back into the controller.
    ///
    /// The rule, following Netflix's `AIMDLimit`:
    ///
    /// - Latency above the target is evidence of queueing, so decrease
    ///   multiplicatively.
    /// - Otherwise increase by one, but *only* when the request ran with the
    ///   server at or near its limit. Growing on evidence gathered while idle
    ///   inflates the limit without ever testing it, and the inflated value is
    ///   then discovered all at once during the next traffic spike.
    fn sample(&self, latency: Duration, inflight_at_start: usize) {
        if !self.config.enabled {
            return;
        }

        let limit = self.limit.load(Ordering::Relaxed);

        let next = if latency > self.config.target_latency {
            let decreased = (limit as f64 * self.config.backoff_ratio).floor() as usize;
            // Always move by at least one, otherwise a backoff ratio close to
            // 1.0 leaves small limits permanently stuck.
            decreased.min(limit.saturating_sub(1)).max(self.config.min)
        } else if inflight_at_start + 1 >= limit {
            limit.saturating_add(1).min(self.config.max)
        } else {
            limit
        };

        if next != limit {
            self.limit.store(next, Ordering::Relaxed);
            metrics::set_concurrency_limit(next);
        }
    }
}

/// An admitted request. Releases its slot and records a sample on drop.
struct Admitted {
    limiter: Arc<AdaptiveLimiter>,
    started: Instant,
    /// False when the limiter is disabled, in which case nothing was counted on
    /// the way in and nothing must be uncounted on the way out.
    counted: bool,
}

impl Drop for Admitted {
    fn drop(&mut self) {
        if !self.counted {
            return;
        }

        let inflight_at_start = self.limiter.inflight.fetch_sub(1, Ordering::AcqRel);
        self.limiter
            .sample(self.started.elapsed(), inflight_at_start.saturating_sub(1));
    }
}

/// Tower layer applying the limiter to every gRPC call.
#[derive(Debug, Clone)]
pub struct AdaptiveConcurrencyLayer {
    limiter: Arc<AdaptiveLimiter>,
}

impl AdaptiveConcurrencyLayer {
    pub fn new(limiter: Arc<AdaptiveLimiter>) -> Self {
        Self { limiter }
    }
}

impl<S> Layer<S> for AdaptiveConcurrencyLayer {
    type Service = AdaptiveConcurrency<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AdaptiveConcurrency {
            inner,
            limiter: Arc::clone(&self.limiter),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AdaptiveConcurrency<S> {
    inner: S,
    limiter: Arc<AdaptiveLimiter>,
}

impl<S, ReqBody, ResBody> Service<http::Request<ReqBody>> for AdaptiveConcurrency<S>
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
        let exempt = EXEMPT_PREFIXES
            .iter()
            .any(|prefix| request.uri().path().starts_with(prefix));

        // The standard tower dance: `poll_ready` was called on `self`, so the
        // reservation belongs to this instance and the clone is what must be
        // left behind for the next call.
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        if exempt {
            return Box::pin(async move { inner.call(request).await });
        }

        let Some(permit) = self.limiter.try_admit() else {
            metrics::record_shed();
            let response = Status::resource_exhausted(
                "server is at its concurrency limit; retry with backoff",
            )
            .into_http();
            return Box::pin(async move { Ok(response) });
        };

        Box::pin(async move {
            let response = inner.call(request).await;
            // Dropped here rather than earlier, so the latency sample covers
            // the whole call including response encoding.
            drop(permit);
            response
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> LimitConfig {
        LimitConfig {
            enabled: true,
            initial: 10,
            min: 2,
            max: 100,
            target_latency: Duration::from_millis(50),
            backoff_ratio: 0.9,
        }
    }

    fn limiter(config: LimitConfig) -> Arc<AdaptiveLimiter> {
        Arc::new(AdaptiveLimiter::new(config))
    }

    #[test]
    fn admits_up_to_the_limit_then_sheds() {
        let limiter = limiter(LimitConfig {
            initial: 3,
            ..config()
        });

        let permits: Vec<_> = (0..3).map(|_| limiter.try_admit()).collect();
        assert!(
            permits.iter().all(Option::is_some),
            "first three are admitted"
        );
        assert_eq!(limiter.inflight(), 3);

        assert!(limiter.try_admit().is_none(), "the fourth is shed");
        // A shed request must not leave its slot occupied, or the limiter
        // ratchets down to nothing under sustained overload.
        assert_eq!(limiter.inflight(), 3);
    }

    #[test]
    fn releasing_a_permit_frees_a_slot() {
        // min must come down too: the initial value is clamped into range, so
        // an initial of 1 under a floor of 2 would silently be a limit of 2.
        let limiter = limiter(LimitConfig {
            initial: 1,
            min: 1,
            ..config()
        });

        let permit = limiter.try_admit().expect("admitted");
        assert!(limiter.try_admit().is_none());

        drop(permit);
        assert_eq!(limiter.inflight(), 0);
        assert!(limiter.try_admit().is_some(), "the slot is reusable");
    }

    #[test]
    fn slow_requests_decrease_the_limit() {
        let limiter = limiter(config());
        let before = limiter.limit();

        limiter.sample(Duration::from_millis(500), before);

        assert!(
            limiter.limit() < before,
            "latency past the target should back the limit off, got {} from {before}",
            limiter.limit()
        );
    }

    #[test]
    fn fast_requests_at_the_limit_increase_it() {
        let limiter = limiter(config());
        let before = limiter.limit();

        // Ran with the server saturated: this is evidence the limit is too low.
        limiter.sample(Duration::from_millis(1), before - 1);

        assert_eq!(limiter.limit(), before + 1);
    }

    /// The trap: growing on every fast request inflates the limit while the
    /// server is idle, so the inflated value is never tested until a spike
    /// arrives and discovers it all at once.
    #[test]
    fn fast_requests_below_the_limit_do_not_increase_it() {
        let limiter = limiter(config());
        let before = limiter.limit();

        limiter.sample(Duration::from_millis(1), 0);

        assert_eq!(limiter.limit(), before, "an idle server proves nothing");
    }

    #[test]
    fn the_limit_never_falls_below_the_floor() {
        let limiter = limiter(LimitConfig {
            initial: 3,
            min: 2,
            backoff_ratio: 0.1,
            ..config()
        });

        for _ in 0..50 {
            limiter.sample(Duration::from_secs(5), 0);
        }

        assert_eq!(limiter.limit(), 2, "the floor holds");
    }

    #[test]
    fn the_limit_never_exceeds_the_ceiling() {
        let limiter = limiter(LimitConfig {
            initial: 8,
            max: 10,
            ..config()
        });

        for _ in 0..50 {
            let at_limit = limiter.limit() - 1;
            limiter.sample(Duration::from_millis(1), at_limit);
        }

        assert_eq!(limiter.limit(), 10, "the ceiling holds");
    }

    /// A backoff ratio near 1.0 rounds to no change at small limits, which
    /// would leave an overloaded server unable to back off at all.
    #[test]
    fn backoff_always_moves_by_at_least_one() {
        let limiter = limiter(LimitConfig {
            initial: 5,
            min: 1,
            backoff_ratio: 0.99,
            ..config()
        });

        limiter.sample(Duration::from_secs(1), 0);
        assert_eq!(limiter.limit(), 4);
    }

    #[test]
    fn a_disabled_limiter_admits_everything() {
        let limiter = limiter(LimitConfig {
            enabled: false,
            initial: 1,
            ..config()
        });

        let permits: Vec<_> = (0..100).map(|_| limiter.try_admit()).collect();
        assert!(permits.iter().all(Option::is_some));
        // Nothing is counted while disabled, so nothing can be miscounted on
        // the way back out.
        assert_eq!(limiter.inflight(), 0);
    }

    #[test]
    fn a_disabled_limiter_never_moves_its_limit() {
        let limiter = limiter(LimitConfig {
            enabled: false,
            ..config()
        });
        let before = limiter.limit();

        limiter.sample(Duration::from_secs(10), 1000);

        assert_eq!(limiter.limit(), before);
    }

    #[test]
    fn the_initial_limit_is_clamped_into_range() {
        let limiter = AdaptiveLimiter::new(LimitConfig {
            initial: 5_000,
            max: 100,
            ..config()
        });
        assert_eq!(limiter.limit(), 100);

        let limiter = AdaptiveLimiter::new(LimitConfig {
            initial: 1,
            min: 4,
            ..config()
        });
        assert_eq!(limiter.limit(), 4);
    }

    /// An inner service that never completes, so the first request holds its
    /// slot for as long as the test needs. Deterministic in a way that firing
    /// concurrent real requests and hoping for contention is not.
    ///
    /// Written out rather than built with `service_fn`, because the layer needs
    /// `Future: Send + 'static` and an opaque `impl Service` return type does
    /// not carry that through.
    #[derive(Clone)]
    struct NeverCompletes;

    impl Service<http::Request<()>> for NeverCompletes {
        type Response = http::Response<()>;
        type Error = std::convert::Infallible;
        type Future = std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
        >;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _request: http::Request<()>) -> Self::Future {
            Box::pin(async {
                std::future::pending::<()>().await;
                unreachable!("the test never lets this resolve")
            })
        }
    }

    fn request(path: &str) -> http::Request<()> {
        http::Request::builder()
            .uri(format!("http://localhost{path}"))
            .body(())
            .expect("valid request")
    }

    /// The status code matters as much as the refusal. `RESOURCE_EXHAUSTED` is
    /// the one clients are expected to retry with backoff; returning
    /// `UNAVAILABLE` or an HTTP 503 would push well-behaved clients into
    /// immediate retries and make an overloaded server worse.
    #[tokio::test]
    async fn the_layer_sheds_with_resource_exhausted() {
        let limiter = limiter(LimitConfig {
            initial: 1,
            min: 1,
            max: 1,
            ..config()
        });
        let mut service = AdaptiveConcurrencyLayer::new(Arc::clone(&limiter)).layer(NeverCompletes);

        // Held, not awaited: dropping it would release the slot.
        let _occupied = service.call(request("/openadtech.vastlint.v1.VastlintService/Validate"));
        assert_eq!(limiter.inflight(), 1);

        let shed = service
            .call(request("/openadtech.vastlint.v1.VastlintService/Validate"))
            .await
            .expect("shedding is a response, not a transport error");

        assert_eq!(
            shed.headers()
                .get("grpc-status")
                .map(|v| v.to_str().unwrap()),
            // 8 is RESOURCE_EXHAUSTED.
            Some("8"),
            "a shed request must carry a gRPC status the caller can act on"
        );
    }

    /// Shedding a health check would make an overloaded instance look dead, so a
    /// load balancer would pull it out of rotation and move its traffic onto
    /// instances that are equally busy.
    #[tokio::test]
    async fn health_checks_are_served_while_shedding() {
        let limiter = limiter(LimitConfig {
            initial: 1,
            min: 1,
            max: 1,
            ..config()
        });
        let mut service = AdaptiveConcurrencyLayer::new(Arc::clone(&limiter)).layer(NeverCompletes);

        let _occupied = service.call(request("/openadtech.vastlint.v1.VastlintService/Validate"));

        // If the health path were shed, this would resolve immediately with a
        // status. Because it is exempt it reaches the inner service and hangs,
        // which is what the timeout observes.
        let health = tokio::time::timeout(
            Duration::from_millis(50),
            service.call(request("/grpc.health.v1.Health/Check")),
        )
        .await;

        assert!(
            health.is_err(),
            "the health path should have reached the inner service rather than being shed"
        );
    }

    #[test]
    fn health_and_reflection_paths_are_exempt() {
        for path in [
            "/grpc.health.v1.Health/Check",
            "/grpc.health.v1.Health/Watch",
            "/grpc.reflection.v1.ServerReflection/ServerReflectionInfo",
            "/grpc.reflection.v1alpha.ServerReflection/ServerReflectionInfo",
        ] {
            assert!(
                EXEMPT_PREFIXES
                    .iter()
                    .any(|prefix| path.starts_with(prefix)),
                "{path} should be exempt from shedding"
            );
        }

        assert!(
            !EXEMPT_PREFIXES.iter().any(|prefix| {
                "/openadtech.vastlint.v1.VastlintService/Validate".starts_with(prefix)
            }),
            "the validation path is not exempt"
        );
    }
}
