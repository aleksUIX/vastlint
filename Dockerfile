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

# Copy full source
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

RUN cargo build --release --bin vastlint

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
