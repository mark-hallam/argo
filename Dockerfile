# syntax=docker/dockerfile:1
#
# SKYCOT production / ECS image (the `argo` Rust gateway). Kept at PARITY with
# deploy/Dockerfile.gateway: SAME binary, SAME on-disk runtime deps (provider
# manifests, truth-boundary policies, pitch corpus), SAME runtime libraries. The
# only intentional divergences are the bind/CMD (this image binds non-loopback
# for the ALB/ECS topology + production-auth mode; the gateway image binds
# loopback behind nginx) and the EXPOSE port.
#
# Letting the two images drift produced the "policy-less gateway" failure class:
# the gateway reads truth-boundary policies + the pitch corpus from disk at
# runtime, and a deploy image has no Cargo.toml, so workspace_root() falls back
# to the container cwd and the reads resolve to /app paths that were never
# copied -> a policy-dependent route (e.g. POST /api/v1/pitch/ask) returns
# 503 pitch_unavailable. WS5 #2763 deleted exactly this for the compose path.
# Runbook + the divergence story: docs/skycot/operator/PRODUCTION-RUNBOOK.md.

# ---- builder ----
FROM rust:1.82-bookworm AS builder

WORKDIR /workspace
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates clang cmake libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY . .

# The project's default release profile (lto=fat, codegen-units=1) needs >7.6 GB
# to LINK the ~60-crate argo binary — more than a small build host / CI runner /
# Docker memory cap, so a fat-LTO link OOM-kills the deploy build. Override to
# thin LTO + parallel codegen for the deploy build only (a deploy image doesn't
# need max optimization); a build-local env override leaves the project's default
# release profile unchanged. Mirrors deploy/Dockerfile.gateway.
ENV CARGO_PROFILE_RELEASE_LTO=thin \
    CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16

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

# ---- runtime ----
# debian:bookworm-slim, NOT distroless/cc-debian12: the argo binary dynamically
# links libdbus-1.so.3 (pulled in via the secret-service/keyring credential
# crate) and libssl3. distroless/cc ships neither, so the binary exits 127 at
# load ("libdbus-1.so.3: cannot open shared object file") BEFORE the gateway
# ever binds — the same boot failure deploy/Dockerfile.gateway fixed on its
# 2026-06-12 deploy-smoke first-green. This image installs the identical runtime
# deps so the two images converge.
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates libssl3 libdbus-1-3 && rm -rf /var/lib/apt/lists/*

# Non-root runtime user (uid/gid 10001, "app"). The gateway parses untrusted
# Stripe/Discord/SMS webhooks + runs computer-use, so it must NOT run as root.
# Port 8080 is > 1024 so no CAP_NET_BIND_SERVICE is needed. Kept at PARITY with
# deploy/Dockerfile.gateway (same uid/gid + /data chown).
RUN groupadd -g 10001 app && useradd -u 10001 -g 10001 -m -s /usr/sbin/nologin app

WORKDIR /app
COPY --from=builder /workspace/target/release/argo /usr/local/bin/argo

# On-disk runtime deps. The binary alone is NOT enough: the gateway reads
# provider manifests, truth-boundary policies, and the pitch corpus from disk at
# runtime. Copy ALL FOUR (parity with deploy/Dockerfile.gateway) so a
# policy-dependent route never falls back to a missing workspace_root() path.
COPY manifests     ./manifests
COPY verticals     ./verticals
COPY policies      ./policies
COPY skill-library ./skill-library

# Pin the on-disk dep locations by env so resolution never depends on the
# process cwd / the workspace_root() Cargo.toml walk (which finds nothing in a
# deploy image). ARGO_PITCH_CORPUS_DIR + ARGO_PITCH_POLICY_DIR are read directly
# by routes_pitch_ask.rs corpus_dir()/policy_dir(); ARGO_VERTICALS_DIR matches
# the gateway image. (No ARGO_MANIFEST_DIR: no runtime code reads it; the
# manifests resolve via the WORKDIR /app cwd fallback to /app/manifests.)
ENV ARGO_HOME=/data \
    ARGO_VERTICALS_DIR=/app/verticals \
    ARGO_PITCH_CORPUS_DIR=/app/skill-library/canonical-qa/corpora/skycot \
    ARGO_PITCH_POLICY_DIR=/app/policies/truth-boundary \
    RUST_LOG=info

# Make ARGO_HOME (/data) writable by the non-root uid on a FRESH empty volume.
# A bare VOLUME/bind mount comes up root-owned; chowning at build time covers
# the docker-run path. On k8s the PVC mount is covered by fsGroup: 10001 in the
# Deployment securityContext (the in-image chown does not reach a PVC mount).
RUN mkdir -p /data && chown 10001:10001 /data

EXPOSE 8080
# Drop to the non-root user for the runtime process.
USER 10001
ENTRYPOINT ["/usr/local/bin/argo"]
CMD ["gateway", "start", "--bind", "0.0.0.0", "--bind-non-loopback", "--accept-insecure-bind-warning", "--port", "8080", "--quiet"]
