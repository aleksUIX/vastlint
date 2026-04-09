# ─────────────────────────────────────────────────────────────────────────────
# Stage 1 — build a fully-static musl binary
#
# rust:alpine is the right base for musl static builds: Alpine ships a native
# musl toolchain so crates with C code (ring, via ureq/rustls) compile cleanly.
# The debian-slim + musl-tools approach breaks ring because that musl-gcc
# wrapper does not support the -m64 flag that ring's build script passes.
# ─────────────────────────────────────────────────────────────────────────────
FROM rust:alpine AS builder

# Alpine's native musl + build essentials for crates with C dependencies (ring)
RUN apk add --no-cache musl-dev gcc make perl

WORKDIR /build

# Cache dependency compilation separately from source changes.
# Copy manifests first so this layer is only invalidated when deps change.
COPY Cargo.toml Cargo.lock ./
COPY crates/vastlint-cli/Cargo.toml   crates/vastlint-cli/Cargo.toml
COPY crates/vastlint-core/Cargo.toml  crates/vastlint-core/Cargo.toml
COPY crates/vastlint-ffi/Cargo.toml   crates/vastlint-ffi/Cargo.toml
COPY crates/vastlint-wasm/Cargo.toml  crates/vastlint-wasm/Cargo.toml

# Stub out every crate so Cargo can resolve and compile all dependencies
# without the real source.  The stubs are replaced by the real COPY below.
RUN for crate in vastlint-cli vastlint-core vastlint-ffi vastlint-wasm; do \
      mkdir -p crates/$crate/src; \
      echo 'fn main() {}' > crates/$crate/src/main.rs; \
      echo '' > crates/$crate/src/lib.rs; \
    done

RUN cargo build --release --bin vastlint 2>/dev/null || true

# Now copy the real source and rebuild only what changed
COPY crates/ crates/

RUN touch crates/vastlint-cli/src/main.rs \
 && cargo build --release --bin vastlint

# ─────────────────────────────────────────────────────────────────────────────
# Stage 2 — final image: scratch + the static binary only
#
# "scratch" is the absolute minimum: zero OS, zero shell, zero attack surface.
# The binary is fully self-contained so nothing else is needed.
# ─────────────────────────────────────────────────────────────────────────────
FROM scratch

# Copy the static binary
COPY --from=builder \
     /build/target/release/vastlint \
     /vastlint

# /data is the conventional mount point for XML files
VOLUME ["/data"]

# Default: read from stdin, output plain text.
# Override at runtime:
#   docker run --rm vastlint vastlint check /data/tag.xml --format json
ENTRYPOINT ["/vastlint"]
CMD ["check", "-"]
