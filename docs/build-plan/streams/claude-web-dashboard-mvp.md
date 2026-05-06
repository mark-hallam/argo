# V1.4 — Web dashboard MVP

**Owner**: claude. **Depends on**: H-orchestrator-routes (#827),
H-skill-library-routes (#817), claude-realtime-audit-stream (#831)
merged.

## Scope

A minimal browser UI served from the existing argo-gateway.
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

Implementation: server-rendered HTML + minimal HTMX for interactivity.
NO React, NO build step. Static templates baked into
`argo-gateway` via `include_str!`. Auto-refresh every 5s via
SSE for the active-actions list.

## Files you may touch
- `crates/argo-gateway/src/dashboard.rs` (NEW — handlers)
- `crates/argo-gateway/src/dashboard_templates/` (NEW dir —
  workers.html, sessions.html, approvals.html, spend.html,
  layout.html, partial_action.html)
- `crates/argo-gateway/src/dashboard_assets/` (NEW dir —
  htmx.min.js + minimal CSS)
- `crates/argo-gateway/src/lib.rs` — narrowly: register dashboard
  routes
- `crates/argo-gateway/Cargo.toml` — narrowly: add `askama` or
  `tera` template-engine dep
- `crates/argo-gateway/tests/dashboard_smoke.rs` (NEW)

## Acceptance criteria
- [ ] All 4 tabs render with real data from existing endpoints
- [ ] SSE auto-refresh on /dashboard/actions/live
- [ ] Approval inline buttons hit existing /approve endpoint
- [ ] No external CDN dependencies (htmx.min.js bundled)
- [ ] Page loads under 200ms locally
- [ ] Works in Safari + Chrome + Firefox
- [ ] 8 unit tests (template rendering) + 2 integration tests
      (HTTP smoke against live gateway)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `claude-web-dashboard-mvp`
- PR title: `gateway: web dashboard MVP — server-rendered HTML + HTMX (claude-web-dashboard-mvp)`
