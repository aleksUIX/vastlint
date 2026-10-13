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

use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::{Mutex, OnceLock};

use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge, IntGaugeVec, Opts,
    Registry, TextEncoder,
};
use vastlint_core as core;

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
    events_published: IntCounter,
    events_dropped: IntCounter,
    concurrency_limit: IntGauge,
    active_workers: IntGauge,
    verdicts: IntCounterVec,
    findings: IntCounterVec,
    affected: IntCounterVec,
    documents: IntCounterVec,
    policy: IntGaugeVec,
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

        let events_published = IntCounter::new(
            "vastlint_grpc_events_published_total",
            "Validation events encoded and handed to the results-stream sink",
        )
        .expect("valid metric definition");

        let events_dropped = IntCounter::new(
            "vastlint_grpc_events_dropped_total",
            "Validation events discarded because the publish queue was full",
        )
        .expect("valid metric definition");

        let concurrency_limit = IntGauge::new(
            "vastlint_grpc_concurrency_limit",
            "Current adaptive concurrency limit",
        )
        .expect("valid metric definition");

        let active_workers = IntGauge::new(
            "vastlint_grpc_active_workers",
            "CPU jobs holding capacity, including jobs whose RPC has ended",
        ).expect("valid metric definition");

        let verdicts = IntCounterVec::new(
            Opts::new(
                "vastlint_grpc_verdicts_total",
                "Completed validations, by caller and whether the document had any errors",
            ),
            &["caller", "valid"],
        )
        .expect("valid metric definition");

        let findings = IntCounterVec::new(
            Opts::new(
                "vastlint_grpc_findings_total",
                "Individual finding occurrences by effective severity. Repeated occurrences count separately.",
            ),
            &["caller", "rule_id", "revenue_impact", "severity"],
        )
        .expect("valid metric definition");

        let affected = IntCounterVec::new(Opts::new("vastlint_grpc_rule_documents_total", "Completed documents affected by a rule and effective severity, counted once per document"), &["caller", "rule_id", "severity"]).expect("valid metric definition");
        let documents = IntCounterVec::new(Opts::new("vastlint_grpc_documents_total", "Completed documents by mutually exclusive outcome: malformed, unsupported, error, warning, info or clean"), &["caller", "outcome"]).expect("valid metric definition");
        let policy = IntGaugeVec::new(Opts::new("vastlint_grpc_policy_info", "Immutable active host rule policy and engine identity; one series per process"), &["profile", "policy_digest", "engine_version", "catalog_digest"]).expect("valid metric definition");
        registry.register(Box::new(affected.clone())).expect("unique metric");
        registry.register(Box::new(documents.clone())).expect("unique metric");
        registry.register(Box::new(policy.clone())).expect("unique metric");
        registry.register(Box::new(requests.clone())).expect("unique metric");
        registry.register(Box::new(latency.clone())).expect("unique metric");
        registry.register(Box::new(shed.clone())).expect("unique metric");
        registry
            .register(Box::new(rate_limited.clone()))
            .expect("unique metric");
        registry
            .register(Box::new(events_published.clone()))
            .expect("unique metric");
        registry
            .register(Box::new(events_dropped.clone()))
            .expect("unique metric");
        registry
            .register(Box::new(concurrency_limit.clone()))
            .expect("unique metric");
        registry.register(Box::new(active_workers.clone())).expect("unique metric");
        registry
            .register(Box::new(verdicts.clone()))
            .expect("unique metric");
        registry
            .register(Box::new(findings.clone()))
            .expect("unique metric");

        Metrics {
            registry,
            requests,
            latency,
            shed,
            rate_limited,
            events_published,
            events_dropped,
            concurrency_limit,
            active_workers,
            verdicts,
            findings,
            affected,
            documents,
            policy,
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

pub fn worker_started() {
    metrics().active_workers.inc();
}
pub fn worker_finished() {
    metrics().active_workers.dec();
}

/// Records a request refused by the rate limiter.
pub fn record_rate_limited() {
    metrics().rate_limited.inc();
}

/// Records a validation event encoded and handed to the sink.
///
/// Counted even when the sink discards, because the number worth watching is
/// how many events the server believes it produced. A gap between this and what
/// a consumer receives is a delivery problem; a gap between this and the request
/// count is a bug here.
pub fn record_event_published() {
    metrics().events_published.inc();
}

/// Records a validation event dropped because the publish queue was full.
///
/// Telemetry loss has to be visible. A results stream that silently skips
/// events under load is worse than one that is switched off, because a consumer
/// counting rejections would read the gap as a drop in rejections.
pub fn record_event_dropped() {
    metrics().events_dropped.inc();
}

/// Records one completed validation for the partner tally.
///
/// Always on, independent of the Avro results stream. The caller label is
/// `x-vastlint-caller` after sanitising: empty becomes `anonymous`, junk
/// charset becomes `anonymous`, and a flood of distinct values past 256
/// collapses to `other` so a request-id accidentally used as a partner name
/// cannot explode the series set.
pub fn record_verdict(caller: Option<&str>, result: &core::ValidationResult) {
    let metrics = metrics();
    let caller = caller_label(caller);
    let valid = if result.summary.is_valid() {
        "true"
    } else {
        "false"
    };
    metrics
        .verdicts
        .with_label_values(&[caller.as_str(), valid])
        .inc();

    let outcome = match result.input_status {
        core::InputStatus::Malformed => "malformed",
        core::InputStatus::Unsupported => "unsupported",
        core::InputStatus::Supported if result.summary.errors > 0 => "error",
        core::InputStatus::Supported if result.summary.warnings > 0 => "warning",
        core::InputStatus::Supported if result.summary.infos > 0 => "info",
        _ => "clean",
    };
    metrics
        .documents
        .with_label_values(&[caller.as_str(), outcome])
        .inc();
    if result.input_status == core::InputStatus::Unsupported {
        return;
    }
    let mut affected = HashSet::new();
    for issue in &result.issues {
        let impact = if rule_revenue_impact(issue.id) {
            "true"
        } else {
            "false"
        };
        metrics
            .findings
            .with_label_values(&[caller.as_str(), issue.id, impact, issue.severity.as_str()])
            .inc();
        if affected.insert((issue.id, issue.severity.as_str())) {
            metrics
                .affected
                .with_label_values(&[caller.as_str(), issue.id, issue.severity.as_str()])
                .inc();
        }
    }
}

/// Called once before accepting traffic. Profiles are host-owned, never request labels.
pub fn set_policy(policy: &crate::policy::RulePolicy) {
    let gauge = &metrics().policy;
    gauge.reset();
    gauge
        .with_label_values(&[
            policy.profile.as_str(),
            policy.digest.as_str(),
            core::VERSION,
            crate::provenance::catalog_digest(),
        ])
        .set(1);
}

/// Distinct caller labels we will keep as their own series.
const MAX_CALLERS: usize = 256;

fn caller_label(caller: Option<&str>) -> String {
    let sanitized = sanitize_caller(caller);
    if sanitized == "anonymous" {
        return sanitized;
    }

    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let mut seen = SEEN
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if seen.contains(&sanitized) {
        return sanitized;
    }
    if seen.len() >= MAX_CALLERS {
        return "other".to_string();
    }
    seen.insert(sanitized.clone());
    sanitized
}

/// Stable partner ids only: ASCII letters, digits, `.`, `_`, `:`, `-`, up to 64
/// bytes. Anything else is `anonymous` rather than a new time series.
fn sanitize_caller(caller: Option<&str>) -> String {
    const MAX_LEN: usize = 64;
    let Some(raw) = caller.map(str::trim).filter(|value| !value.is_empty()) else {
        return "anonymous".to_string();
    };
    if raw.len() <= MAX_LEN
        && raw.bytes().all(|byte| {
            matches!(
                byte,
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_' | b':' | b'-'
            )
        })
    {
        raw.to_string()
    } else {
        "anonymous".to_string()
    }
}

fn rule_revenue_impact(rule_id: &str) -> bool {
    static IMPACT: OnceLock<HashSet<&'static str>> = OnceLock::new();
    IMPACT
        .get_or_init(|| {
            core::all_rules()
                .iter()
                .filter(|rule| rule.revenue_impact())
                .map(|rule| rule.id)
                .collect()
        })
        .contains(rule_id)
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
    serve_listener(tokio::net::TcpListener::bind(addr).await?).await
}

/// Bounded HTTP handling. Slow or abandoned scrapes cannot create unlimited tasks.
pub async fn serve_listener(listener: tokio::net::TcpListener) -> std::io::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let slots = std::sync::Arc::new(tokio::sync::Semaphore::new(16));
    loop {
        let (mut socket, _) = listener.accept().await?;
        let Ok(permit) = slots.clone().try_acquire_owned() else {
            continue;
        };
        tokio::spawn(async move {
            let _permit = permit;
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), async {
                let mut request = [0u8; 4096];
                let mut used = 0;
                loop {
                    let read = socket.read(&mut request[used..]).await?;
                    if read == 0 { return Ok::<_, std::io::Error>(()); }
                    used += read;
                    if request[..used].windows(4).any(|w| w == b"\r\n\r\n") { break; }
                    if used == request.len() { return Ok(()); }
                }
                let (status, body) = if request[..used].starts_with(b"GET /metrics HTTP/1.") {
                    ("200 OK", render())
                } else { ("404 Not Found", String::new()) };
                let response = format!("HTTP/1.1 {status}\r\nContent-Type: text/plain; version=0.0.4\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                socket.write_all(response.as_bytes()).await?;
                socket.shutdown().await
            }).await;
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
    fn partner_tallies_use_the_caller_label() {
        let result = core::validate(
            r#"<VAST version="2.0"><Ad id="1"><InLine><AdSystem>Test</AdSystem><AdTitle>Test</AdTitle><Creatives><Creative><Linear><Duration>00:00:30</Duration><MediaFiles><MediaFile delivery="progressive" type="video/mp4" width="640" height="360">https://cdn.example.com/ad.mp4</MediaFile></MediaFiles></Linear></Creative></Creatives></InLine></Ad></VAST>"#,
        );
        record_verdict(Some("dsp-b"), &result);

        let rendered = render();
        assert!(rendered.contains("vastlint_grpc_verdicts_total"));
        assert!(rendered.contains("vastlint_grpc_findings_total"));
        assert!(rendered.contains("caller=\"dsp-b\""));
        assert!(rendered.contains("valid=\"false\""));
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.id == "VAST-2.0-inline-impression"),
            "the tally fixture must fire the $ Impression rule"
        );
        assert!(rendered.contains("rule_id=\"VAST-2.0-inline-impression\""));
        assert!(rendered.contains("revenue_impact=\"true\""));
    }

    #[test]
    fn junk_caller_labels_collapse_to_anonymous() {
        let result = core::validate("<VAST version=\"2.0\"></VAST>");
        record_verdict(Some("not a partner!!!"), &result);
        record_verdict(None, &result);

        let rendered = render();
        assert!(rendered.contains("caller=\"anonymous\""));
        assert!(!rendered.contains("not a partner"));
    }

    #[test]
    fn the_registry_is_shared_across_calls() {
        let before = metrics().shed.get();
        record_shed();
        assert_eq!(metrics().shed.get(), before + 1);
    }
}

#[cfg(test)]
mod endpoint_tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    #[tokio::test]
    async fn metrics_http_rejects_other_paths_and_times_out_partial_headers() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let task = tokio::spawn(serve_listener(listener));
        let mut stalled = Vec::new();
        for _ in 0..16 {
            let mut socket = tokio::net::TcpStream::connect(addr).await.unwrap();
            socket
                .write_all(b"GET /metrics HTTP/1.1\r\n")
                .await
                .unwrap();
            stalled.push(socket);
        }
        tokio::time::sleep(std::time::Duration::from_millis(2100)).await;
        for path in ["/metrics", "/other"] {
            let mut socket = tokio::net::TcpStream::connect(addr).await.unwrap();
            socket
                .write_all(format!("GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n").as_bytes())
                .await
                .unwrap();
            let mut response = String::new();
            tokio::time::timeout(
                std::time::Duration::from_secs(2),
                socket.read_to_string(&mut response),
            )
            .await
            .unwrap()
            .unwrap();
            assert!(response.starts_with(if path == "/metrics" {
                "HTTP/1.1 200"
            } else {
                "HTTP/1.1 404"
            }));
        }
        task.abort();
    }
}
