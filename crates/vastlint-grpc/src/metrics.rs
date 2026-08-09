//! Prometheus instrumentation and the `/metrics` endpoint.
//!
//! What is measured is chosen to answer one question: is the server keeping its
//! promise, and if not, which part broke. That means the shed count and the
//! current concurrency limit sit alongside latency, because a p99 that looks
//! healthy while the shed rate climbs is a server that got fast by refusing
//! work.
//!
//! ## On percentiles from bucketed histograms
//!
//! Prometheus histograms give quantiles by interpolating between bucket edges,
//! so a p999 is only as good as the buckets around it. The buckets below are
//! chosen against measured behaviour rather than left at the library defaults,
//! whose lowest edge of 5ms would put every single-tag validation in the first
//! bucket and make every quantile a straight-line guess.
//!
//! For the load experiment the client measures its own latencies with an HDR
//! histogram, which has no bucket-choice problem. These metrics are for
//! operating the server; that one is for the claim.

use std::net::SocketAddr;
use std::sync::OnceLock;

use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge, Opts, Registry,
    TextEncoder,
};

/// Latency buckets in seconds, spanning the measured range of the validator.
///
/// vastlint benchmarks at 363µs light and 2,104µs heavy per tag, so the
/// interesting region is roughly 100µs to 10ms, with the tail above it existing
/// to distinguish "slow" from "queueing".
const LATENCY_BUCKETS: &[f64] = &[
    0.000_1, 0.000_25, 0.000_5, 0.001, 0.002, 0.004, 0.008, 0.016, 0.032, 0.064, 0.128, 0.256, 0.5,
    1.0, 2.5,
];

struct Metrics {
    registry: Registry,
    requests: IntCounterVec,
    latency: HistogramVec,
    shed: IntCounter,
    rate_limited: IntCounter,
    concurrency_limit: IntGauge,
}

static METRICS: OnceLock<Metrics> = OnceLock::new();

fn metrics() -> &'static Metrics {
    METRICS.get_or_init(|| {
        let registry = Registry::new();

        let requests = IntCounterVec::new(
            Opts::new(
                "vastlint_grpc_requests_total",
                "gRPC requests completed, by method and resulting status code",
            ),
            &["method", "status"],
        )
        .expect("valid metric definition");

        let latency = HistogramVec::new(
            HistogramOpts::new(
                "vastlint_grpc_request_duration_seconds",
                "Server-side handling time, by method",
            )
            .buckets(LATENCY_BUCKETS.to_vec()),
            &["method"],
        )
        .expect("valid metric definition");

        let shed = IntCounter::new(
            "vastlint_grpc_shed_total",
            "Requests refused with RESOURCE_EXHAUSTED because the server was at its concurrency limit",
        )
        .expect("valid metric definition");

        let rate_limited = IntCounter::new(
            "vastlint_grpc_rate_limited_total",
            "Requests refused with RESOURCE_EXHAUSTED because the caller exceeded its rate limit",
        )
        .expect("valid metric definition");

        let concurrency_limit = IntGauge::new(
            "vastlint_grpc_concurrency_limit",
            "Current adaptive concurrency limit",
        )
        .expect("valid metric definition");

        registry.register(Box::new(requests.clone())).expect("unique metric");
        registry.register(Box::new(latency.clone())).expect("unique metric");
        registry.register(Box::new(shed.clone())).expect("unique metric");
        registry
            .register(Box::new(rate_limited.clone()))
            .expect("unique metric");
        registry
            .register(Box::new(concurrency_limit.clone()))
            .expect("unique metric");

        Metrics {
            registry,
            requests,
            latency,
            shed,
            rate_limited,
            concurrency_limit,
        }
    })
}

/// Records one completed RPC.
///
/// Called from the service methods rather than from a middleware layer, because
/// that is where the gRPC `Status` actually exists. In a layer the status lives
/// in the response trailers, and reading it there means buffering the body to
/// learn something the handler already knew.
pub fn record_request(method: &str, status: &str, seconds: f64) {
    let metrics = metrics();
    metrics.requests.with_label_values(&[method, status]).inc();
    metrics
        .latency
        .with_label_values(&[method])
        .observe(seconds);
}

/// Records a request refused by the concurrency limiter.
pub fn record_shed() {
    metrics().shed.inc();
}

/// Records a request refused by the rate limiter.
pub fn record_rate_limited() {
    metrics().rate_limited.inc();
}

/// Publishes the current adaptive limit.
pub fn set_concurrency_limit(limit: usize) {
    metrics().concurrency_limit.set(limit as i64);
}

/// Renders the registry in Prometheus text exposition format.
pub fn render() -> String {
    let encoder = TextEncoder::new();
    let mut buffer = Vec::new();
    // An encoding failure here is a bug in a metric definition, not a runtime
    // condition. Returning the error to a scraper would hide it; an empty body
    // would look like a healthy server with no traffic.
    encoder
        .encode(&metrics().registry.gather(), &mut buffer)
        .expect("metrics encode");
    String::from_utf8(buffer).expect("metrics are utf-8")
}

/// Serves `/metrics` on its own port.
///
/// Separate from the gRPC listener on purpose. Metrics must stay scrapeable
/// exactly when the main port is saturated, and sharing a listener would put
/// them behind the same limiter that is shedding.
pub async fn serve(addr: SocketAddr) -> std::io::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind(addr).await?;

    loop {
        let (mut socket, _) = match listener.accept().await {
            Ok(accepted) => accepted,
            // One failed accept, typically a file descriptor limit, must not
            // take the metrics endpoint down for good.
            Err(_) => continue,
        };

        tokio::spawn(async move {
            // Enough for a request line and headers. The endpoint answers the
            // same way regardless of what was asked, so the request is read
            // only to keep the client from seeing a reset before it finishes
            // writing.
            let mut discard = [0u8; 1024];
            let _ = socket.read(&mut discard).await;

            let body = render();
            let response = format!(
                "HTTP/1.1 200 OK\r\n\
                 Content-Type: text/plain; version=0.0.4\r\n\
                 Content-Length: {}\r\n\
                 Connection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.shutdown().await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latency_buckets_are_sorted_and_cover_the_working_range() {
        assert!(
            LATENCY_BUCKETS.windows(2).all(|pair| pair[0] < pair[1]),
            "prometheus requires strictly increasing bucket edges"
        );

        // The measured per-tag range, 363µs to 2,104µs, must not land in the
        // first or last bucket, or every quantile inside it is interpolation
        // across the whole range.
        let light = 0.000_363;
        let heavy = 0.002_104;
        assert!(
            LATENCY_BUCKETS[0] < light,
            "light tags need buckets below them"
        );
        assert!(
            LATENCY_BUCKETS.last().unwrap() > &heavy,
            "heavy tags need buckets above them"
        );
        assert!(
            LATENCY_BUCKETS.iter().filter(|edge| **edge < heavy).count() >= 4,
            "the working range needs enough edges for a meaningful p999"
        );
    }

    #[test]
    fn rendering_produces_prometheus_text() {
        record_request("Validate", "ok", 0.001);
        record_shed();
        set_concurrency_limit(17);

        let rendered = render();

        assert!(rendered.contains("vastlint_grpc_requests_total"));
        assert!(rendered.contains("vastlint_grpc_shed_total"));
        assert!(rendered.contains("vastlint_grpc_concurrency_limit 17"));
        // Labels have to survive, or per-method breakdown is unavailable
        // exactly when it is needed.
        assert!(rendered.contains("method=\"Validate\""));
        assert!(rendered.contains("status=\"ok\""));
    }

    #[test]
    fn the_registry_is_shared_across_calls() {
        let before = metrics().shed.get();
        record_shed();
        assert_eq!(metrics().shed.get(), before + 1);
    }
}
