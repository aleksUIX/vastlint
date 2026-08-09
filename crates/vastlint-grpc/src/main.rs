//! vastlint gRPC server.
//!
//! ```text
//! VASTLINT_GRPC_ADDR=0.0.0.0:50051 vastlint-grpc
//! grpcurl -plaintext localhost:50051 list
//! curl localhost:9090/metrics
//! ```
//!
//! Reflection is enabled, so `grpcurl` needs no local copy of the proto, and
//! `grpc.health.v1` is served for readiness probes.

use std::sync::Arc;

use mimalloc::MiMalloc;
use tonic::transport::Server;
use vastlint_grpc::config::Config;
use vastlint_grpc::events::{NullSink, Publisher, Sink};
use vastlint_grpc::limit::{AdaptiveConcurrencyLayer, AdaptiveLimiter};
use vastlint_grpc::metrics;
use vastlint_grpc::proto::vastlint_service_server::VastlintServiceServer;
use vastlint_grpc::proto::FILE_DESCRIPTOR_SET;
use vastlint_grpc::ratelimit::{RateLimitLayer, RateLimiter};
use vastlint_grpc::service::VastlintApi;

/// Validation builds an owned document tree per call, so under concurrency the
/// system allocator's shared free list becomes the bottleneck before the
/// validator does. Same reasoning as vastlint-mcp.
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;

    // The runtime is built by hand rather than through `#[tokio::main]` so the
    // blocking pool can be sized. That pool is where validation actually runs,
    // and its default ceiling of 512 threads is sized for blocking I/O, not for
    // CPU-bound work. Leaving it at the default means 512 concurrent
    // validations on a machine with a dozen cores, which does not raise
    // throughput, only latency and context switches, and it puts the real
    // admission decision somewhere nobody configured.
    let mut runtime = tokio::runtime::Builder::new_multi_thread();
    runtime
        .enable_all()
        .max_blocking_threads(config.blocking_threads);
    if let Some(threads) = config.worker_threads {
        runtime.worker_threads(threads);
    }

    runtime.build()?.block_on(serve(config))
}

async fn serve(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let limiter = Arc::new(AdaptiveLimiter::new(config.limit.clone()));
    let rate_limiter = Arc::new(RateLimiter::new(config.rate_limit.clone()));
    metrics::set_concurrency_limit(limiter.limit());

    // Health reporting. Marked serving immediately: the rule catalog is static
    // and there is no warm-up, so there is no state in which the process is up
    // but unable to answer.
    let (health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<VastlintServiceServer<VastlintApi>>()
        .await;

    // v1 and v1alpha are both registered. grpcurl and several older tools still
    // speak v1alpha, and serving only v1 makes the server look like it has no
    // reflection at all to those clients.
    //
    // The health descriptor is registered alongside vastlint's own. Reflection
    // only reports what is in its descriptor pool, so without this the health
    // service is served but undiscoverable: `grpcurl list` omits it and calling
    // it fails with "target server does not expose service grpc.health.v1.Health"
    // even though it is right there. Anything that probes by reflection rather
    // than by a vendored copy of the health proto would conclude the server has
    // no health checking.
    let reflection_v1 = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(tonic_health::pb::FILE_DESCRIPTOR_SET)
        .build_v1()?;
    let reflection_v1alpha = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(tonic_health::pb::FILE_DESCRIPTOR_SET)
        .build_v1alpha()?;

    // Built before anything is bound or printed. A misconfigured results stream
    // should fail while the process is still obviously starting up, not after a
    // banner that says the server is listening.
    let publisher = if config.events.enabled {
        let sink: Arc<dyn Sink> = build_sink(&config)?;
        Some(Arc::new(Publisher::spawn(
            sink,
            config.events.buffer,
            config.events.schema_id,
        )))
    } else {
        None
    };

    if let Some(metrics_addr) = config.metrics_addr {
        tokio::spawn(async move {
            if let Err(error) = metrics::serve(metrics_addr).await {
                eprintln!("metrics endpoint stopped: {error}");
            }
        });
        eprintln!("metrics on http://{metrics_addr}/metrics");
    }

    banner(&config, &limiter);

    // A message cap on the decoder, not just on the handler. Rejecting a 40 MB
    // body after decoding it has already paid the cost the cap exists to avoid.
    // The streaming handler gets the limiter directly, because streams are
    // exempt from the tower layer: one long-lived stream would otherwise hold a
    // concurrency slot for its whole lifetime and report its entire duration as
    // a latency sample, driving the limit to the floor. Admission for streams
    // happens per message instead.
    let mut api = VastlintApi::new().with_limiter(Arc::clone(&limiter), config.stream_buffer);
    if let Some(publisher) = publisher {
        api = api.with_publisher(publisher, config.events.schema_id);
    }

    let vastlint =
        VastlintServiceServer::new(api).max_decoding_message_size(config.max_message_bytes);

    let mut server = Server::builder();
    if let Some(bytes) = config.stream_window_bytes {
        server = server.initial_stream_window_size(bytes);
    }
    if let Some(bytes) = config.connection_window_bytes {
        server = server.initial_connection_window_size(bytes);
    }

    server
        // Rate limiting sits outside the concurrency limiter on purpose. A
        // caller over its allowance should be refused before it can occupy a
        // concurrency slot, otherwise one client in a retry storm crowds out
        // everyone else while staying under the aggregate limit.
        .layer(RateLimitLayer::new(
            Arc::clone(&rate_limiter),
            config.caller_header.clone(),
        ))
        .layer(AdaptiveConcurrencyLayer::new(Arc::clone(&limiter)))
        .add_service(health_service)
        .add_service(reflection_v1)
        .add_service(reflection_v1alpha)
        .add_service(vastlint)
        .serve_with_shutdown(config.addr, shutdown())
        .await?;

    Ok(())
}

/// Chooses where validation events go.
///
/// With the `kafka` feature off, or with no brokers configured, events are
/// still built and encoded and then discarded. That is deliberate: it keeps the
/// encoding path exercised in every deployment, so a schema problem surfaces
/// wherever the server runs rather than only where a broker happens to exist.
fn build_sink(config: &Config) -> Result<Arc<dyn Sink>, Box<dyn std::error::Error>> {
    if config.events.brokers.is_empty() {
        eprintln!("events: enabled with no brokers, encoding and discarding");
        return Ok(Arc::new(NullSink::default()));
    }

    #[cfg(feature = "kafka")]
    {
        let sink = vastlint_grpc::events::kafka::KafkaSink::new(
            &config.events.brokers,
            &config.events.topic,
        )?;
        eprintln!(
            "events: publishing to {} topic {}",
            config.events.brokers, config.events.topic
        );
        Ok(Arc::new(sink))
    }

    #[cfg(not(feature = "kafka"))]
    {
        // Refusing rather than silently discarding. An operator who set brokers
        // expects records to arrive, and a binary built without the feature
        // cannot deliver them. Starting anyway would look like success.
        Err(format!(
            "VASTLINT_KAFKA_BROKERS is set to {:?} but this binary was built without the \
             `kafka` feature; rebuild with --features kafka or unset the brokers",
            config.events.brokers
        )
        .into())
    }
}

/// Prints the effective configuration at startup.
///
/// Every value that changes how the server behaves under load, printed once, so
/// that a latency graph can be matched to the settings that produced it. An
/// experiment whose configuration was never recorded is an anecdote.
fn banner(config: &Config, limiter: &AdaptiveLimiter) {
    eprintln!(
        "vastlint-grpc {} listening on {}",
        env!("CARGO_PKG_VERSION"),
        config.addr
    );
    eprintln!("catalog {}", vastlint_grpc::provenance::catalog_digest());

    if config.limit.enabled {
        eprintln!(
            "concurrency limit: adaptive, starting at {} in [{}, {}], target latency {:?}, backoff {}",
            limiter.limit(),
            config.limit.min,
            config.limit.max,
            config.limit.target_latency,
            config.limit.backoff_ratio,
        );
    } else {
        eprintln!("concurrency limit: DISABLED, no shedding");
    }

    if config.rate_limit.per_second > 0 {
        eprintln!(
            "rate limit: {}/s per caller, burst {}, identified by {}",
            config.rate_limit.per_second, config.rate_limit.burst, config.caller_header,
        );
    } else {
        eprintln!("rate limit: disabled");
    }

    eprintln!("max message size: {} bytes", config.max_message_bytes);
    eprintln!(
        "stream buffer: {} messages in flight, windows {}/{}",
        config.stream_buffer,
        config
            .stream_window_bytes
            .map(|bytes| bytes.to_string())
            .unwrap_or_else(|| "default".to_string()),
        config
            .connection_window_bytes
            .map(|bytes| bytes.to_string())
            .unwrap_or_else(|| "default".to_string()),
    );
    eprintln!(
        "validation threads: {} ({} async workers)",
        config.blocking_threads,
        config
            .worker_threads
            .map(|threads| threads.to_string())
            .unwrap_or_else(|| "default".to_string())
    );
}

/// Stops accepting on SIGINT so in-flight requests finish rather than being cut
/// off mid-response. Kubernetes sends SIGTERM, which is handled the same way.
async fn shutdown() {
    let _ = tokio::signal::ctrl_c().await;
    eprintln!("shutting down");
}
