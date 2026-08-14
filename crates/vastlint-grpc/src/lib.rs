//! gRPC server for vastlint.
//!
//! Serves `openadtech.vastlint.v1` over HTTP/2, with server reflection and
//! `grpc.health.v1`. The transport layer is a thin adapter: every RPC delegates
//! to `vastlint-core` and holds no validation logic of its own, so the gRPC
//! surface and the CLI cannot disagree about what a document means.
//!
//! The wire contract lives in `proto/openadtech/vastlint/v1/vastlint.proto` and
//! is not generated from the Rust types. `buf breaking` guards it in CI.

pub mod config;
pub mod convert;
pub mod deadline;
pub mod events;
pub mod limit;
pub mod metrics;
pub mod provenance;
pub mod ratelimit;
pub mod service;

/// Generated bindings for `openadtech.vastlint.v1`.
pub mod proto {
    #![allow(clippy::doc_overindented_list_items)]

    tonic::include_proto!("openadtech.vastlint.v1");

    /// The encoded file descriptor set, served by reflection so that `grpcurl`
    /// can describe and call this server with no local copy of the proto.
    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("vastlint_descriptor");
}
