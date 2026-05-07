# V1.4 — Approval timeout + default decision

**Owner**: claude. **Depends on**: H-auto-approval merged,
session_approval.rs stable.

## Why

Pending approval requests today block their worker indefinitely.
If an operator goes offline (lunch, weekend, dropped phone),
forge runs stall forever. We need a per-request timeout with a
configurable default decision (typically `Deny`, occasionally
`Approve` for low-risk work) so the worker can move on.

## Scope

Extend the approval request schema with two optional fields:
- `timeout_secs: Option<u64>` — when None, no timeout
  (preserves current behaviour for legacy rows)
- `on_timeout: Option<TimeoutDecision>` — `Approve | Deny | Escalate`,
  default `Deny`

A background scheduler in the worker_actions module sweeps
pending approvals on a 30s tick. Any pending row whose
`created_at + timeout_secs` is in the past gets the
`on_timeout` decision applied via the same `apply_decision`
path used by the CLI.

`Escalate` re-emits the notification via the channel-fallback
chain (separate brief; if that brief is unmerged, treat
`Escalate` as `Deny` with a TODO comment).

CLI surface:
- `argo approval request --timeout 600 --on-timeout deny`
  (existing request command gains flags)
- `argo doctor approvals` lists timeouts that fired in last
  24 h

## Approach

1. NEW `crates/argo-cli/src/handlers/worker_actions_auto_approval_timeout.rs`
   (~250 LOC):
   - `pub async fn timeout_sweeper(store: Arc<Store>) -> !`
     loop with `tokio::time::interval(Duration::from_secs(30))`
   - `async fn apply_pending_timeouts(store: &Store) -> ArgoResult<usize>`
     queries pending rows past their deadline; calls
     `apply_decision` for each.
2. Schema extension — add columns to existing approval table:
   - `timeout_secs INTEGER NULL`
   - `on_timeout TEXT NULL`
   Migration file in `crates/argo-store/migrations/` (one new
   file; do NOT modify previous migrations).
3. Spawn the sweeper from the existing forge-runtime
   bootstrap point (one new line). Sweeper exits gracefully
   on shutdown signal.
4. Tests: time-mocked unit tests that fast-forward `tokio::time`
   to verify a row past deadline gets `Deny`.

## Files you may touch

- `crates/argo-cli/src/handlers/worker_actions_auto_approval_timeout.rs` (NEW, ≤ 320 LOC)
- `crates/argo-cli/src/handlers/worker_actions_auto_approval_timeout_tests.rs` (NEW, ≤ 280 LOC)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: pub mod
- `crates/argo-store/migrations/<NNNN>_approval_timeout.sql` (NEW)
- `crates/argo-store/src/approvals.rs` (or wherever the
  approval CRUD lives) — narrowly: extend insert/update to
  carry the two new columns
- `crates/argo-types/src/handoff.rs` — narrowly: add the two
  optional fields + a `TimeoutDecision` enum
- `crates/argo-cli/src/cli.rs` — narrowly: add `--timeout`
  + `--on-timeout` flags to the existing `approval request`
  command (BLANKET CLI ALLOWLIST)

## Files you must NOT touch

- `session_approval.rs` decision logic — call existing
  `apply_decision`; do not modify it
- Notification channels (`crates/argo-channels/`) — sweeper
  does not send notifications, just decides
- Any other migration file

## Acceptance criteria

- [ ] Pending approval with `timeout_secs = 600` is decided
      `Deny` by the sweeper after 600s (verified via mocked clock)
- [ ] Approvals without timeout remain pending forever
      (legacy behaviour preserved)
- [ ] `on_timeout = "approve"` results in `Approved` outcome
- [ ] `on_timeout = "escalate"` re-emits notification (or
      falls back to Deny if fallback brief unmerged)
- [ ] Sweeper handles 1000 pending rows without burning CPU
      (single SQL `WHERE deadline_unix < NOW()` query)
- [ ] Migration is forward-only; existing rows get NULLs (no
      timeout)
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-approval-timeout-default-decision`
- PR title: `approval: timeout + default decision (claude-approval-timeout-default-decision)`

## Notes for the implementer

- Use `tokio::time::pause()` + `advance()` for deterministic
  sweeper tests; never sleep in tests.
- The sweeper MUST be idempotent — a missed tick (process
  restart) just means the next sweep catches up.
- Log every timeout-applied decision at INFO level with
  `approval_id, on_timeout, age_secs` fields so audit-export
  (separate brief) can pick it up.
