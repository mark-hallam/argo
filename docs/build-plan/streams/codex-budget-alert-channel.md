# V1.4 — Budget alert push notifications

**Owner**: codex. **Depends on**: H-spend-persistence (#809),
H-budget-cli (#820), D-push (#798) all merged.

## Scope

Today operators learn about a worker's budget overrun only by
running `argo budget show` reactively. Push notification on
threshold-cross is missing — high-stakes for autonomous workers
that quietly drift toward their cap overnight.

Add a budget watcher that:
- Polls `spend_today_for_worker` + `spend_this_month` every N
  seconds (default 60; override via `$ARGO_BUDGET_WATCH_SECS`).
- On threshold cross (80% warning, 100% breach), fires a
  PushApprovalChannel notification with the worker_id, threshold
  pct, current spend, and remaining headroom.
- De-duplicates: each (worker_id, threshold) pair fires at most
  once per day (resets at midnight local).
- Records each fire in a new `worker_budget_alert` table for
  auditability.

CLI surface:
- `argo budget alerts list [--worker <id>] [--since <duration>]
  [--json]` — read recent alerts.
- `argo budget alerts test --worker <id> --threshold 80|100` —
  trigger a synthetic alert (useful for verifying push setup).

Watcher is launched via `argo budget watch` (long-running) or
hooked into the existing daemon if one is running. Brief assumes
short-lived `argo budget watch` for v1; daemon hookup later.

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-store/migrations/26__worker_budget_alert.sql`
  (NEW) — `worker_budget_alert` table (worker_id, threshold,
  fired_at, spend_amount, headroom_amount, notified_via)
- `crates/argo-store/src/budget_alert.rs` (NEW) — insert /
  list / has_fired_today helpers
- `crates/argo-store/src/lib.rs` — narrowly: re-export
- `crates/argo-cli/src/handlers/budget_watch.rs` (NEW)
- `crates/argo-cli/src/handlers/budget_alerts_list.rs` (NEW)
- `crates/argo-cli/src/handlers/budget_alerts_test.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 3 pub mods
- `crates/argo-cli/src/cli_budget_cmds.rs` — narrowly: add
  `Watch`, `Alerts(BudgetAlertsCmd)` variants
- `crates/argo-cli/tests/budget_alerts_smoke.rs` (NEW)

## Acceptance criteria
- [ ] Threshold 80% fires at first cross; doesn't re-fire same
      day after re-cross
- [ ] Threshold 100% fires independently from 80%
- [ ] Both thresholds reset at midnight local (verified via
      table query day-over-day)
- [ ] PushApprovalChannel.send() invoked with correct
      worker_id + threshold metadata
- [ ] `argo budget alerts list --json` returns NDJSON of fires
- [ ] `argo budget alerts test --worker x --threshold 100`
      bypasses dedup and immediately fires (records as `synthetic=true`)
- [ ] 5 unit tests (dedup, threshold cross, midnight reset,
      synthetic, list) + 1 CLI smoke
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-budget-alert-channel`
- PR title: `budget: push alerts on 80%/100% threshold cross (codex-budget-alert-channel)`
