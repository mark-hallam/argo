# claude-dashboard-fleet-view-html — Web HTML view of all workers

**Owner**: claude. **Depends on**: `claude-web-dashboard-mvp` MERGED — extends the dashboard surface with a fleet view.

## Scope

The existing dashboard MVP shows a single session/worker. This brief adds a
fleet view that lists every worker (agent), its current status, and recent
activity. Served as plain HTML from the gateway. No JS framework — vanilla
HTML + a tiny `<script>` polling block. Read-only.

The page answers: "what is every worker doing right now, and what did they
just do?" Operators land here from RUNBOOK to triage swarm health.

## Approach (concrete)

1. **NEW** `crates/argo-cli/src/handlers/gateway/dashboard_fleet.rs` (~280 LOC):
   - `pub async fn handle_dashboard_fleet(state: Arc<GatewayState>) -> impl IntoResponse`
   - Query the worker registry / sessions store for every active worker
   - For each worker: id, kind, status (idle/running/blocked/error), last
     action, last activity timestamp, recent (last 5) action ids
   - Render to HTML via `format!` (mirrors the MVP dashboard pattern —
     no template engine)
   - Light JS polling (`setTimeout` reload every 10s)
2. **Modify** `crates/argo-cli/src/handlers/gateway/routes.rs`:
   - Mount `GET /dashboard/fleet` → `dashboard_fleet::handle_dashboard_fleet`
3. **Modify** `crates/argo-cli/src/handlers/gateway/mod.rs`: 1 pub mod line.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/dashboard_fleet.rs` (NEW, ≤ 320 LOC)
- `crates/argo-cli/src/handlers/gateway/dashboard_fleet_tests.rs` (NEW, ≤ 200 LOC)
  - `renders_empty_fleet`
  - `renders_workers_with_status`
  - `escapes_html_in_action_text`
  - `200_status_and_html_content_type`
  - `recent_activity_ordered_newest_first`
- `crates/argo-cli/src/handlers/gateway/routes.rs` — narrow: 1 route line
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrow: 1 pub mod line
- `crates/argo-cli/tests/dashboard_fleet_smoke.rs` (NEW) — 1 integration test
  hitting `/dashboard/fleet` against a gateway with a seeded worker

## Files you must NOT touch

- argo-store schema — no new tables; read existing worker/session rows
- argo-types — no new public types
- The existing single-session dashboard handler — leave it alone
- argo-tui or any CLI presenter — this is HTML-only

## Acceptance criteria

- [ ] `GET /dashboard/fleet` returns 200 with `text/html; charset=utf-8`
- [ ] Lists every worker with status badge + last activity timestamp
- [ ] Empty fleet renders a clean "no workers" message (no panic)
- [ ] All worker ids, action text, and free-form fields HTML-escaped
- [ ] Auto-refreshes every 10s via inline `setTimeout`
- [ ] No external CSS/JS — fully self-contained page
- [ ] 5 unit tests + 1 smoke test
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-dashboard-fleet-view-html`
- PR title: `dashboard: fleet view HTML at /dashboard/fleet (claude-dashboard-fleet-view-html)`

## Notes for the implementer

- Mirror the MVP dashboard handler's structure exactly — same imports, same
  HTML scaffold, same escape helper (lift it to a shared module if cleaner).
- Status badge colors: idle=grey, running=blue, blocked=amber, error=red.
  Inline `<style>` only.
- Keep recent-activity to 5 rows per worker — beyond that the page bloats.
- This is foundation for future per-route dashboards; keep the HTML scaffold
  reusable (one shared `render_layout(title, body)` helper).
