# Mission load-bearing wire — Gateway TLS termination + reverse-proxy doc

**Owner**: claude. **Depends on**: existing gateway `boot.rs` socket listener (verify path via `find crates -name boot.rs -path '*gateway*'`), `argo-config` sections.
**Authored against**: origin/main `01aa2f5d` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-gateway-tls-termination-or-proxy-doc`.

> SOURCE: MISSION_BACKLOG.md Section 1 #20 ("Add TLS termination support OR document explicit reverse-proxy requirement"). **Tier M (security).** Today the gateway binds plaintext HTTP on 127.0.0.1:7400. Operators who expose it externally MUST front it with a reverse-proxy that terminates TLS — but nothing in the codebase or docs says this clearly, and there's no fail-loud path when an operator misconfigures. This brief either lands native TLS OR locks down the no-TLS path so it can't accidentally be exposed.

## Scope

The brief intentionally offers a fork — implementer picks based on the time budget:

**Path A (preferred, ~600 LOC): native TLS termination** in the gateway listener. Add `[gateway.tls]` config with `cert_pem_path` + `key_pem_path` (file paths) + optional `client_ca_pem_path` for mTLS. Wrap the TcpListener accept loop in `tokio-rustls` server-side handshake. When `[gateway.tls].enabled = true` AND the gateway bind address is NOT `127.0.0.1`, ALLOW startup; when bind is non-loopback AND `tls.enabled = false`, REFUSE TO BOOT with a load-bearing error pointing at this brief's docs.

**Path B (fallback, ~200 LOC): document + enforce reverse-proxy contract.** No native TLS, but add:
- `[gateway.behind_proxy]` config block w/ `expected_proxy: bool = false`, `trusted_proxies: Vec<IpAddr>` (the only IPs allowed to set X-Forwarded-For)
- Refuse-to-boot when bind is non-loopback AND `expected_proxy = false`
- Honor X-Forwarded-For ONLY when peer IP is in `trusted_proxies` (else log warn and use peer IP)
- NEW runbook `docs/skycot/runbook/gateway-tls-deployment.md` covering caddy + nginx + traefik + envoy + cloudflare-tunnel front-end recipes

Whichever path the implementer picks, the BIND-NON-LOOPBACK guard is the load-bearing acceptance criterion: today an operator can run `argo gateway start --bind 0.0.0.0:7400` and SKYCOT happily serves plaintext to the world. After this brief lands, that command exits non-zero with a clear error pointing at the runbook.

This is NOT in scope: full Let's Encrypt ACME integration (separate); SNI-based multi-tenant cert serving (separate); HSTS / HPKP header policy (separate Tier M brief); certificate-pinning for outbound provider calls (separate); WebSocket-over-TLS specifically (covered by Path A through tokio-rustls' generic upgrade path).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

### Path A files (native TLS):
- `crates/argo-cli/src/handlers/gateway/tls.rs` (NEW, ≤ 220 LOC) — `pub async fn wrap_acceptor` + cert/key loader
- `crates/argo-cli/src/handlers/gateway/tls_tests.rs` (NEW, sibling, ≤ 180 LOC) — 6 unit tests (load-cert / load-key / mtls-presented / mtls-required-but-missing / cert-expired-rejected / bind-loopback-allowed-without-tls)
- `crates/argo-cli/src/handlers/gateway/boot.rs` — narrowly scoped: ADD wrap-acceptor hook when `[gateway.tls].enabled = true` (≤ 30 LOC delta)
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly scoped: register `pub mod tls;`
- `crates/argo-cli/Cargo.toml` — narrowly scoped: ADD `tokio-rustls` + `rustls-pemfile` workspace deps if not present

### Path B files (proxy doc enforcement):
- `crates/argo-cli/src/handlers/gateway/proxy_guard.rs` (NEW, ≤ 150 LOC) — `pub fn validate_bind_address` + `pub fn extract_real_ip`
- `crates/argo-cli/src/handlers/gateway/proxy_guard_tests.rs` (NEW, sibling, ≤ 120 LOC) — 5 unit tests
- `crates/argo-cli/src/handlers/gateway/boot.rs` — narrowly scoped: ADD validate-bind hook + X-Forwarded-For extractor wired into existing request handling (≤ 25 LOC delta)
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly scoped: register `pub mod proxy_guard;`
- `docs/skycot/runbook/gateway-tls-deployment.md` (NEW, ≤ 280 LOC) — caddy / nginx / traefik / envoy / cloudflare-tunnel recipes

### Shared (both paths):
- `crates/argo-config/src/sections.rs` — narrowly scoped: ADD `[gateway.tls]` OR `[gateway.behind_proxy]` config block (≤ 35 LOC)
- `crates/argo-config/src/defaults.rs` — narrowly scoped: ADD default values
- `crates/argo-cli/src/handlers/gateway/startup_check.rs` — narrowly scoped: ADD the bind-non-loopback refuse-to-boot guard (load-bearing, must be present whichever path is taken; ≤ 30 LOC delta)
- `crates/argo-cli/tests/gateway_bind_safety_smoke.rs` (NEW, ≤ 120 LOC) — integration smoke: `argo gateway start --bind 0.0.0.0:0` exits non-zero w/o tls/proxy config; with config, boots cleanly

## Files you must NOT touch

- `crates/argo-store/migrations/**` — no schema change
- `crates/argo-channels/**` — orthogonal
- Any vertical content

## Acceptance criteria

- [ ] **Load-bearing**: `argo gateway start --bind 0.0.0.0:7400` exits NON-ZERO with a clear error message + runbook link when no TLS/proxy config is set
- [ ] **Load-bearing**: `argo gateway start --bind 127.0.0.1:7400` boots successfully (loopback is always safe — local dev unaffected)
- [ ] **Path A only**: `[gateway.tls].enabled = true` + valid cert/key paths → gateway serves HTTPS on the configured bind
- [ ] **Path A only**: invalid cert path → refuse-to-boot with `argo doctor`-shape error pointing at the offending path
- [ ] **Path A only**: mTLS-required + client doesn't present cert → 403 (not 200)
- [ ] **Path B only**: `[gateway.behind_proxy].expected_proxy = true` allows non-loopback bind; X-Forwarded-For only honored from `trusted_proxies`
- [ ] **Path B only**: runbook covers 5 reverse-proxy products with working config snippets
- [ ] Implementer documents in PR body which path they took and why (time budget / risk appetite / explicit operator preference)
- [ ] 5-6 unit tests on the chosen path + 1 integration smoke for the bind-safety guard
- [ ] `cargo clippy -p argo-config -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-config -p argo-cli` passes
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-gateway-tls-termination-or-proxy-doc`
- PR title (Path A): `gateway: TLS termination + non-loopback bind guard (mission-gateway-tls-termination-or-proxy-doc)`
- PR title (Path B): `gateway: reverse-proxy contract enforcement + deployment runbook (mission-gateway-tls-termination-or-proxy-doc)`

## Notes for the implementer

- **Pick Path A if** you have ≥ 4 hours of clean-build budget + you're confident with `tokio-rustls`. The wrapping is mechanical but cert hot-reload is intentionally OUT of scope (operators must restart for cert rotation — document this as a HARD edge in module docs).
- **Pick Path B if** time-constrained or if you suspect cert hot-reload will become a follow-up brief soon — Path A locks in the "restart for rotation" trade-off and Path B keeps that option open.
- The BIND-NON-LOOPBACK guard is the load-bearing acceptance criterion: it must be present in BOTH paths. The "today" failure mode is `argo gateway start --bind 0.0.0.0` serving plaintext to the world; this brief closes that hole regardless of path.
- For Path A: cert/key loaded via `rustls-pemfile::certs` + `rustls-pemfile::pkcs8_private_keys`. Build the `ServerConfig` with `with_safe_defaults()` for cipher selection. Don't expose cipher-suite picker to operators in v1.
- For Path B: `trusted_proxies` MUST be checked against `TcpStream::peer_addr` (the TCP-layer source). If peer IP matches `127.0.0.1` AND `expected_proxy = true`, that's the typical caddy/nginx-on-same-host pattern; that case should work without listing 127.0.0.1 in `trusted_proxies` explicitly (default-allow loopback).
- The runbook (Path B file) is GOLD if Path A is chosen — the same recipes are still useful for operators who want to front a TLS gateway with a CDN. So if Path A is chosen, still ship the runbook with the CDN/cloudflare-tunnel sections.
- Document the choice in the PR body's "Sensible deviations" section. Future implementers reading this brief should not assume one path was correct; the trade-off is intentional.
