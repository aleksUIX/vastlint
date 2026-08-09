//! vastlint gRPC server.
//!
//! ```text
//! VASTLINT_GRPC_ADDR=0.0.0.0:50051 vastlint-grpc
//! grpcurl -plaintext localhost:50051 list
//! ```
//!
//! Reflection is enabled, so `grpcurl` needs no local copy of the proto, and
//! `grpc.health.v1` is served for readiness probes.

use std::net::SocketAddr;

use mimalloc::MiMalloc;
use tonic::transport::Server;
use vastlint_grpc::proto::vastlint_service_server::VastlintServiceServer;
use vastlint_grpc::proto::FILE_DESCRIPTOR_SET;
use vastlint_grpc::service::VastlintApi;

/// Validation builds an owned document tree per call, so under concurrency the
/// system allocator's shared free list becomes the bottleneck before the
/// validator does. Same reasoning as vastlint-mcp.
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

/// Default listen address. Binds all interfaces because the deployment target
/// is a container.
const DEFAULT_ADDR: &str = "0.0.0.0:50051";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr: SocketAddr = std::env::var("VASTLINT_GRPC_ADDR")
        .unwrap_or_else(|_| DEFAULT_ADDR.to_string())
        .parse()?;

    let service = VastlintApi::new();

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

    eprintln!(
        "vastlint-grpc {} listening on {addr}",
        env!("CARGO_PKG_VERSION")
    );
    eprintln!("catalog {}", vastlint_grpc::provenance::catalog_digest());

    Server::builder()
        .add_service(health_service)
        .add_service(reflection_v1)
        .add_service(reflection_v1alpha)
        .add_service(VastlintServiceServer::new(service))
        .serve_with_shutdown(addr, shutdown())
        .await?;

    Ok(())
}

/// Stops accepting on SIGINT so in-flight requests finish rather than being cut
/// off mid-response. Kubernetes sends SIGTERM, which is handled the same way.
async fn shutdown() {
    let _ = tokio::signal::ctrl_c().await;
    eprintln!("shutting down");
}
