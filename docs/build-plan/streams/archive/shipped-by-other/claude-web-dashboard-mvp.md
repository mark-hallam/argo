# V1.4 — Web dashboard MVP

**Owner**: claude. **Depends on**: H-orchestrator-routes (#827),
H-skill-library-routes (#817), claude-realtime-audit-stream (#831)
merged.

> **REWRITE NOTE (tick 269 by conductor)**: original brief pointed
> to `crates/argo-gateway/src/dashboard.rs`, but the
> `argo-gateway` crate hosts chat-platform inbound adapters and
> contains NO HTTP server. The actual HTTP gateway server lives
> at `crates/argo-cli/src/handlers/gateway/`. This rewrite moves
> all paths under `crates/argo-cli/src/handlers/gateway/` and
> preserves the operator-UX intent (server-rendered HTML + HTMX,
> no React, no build step).

## Scope

A minimal browser UI served from the existing argo HTTP gateway
(implemented in `crates/argo-cli/src/handlers/gateway/`).
Operator opens `http://localhost:7090/dashboard/` after
`argo gateway start` and sees:

- **Workers tab** — table of all workers with status (active /
  paused / circuit-open), last action, today's spend, channel
  preferences. Click row → worker portrait.
- **Sessions tab** — recent sessions across workers, clickable
  to drill into the audit chain.
- **Approvals tab** — pending approvals, inline approve/deny
  buttons (uses existing webinbox approval-token flow).
- **Spend tab** — chart: daily spend per worker, last 30 days.

Implementation: server-rendered HTML + minimal HTMX for
interactivity. NO React, NO build step. Static templates baked
into the gateway binary via `include_str!`. Auto-refresh every
5s via SSE for the active-actions list.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/dashboard.rs` (NEW —
  axum handlers for the 4 tabs + SSE endpoint; ≤ 480 LOC)
- `crates/argo-cli/src/handlers/gateway/routes_dashboard.rs`
  (NEW — route registration helper used by `routes.rs`; ≤ 200 LOC)
- `crates/argo-cli/src/handlers/gateway/dashboard_templates/`
  (NEW dir — `workers.html`, `sessions.html`, `approvals.html`,
  `spend.html`, `layout.html`, `partial_action.html`)
- `crates/argo-cli/src/handlers/gateway/dashboard_assets/`
  (NEW dir — `htmx.min.js` + minimal CSS, both bundled via
  `include_str!`/`include_bytes!`; NO external CDN)
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly: 2
  pub mod lines for the new modules
- `crates/argo-cli/src/handlers/gateway/routes.rs` — narrowly:
  call `routes_dashboard::register(...)` to mount `/dashboard/*`
- `crates/argo-cli/Cargo.toml` — narrowly: add `askama` (or
  `tera`) template-engine dep
- `crates/argo-cli/tests/dashboard_smoke.rs` (NEW — HTTP smoke
  against a live gateway; ≤ 250 LOC)
- BLANKET CLI ALLOWLIST applies for any incidental CLI plumbing

## Files you must NOT touch

- `crates/argo-gateway/` — that crate handles chat-platform
  inbound adapters (Slack/WhatsApp/etc.); the dashboard belongs
  on the HTTP gateway, NOT there
- Any existing `routes_*.rs` file other than the narrow
  `routes.rs` mount-point edit
- argo-store schema — dashboard reads via existing endpoints only

## Acceptance criteria

- [ ] All 4 tabs render at `http://localhost:7090/dashboard/`
      with real data from existing endpoints
- [ ] SSE auto-refresh on `/dashboard/actions/live`
- [ ] Approval inline buttons hit existing `/approve` endpoint
- [ ] No external CDN dependencies (htmx.min.js bundled via
      `include_bytes!`)
- [ ] Page loads under 200ms locally
- [ ] Works in Safari + Chrome + Firefox
- [ ] 8 unit tests (template rendering) + 2 integration tests
      (HTTP smoke against live gateway via `dashboard_smoke.rs`)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-web-dashboard-mvp`
- PR title: `gateway: web dashboard MVP — server-rendered HTML + HTMX (claude-web-dashboard-mvp)`

## Notes for the implementer

- The HTTP gateway's mount-point in `routes.rs` is the single
  source of route registration. Keep the dashboard module
  self-contained behind a `routes_dashboard::register(router)`
  helper so `routes.rs` only gets a 1-line addition (preserves
  the routes.rs LOC budget).
- HTMX bundle: pin a specific minified version (e.g. v1.9.10);
  do NOT pull at build time.
- Follow the existing `routes_audit_stream.rs` SSE pattern for
  the live-actions feed.
