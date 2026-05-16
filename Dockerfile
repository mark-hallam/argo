# syntax=docker/dockerfile:1

FROM rust:1.82-bookworm AS builder

WORKDIR /workspace
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates clang cmake libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY . .
RUN cargo build --locked --release -p argo-cli --bin argo

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
