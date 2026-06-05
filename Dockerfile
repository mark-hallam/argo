# syntax=docker/dockerfile:1

FROM rust:1.82-bookworm AS builder

WORKDIR /workspace
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates clang cmake libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY . .

# Build with SQLCipher by default. In production mode (the deploy binds
# non-loopback / sets ARGO_GATEWAY_PROFILE=production) the gateway runs a
# fail-closed at-rest-encryption gate (argo-store store_open.rs
# enforce_enterprise_encryption) that REFUSES TO BOOT unless the binary is
# SQLCipher-linked AND a DB key (SKYCOT_DB_KEY_BASE64) is set. A stock build
# crash-loops at boot in production. A SQLCipher-linked binary is safe in dev
# mode too (the gate doesn't fire without production mode). Override to "" for
# a faster stock dev image.
ARG CARGO_FEATURES=bundled-sqlcipher
RUN cargo build --locked --release -p argo-cli --bin argo --features "${CARGO_FEATURES}"

FROM gcr.io/distroless/cc-debian12

WORKDIR /app
ENV ARGO_HOME=/data \
    ARGO_VERTICALS_DIR=/app/verticals \
    RUST_LOG=info

COPY --from=builder /workspace/target/release/argo /usr/local/bin/argo
COPY manifests ./manifests
COPY verticals ./verticals

EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/argo"]
CMD ["gateway", "start", "--bind", "0.0.0.0", "--bind-non-loopback", "--accept-insecure-bind-warning", "--port", "8080", "--quiet"]
