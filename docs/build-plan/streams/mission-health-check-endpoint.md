# Mission load-bearing wire — `/health` endpoint for gateway (k8s/systemd readiness)

**Owner**: claude. **Depends on**: existing gateway routes layout (verified — `crates/argo-cli/src/handlers/gateway/` holds route handlers; `crates/argo-cli/tests/mcp_health_smoke.rs` exists already for MCP health but no gateway health).
**Authored against**: origin/main `f8f51df4` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-health-check-endpoint`.

> SOURCE: MISSION_BACKLOG.md Section 1 #38 — "Add health-check endpoint for k8s/systemd readiness." Today the gateway boots and serves application routes but provides no dedicated readiness/liveness endpoint. Mission promise: `GET /health` returns 200 with a minimal JSON when the gateway is ready to serve traffic, 503 with a reason when not.

## Scope

1. **NEW gateway route**: `GET /health` — returns:
   - **200** with `{"status":"ok","schema_version":<N>,"uptime_seconds":<S>}` when all readiness checks pass.
   - **503** with `{"status":"degraded","reason":"<reason>"}` on failure.

2. **Readiness checks** (sequential, short-circuit on first failure):
   - SQLite store ping: `SELECT 1` round-trip < 100ms.
   - Schema version matches `EXPECTED_SCHEMA_VERSION`.
   - Provider config loaded (at least one provider entry present).
   - Auth keystore reachable (the config sets the bearer secret or `ARGO_GATEWAY_AUTH_BYPASS=1`).

3. **Separate `/livez`** (k8s convention): minimal liveness probe — returns 200 if the process is up, **no readiness checks**. Useful to distinguish "alive but unready" from "dead."

4. **`/readyz`** alias to `/health` (k8s convention). Same body shape.

5. **Operator inspection**: existing `argo doctor` (verify path) — narrowly scoped: ADD a section that hits the local gateway `/health` if running, shows the response inline. ≤ 30 LOC delta.

This is NOT in scope: per-tenant health; dependency health (provider liveness, MCP server liveness — those are separate); auth on the health endpoints (intentionally unauthenticated — k8s/systemd needs them callable without credentials); metrics export on these endpoints (separate brief).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_health.rs` (NEW, ≤ 200 LOC) — the 3 endpoint handlers + `HealthCheck::run`.
- `crates/argo-cli/src/handlers/gateway/routes_health_tests.rs` (NEW, sibling, ≤ 150 LOC) — 6 unit tests: ok-all-checks-pass; degraded-schema-mismatch; degraded-store-timeout; livez-always-ok; readyz-aliases-health; uptime-counter.
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly scoped: ADD route registration for `/health`, `/livez`, `/readyz`. ≤ 20 LOC delta.
- `crates/argo-cli/tests/gateway_health_smoke.rs` (NEW, ≤ 200 LOC) — 1 integration smoke: boot gateway, hit each endpoint, assert response shape.
- `crates/argo-cli/src/handlers/doctor.rs` (verify path) — narrowly scoped: ADD optional /health probe section. ≤ 30 LOC delta.

## Files you must NOT touch

- `crates/argo-cli/src/handlers/gateway/auth.rs` (or wherever auth middleware lives) — health endpoints bypass auth; explicitly mark in route registration
- `crates/argo-store/migrations/**` — no schema change
- Any non-cli crate

## Acceptance criteria

- [ ] `GET /health` returns 200 + correct body on happy path
- [ ] `GET /health` returns 503 + reason on each failure mode (4 scenarios)
- [ ] `GET /livez` returns 200 regardless of readiness checks
- [ ] `GET /readyz` aliases `/health` byte-identically
- [ ] Endpoints are unauthenticated (no bearer required)
- [ ] `uptime_seconds` increments correctly
- [ ] `argo doctor` shows /health response when gateway is local-running
- [ ] 6 unit tests + 1 integration smoke
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-cli` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-health-check-endpoint`
- PR title: `gateway: /health + /livez + /readyz endpoints for k8s/systemd readiness (mission-health-check-endpoint)`

## Notes for the implementer

- The 100ms SQLite-ping budget is a starting point — make it a tunable constant in the route module.
- The "auth bypass" behavior is **load-bearing**: orchestrators need to call /health without credentials. Document in PR body which middleware layer the route sits below.
- For the integration smoke: reuse the existing gateway-boot harness (e.g. `gateway_security_headers_smoke.rs` shows the pattern).
- The uptime counter: use `Instant::now()` captured at gateway start and threaded into the route handler via app state. Don't use `chrono::Utc::now` (gives wall-clock not monotonic).
- The doctor integration should fail GRACEFULLY (warn + continue) if gateway isn't running locally — don't crash the doctor command.
