# claude-error-rate-dashboard — Error rate per route + per provider over time

**Owner**: claude. **Depends on**: existing gateway request log + provider
call log in argo-store.

**DEEP-DRIFT — CONDUCTOR REWRITE NEEDED**: brief depends on a "gateway request log + provider call log in argo-store" that **does not exist**. Audited migrations v1..v60 — no per-HTTP-route table, no provider-call outcome table. `audit_record` carries typed `AuditKind` events (ToolResult / Halt / RbacDeny / etc.) but no `http.request`, `route.*`, `provider.call`, or HTTP-status kind; `routing_decision` records pre-call provider choice with no outcome column; `spend_ledger` records charged calls with no error column. The brief's "HTTP 5xx for routes" + "provider call where outcome != Ok" definitions cannot be answered from the store as it stands. **Recommended precursors**: queue `claude-gateway-request-log-table` (persist `(ts, route, status_code, transport_error)` rows — coordinate with `mission-gateway-request-audit-log` which covers this exact ground) + `claude-provider-call-outcome-log` (persist `(provider, model, outcome, ts)` rows). Once those land, this dashboard brief can be reattempted unchanged.

## Scope

Operators currently grep logs to spot a regression. This brief adds a CLI
dashboard that breaks error rate down by:
- HTTP route (gateway side)
- LLM provider (provider call side)

Output is a markdown table with one row per route/provider and columns
`requests`, `errors`, `error_rate_pct`, `p_change_vs_prev_window`.
Optional `--since <date>` clips the window.

## Approach (concrete)

1. **NEW** `crates/argo-cli/src/handlers/errors_dashboard.rs` (~320 LOC):
   - `pub fn run(since: Option<DateTime<Utc>>, group_by: GroupBy, format: OutputFormat) -> ArgoResult<()>`
   - `GroupBy` enum: `Route`, `Provider`, `Both`
   - Two query helpers:
     - `route_error_breakdown(store, since) -> Vec<RouteErrorRow>`
     - `provider_error_breakdown(store, since) -> Vec<ProviderErrorRow>`
   - Compare against the immediately-preceding equal-length window for
     `p_change_vs_prev_window` (positive % = getting worse)
2. **Modify** `crates/argo-cli/src/cli.rs`:
   - Add `Dashboard { since, group_by, format }` variant under `Errors`
3. **Modify** `crates/argo-cli/src/main.rs`: 1 dispatch arm.

## Files you may touch

- `crates/argo-cli/src/handlers/errors_dashboard.rs` (NEW, ≤ 380 LOC)
- `crates/argo-cli/src/handlers/errors_dashboard_tests.rs` (NEW, ≤ 280 LOC)
  - 8 unit tests:
    - empty store → empty table (not panic)
    - single route with 0 errors renders 0%
    - single route with 50% errors renders 50.0%
    - provider grouping
    - both grouping renders two sub-tables
    - `--since` clips correctly
    - prev-window comparison: improving → negative %
    - prev-window comparison: regressing → positive %
- `crates/argo-cli/src/cli.rs` — narrow: 1 subcommand
- `crates/argo-cli/src/main.rs` — narrow: 1 dispatch arm

## Files you must NOT touch

- argo-store schema — read existing tables only
- argo-types — keep row structs private to handler
- Other `Errors` subcommand handlers — leave them alone

## Acceptance criteria

- [ ] `argo errors dashboard` defaults to last 24h, groups by route
- [ ] `--group-by provider|route|both` works
- [ ] `--since 2026-05-01` clips correctly
- [ ] `--format json` emits structured rows
- [ ] Sorted by error_rate_pct desc by default
- [ ] Prev-window comparison column shows `+12.3%` / `-4.5%` / `n/a` (no prev)
- [ ] 8 unit tests
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-error-rate-dashboard`
- PR title: `errors: argo errors dashboard route+provider breakdown (claude-error-rate-dashboard)`

## Notes for the implementer

- Definition of "error": HTTP 5xx for routes; provider call where
  `outcome != Ok` for providers. Document this in the help text so
  operators can't be surprised.
- Prev window: `since - (now - since)` length-matched. If `since` is older
  than oldest data, prev window is "n/a".
- Keep route normalization stable: collapse `/v1/sessions/123/messages` →
  `/v1/sessions/{id}/messages` using the route registry, not regex on path.
