# Stream Audit-P0-1 — gate gateway dashboards behind bearer auth + loopback-bind option

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `d2e4a579` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-audit-gateway-dashboard-auth-gate`.
**Source**: 2026-05-19 codex read-only architecture audit P0-2, validated by claude `VALIDATION.md` (CONFIRMED on `ff02a823`).

## AMENDED 2026-05-19 by conductor (post-BLOCK)

Original brief was BLOCKED 2026-05-19 by codex pre-implementation: loopback-only acceptance requires TCP peer IP threading through `boot_router.rs` (outside may-touch). Conductor narrowed scope:

- **Dropped**: `dashboard_bind_loopback_only` config field + dashboard.rs loopback-only enforcement. Defer to v1.x sibling brief `mission-audit-dashboard-loopback-only` if/when needed.
- **Kept**: the primary security fix — move dashboard mounts AFTER `check_bearer_auth` so dashboards inherit the bearer-auth gate.

The kept core change is the high-value security closure. The loopback-only fallback was nice-to-have for dev environments but required surface beyond may-touch.

## Scope

The legacy server-rendered gateway dashboards are mounted **before** the bearer-auth gate. `crates/argo-cli/src/handlers/gateway/routes.rs:102-108` dispatches the dashboard + fleet-dashboard handlers, and `routes.rs:124-127` only runs `check_bearer_auth` afterwards. When `ARGO_GATEWAY_BEARER_TOKEN` is unset (dev default per `routes_auth.rs:31-36`), the dashboards are reachable by anyone with network access to the gateway port. When set, the dashboards have already been matched and served before bearer-auth runs.

The dashboards expose: per-worker budget + spend ledger, `audit_record` activity, pending approvals, sessions, live actions, and worker IDs. Any unauthenticated caller on a network-reachable gateway can read all of this.

This brief reorders the routing so dashboard mount points run **after** bearer-auth, adds an opt-in config flag `[gateway] dashboard_bind_loopback_only = true` that refuses non-loopback dashboard requests when set, and adds tests proving `/dashboard/*` returns `401` without a valid token.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes.rs` — move the dashboard dispatch block (currently lines 102-108) to AFTER the bearer-auth gate (currently line 124-127). Delete the comment at lines 93-101 that justified the pre-auth mount.
- `crates/argo-cli/tests/gateway_dashboard_auth_smoke.rs` (NEW, follows existing `cross_tenant_skill_discovery_negative.rs` shape) — three test cases: (1) `ARGO_GATEWAY_BEARER_TOKEN` set + no header → `401`; (2) bearer token set + correct header → `200`.

## Files you must NOT touch

- The dashboard rendering code (`dashboard/render.rs`, `dashboard_fleet.rs`) — those are read-only views; data exposure is fixed by gating, not by editing renders.
- The bearer-auth code itself (`routes_auth.rs`) — a separate brief (`mission-audit-bearer-fail-closed-prod`) addresses the fail-open default.
- Any `web/` frontend code — the SPA authenticates separately.
- Other route mount points in `routes.rs` (only the dashboard block moves).

## Acceptance criteria

- [ ] `routes.rs` dispatch order is: maintenance gate → bearer-auth gate → public-GET allowlist → all other routes (dashboard included). The dashboard block no longer sits between maintenance and auth.
- [ ] When `ARGO_GATEWAY_BEARER_TOKEN` is unset, dashboards remain reachable (back-compat for dev environments).
- [ ] When `ARGO_GATEWAY_BEARER_TOKEN` is set, dashboards return `401` without a valid `Authorization: Bearer <token>` header.
- [ ] When `dashboard_bind_loopback_only=true`, non-loopback dashboard requests return `403`. Loopback requests pass through to the existing auth check.
- [ ] The three new test cases in `gateway_dashboard_auth_smoke.rs` pass on Linux and macOS.
- [ ] No regression in existing routes tests (`cargo test -p argo-cli` clean).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean — claim in PR body.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `codex-mission-audit-gateway-dashboard-auth-gate`
- PR title: `gateway: dashboards behind bearer auth + loopback-bind option (mission-audit-gateway-dashboard-auth-gate)`

## Notes for the implementer

- The dashboards were mounted before bearer auth almost certainly for operator first-run UX. The loopback-bind flag is the replacement: an operator on `localhost` keeps their dashboard without bearer auth, but a public-bound gateway is locked down.
- Use whatever peer-IP extraction the existing maintenance gate uses — do not reinvent.
- The `dashboard_bind_loopback_only` flag must be documented in `docs/skycot/operator/configuration.md` if that doc exists; otherwise add a `// CONFIG-REFERENCE:` comment in the new config section so a docs sweep picks it up.
- The 401 body should match what `check_bearer_auth` returns elsewhere — be consistent, do not introduce a new error shape.

## Out of scope (follow-up briefs)

- Removing the legacy dashboards entirely (the SPA `/team` page is the v1.x replacement).
- Per-role visibility within the dashboard (only owners see spend, etc.) — that's a `CapabilityEngine` gate, separate brief.
- Migrating dashboard routes to the new bearer-auth + session model coming from PR #1835 — defer until #1835 lands.
