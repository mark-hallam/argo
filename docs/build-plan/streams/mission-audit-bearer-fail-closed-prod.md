# Stream Audit-P0-3 — bearer auth fail-closed in production / non-loopback mode

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `d2e4a579` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-audit-bearer-fail-closed-prod`.
**Source**: 2026-05-19 codex audit P1-3, validated by claude `VALIDATION.md` (CONFIRMED — `routes_auth.rs:31-36` returns `None` when `ARGO_GATEWAY_BEARER_TOKEN` is unset, no production-mode guard).

## Scope

When `ARGO_GATEWAY_BEARER_TOKEN` is unset or empty, `check_bearer_auth` in `crates/argo-cli/src/handlers/gateway/routes_auth.rs:31-36` returns `None` (auth passes). This is the dev default. There is **no startup check** that catches a production deployment with an unset token — the gateway will silently boot fail-open on a non-loopback bind.

Additionally, `routes_auth.rs:85-92` adds `/api/v1/wizard` to the public-GET allowlist (the first-run setup wizard) — convenient for dev, but in production an attacker can hit the wizard and complete setup before the operator does.

This brief: (a) detects "production mode" (any non-loopback bind, or explicit `ARGO_GATEWAY_PROFILE=production`) and refuses to start with `ARGO_GATEWAY_BEARER_TOKEN` unset; (b) protects `/api/v1/wizard` with a one-time bootstrap token (generated on first start, written to `~/.argo/.bootstrap-token`, required as `X-Argo-Bootstrap` header for any wizard request); (c) makes the dev bypass loud — `tracing::warn!` at every bypassed request, not just at boot.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_auth.rs` — change `check_bearer_auth` behavior when token is unset: if `is_production_mode()` → return `Reject(401 "gateway-bearer-token-required-in-production")`; else → return `Pass(DevBypass)` and emit `tracing::warn!("auth bypass: bearer token unset, dev mode only")` on every request (not just at boot). The `is_public_get` allowlist no longer includes `/api/v1/wizard` unconditionally — wizard routes route through a new `check_bootstrap_token` helper.
- `crates/argo-cli/src/handlers/gateway/routes_auth_production_mode.rs` (NEW) — `pub fn is_production_mode() -> bool`. True if any of: `ARGO_GATEWAY_PROFILE=production`, `ARGO_GATEWAY_BIND` is set and parses to a non-loopback address, or `ARGO_GATEWAY_BIND` is unset and the default bind in `routes.rs` is non-loopback. Single source of truth; called from boot + auth.
- `crates/argo-cli/src/handlers/gateway/boot.rs` — at startup, call `is_production_mode()`; if true and `ARGO_GATEWAY_BEARER_TOKEN` is unset, log `ERROR` + return `Err(ArgoError::Config("ARGO_GATEWAY_BEARER_TOKEN required when binding non-loopback or ARGO_GATEWAY_PROFILE=production"))`. Boot fails closed; no silent fall-through.
- `crates/argo-cli/src/handlers/gateway/bootstrap_token.rs` (NEW) — `pub fn ensure_bootstrap_token() -> ArgoResult<String>` reads / writes `~/.argo/.bootstrap-token` (0600 perms on Unix). `pub fn check_bootstrap_token(req) -> Option<Response>` validates `X-Argo-Bootstrap` header against the file; mismatch returns `401`. Token rotation: `argo gateway bootstrap rotate` regenerates it.
- `crates/argo-cli/src/handlers/gateway/routes.rs` — for wizard paths, call `check_bootstrap_token` before falling through to bearer auth. Once bearer token is configured + wizard has been completed (sentinel file `~/.argo/.wizard-done`), wizard routes return `410 Gone`.
- `crates/argo-cli/tests/gateway_fail_closed_prod_smoke.rs` (NEW) — four cases: (1) `ARGO_GATEWAY_PROFILE=production` + bearer unset → boot returns `Err` with helpful message; (2) wizard route without bootstrap token in production → 401; (3) wizard route with bootstrap token → 200; (4) wizard route after `wizard-done` sentinel → 410.

## Files you must NOT touch

- `routes_auth_principal.rs` / `routes_skill_ir.rs` / `routes_ghostwriter.rs` — those are owned by `mission-audit-principal-tenant-from-token`.
- The `__Host-session` cookie path (PR #1835's scope) — this brief is about bearer auth, not session auth.
- Any web/frontend code — the SPA's signup wizard runs over magic-link, not the legacy `/api/v1/wizard`.

## Acceptance criteria

- [ ] `is_production_mode()` returns true under any of: `ARGO_GATEWAY_PROFILE=production`, `ARGO_GATEWAY_BIND` set to a non-loopback IP, OR default-bind is non-loopback (currently `0.0.0.0:8080` per `boot.rs`).
- [ ] Boot fails with a clear error if `is_production_mode() && ARGO_GATEWAY_BEARER_TOKEN.is_empty()`.
- [ ] Dev bypass emits `tracing::warn!` on every request, not just at boot.
- [ ] `/api/v1/wizard/*` requires `X-Argo-Bootstrap` header matching `~/.argo/.bootstrap-token` UNTIL `~/.argo/.wizard-done` exists; after that, wizard returns `410 Gone`.
- [ ] `argo gateway bootstrap rotate` regenerates the bootstrap token + emits an audit row.
- [ ] Four new test cases in `gateway_fail_closed_prod_smoke.rs` pass.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean — claim in PR body.
- [ ] `cargo test -p argo-cli --test gateway_fail_closed_prod_smoke` passes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.
- [ ] Operator docs note added (or `// CONFIG-REFERENCE:` comment in `routes_auth_production_mode.rs`) describing the production-mode detection rules and the bootstrap-token flow.

## Branch + PR

- Branch: `codex-mission-audit-bearer-fail-closed-prod`
- PR title: `gateway: fail-closed bearer auth in production + bootstrap-token-gated wizard (mission-audit-bearer-fail-closed-prod)`

## Notes for the implementer

- The bootstrap-token file lives at `~/.argo/.bootstrap-token`. On Unix, set mode `0600` at write time. On Windows, leave default ACL. Document the path.
- The wizard sentinel `~/.argo/.wizard-done` should be created by the wizard's "complete" endpoint, not externally. Once it exists, the wizard is permanently disabled (404/410).
- Be careful with `is_production_mode()` — the default bind on a fresh dev install often parses to `0.0.0.0`. The conservative interpretation: any default of `0.0.0.0` IS production mode. An operator wanting local-only dev sets `ARGO_GATEWAY_BIND=127.0.0.1:<port>` explicitly. Document this.
- Boot error message must include the offending env var name (`ARGO_GATEWAY_BEARER_TOKEN`) and a one-line "see docs/skycot/operator/security.md" pointer. If that doc doesn't exist, leave a `// DOC-MISSING:` marker.
- The "loud dev bypass" warn should be rate-limited (e.g. once per minute, not per request) or it floods logs. Use `tracing::warn!` with a `static AtomicU64` last-warn-timestamp.

## Out of scope (follow-up briefs)

- mTLS or other transport-layer auth — leave for v1.x enterprise hardening.
- Bootstrap token revocation across multiple hosts — single-host install only for v1.
- A web-UI for bootstrap-token rotation — CLI is sufficient for now.
- The dev-mode banner in the SPA showing "auth bypassed" — UX work, separate brief.
