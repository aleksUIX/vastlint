//! Generates the `openadtech.vastlint.v1` bindings and the encoded file
//! descriptor set.
//!
//! The descriptor set is what server reflection serves, so `grpcurl` can
//! describe and call this server without a local copy of the proto. Same
//! approach as the reflection support added to the ARTF Rust reference
//! implementation.
//!
//! Compilation goes through `protox`, a protobuf compiler written in Rust,
//! rather than shelling out to `protoc`. That is not a style preference. The
//! default `prost-build` path requires a `protoc` binary on `PATH` at build
//! time, which would mean every CI runner across three operating systems needs
//! a new install step, and, worse, anyone running `cargo install vastlint-grpc`
//! from crates.io would need it too. A validator that will not build without an
//! undeclared system dependency is not one distribution channel, it is a
//! support burden.

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../proto");
    let proto = proto_root.join("openadtech/vastlint/v1/vastlint.proto");
    let descriptor_set = PathBuf::from(std::env::var("OUT_DIR")?).join("vastlint_descriptor.bin");

    // Rebuild when the contract changes. Without this, editing the proto leaves
    // a stale generated module in place and the mismatch surfaces as a
    // confusing type error somewhere else.
    println!("cargo:rerun-if-changed={}", proto.display());

    let file_descriptors = protox::compile([&proto], [&proto_root])?;

    // Written by hand rather than via `file_descriptor_set_path`, because that
    // option belongs to the protoc path we are deliberately not taking.
    std::fs::write(&descriptor_set, {
        use prost::Message;
        file_descriptors.encode_to_vec()
    })?;

    tonic_prost_build::configure()
        .build_server(true)
        // The client is generated too, so integration tests exercise the real
        // wire path rather than calling the service implementation directly.
        .build_client(true)
        .compile_fds(file_descriptors)?;

    Ok(())
}
