# Mission load-bearing wire — escalation on operator no-reply for approval requests

**Owner**: claude. **Depends on**: `claude-approval-request-store` MERGED (#1091); `argo-channels::NotifyChannel` trait stable; existing `expire_overdue` primitive in `argo-store::approvals`.
**Authored against**: origin/main `e17857b0` on `2026-05-10`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-escalation-on-no-reply`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 6 item #130 ("Add escalation-on-no-reply"). The approval-request store now records `deadline_at` per request (#1091 MERGED), and `expire_overdue` records the row's `default_decision` when the deadline lapses. But there's no escalation step BEFORE expiration — operators get one ping at request time, then silence, then auto-expire to default. Mission Claim 6 ("multi-channel approvals") promises a real reminder/escalation flow.

## Scope

Add a periodic background task that scans `approval_request` rows where:
- `status = Pending`
- `deadline_at` is in the future BUT `(deadline_at - now) <= reminder_window` (default 30 minutes)
- No `escalation_sent_at` recorded yet (NEW column tracking whether reminder fired)

For each row found: fan out a `Notification` of kind `ApprovalReminder` (NEW variant) to the originally-requested channels, then stamp `escalation_sent_at = now`. The actual `expire_overdue` flow (separate brief #118 if not yet shipped) consumes the deadline regardless.

Wire from gateway boot like other periodic tasks (`_alerts_loop`, `_sweeper`). Configurable via `ARGO_APPROVAL_REMINDER_WINDOW_MINUTES` (default 30) and `ARGO_APPROVAL_REMINDER_INTERVAL_SECS` (default 300 = scan every 5 min).

Schema change: ADD column `approval_request.escalation_sent_at INTEGER NULL` via NEW migration v50 (next free slot — v49 is `mvp-worker-skills-table` which is in flight; verify v49 hasn't merged; if v49 still TODO/CLAIMED, this brief uses v51 to avoid collision).

This is NOT in scope: multi-step escalation (only one reminder for v1; #118 multi-step-approval covers chain); changing approval channel routing; per-tenant reminder window override; recipient-side opt-out.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/migrations/50__approval_escalation.sql` (NEW; OR `51__...` if v50 contested) — `ALTER TABLE approval_request ADD COLUMN escalation_sent_at INTEGER NULL;`. Idempotent via `column_exists` PRAGMA probe (mirrors existing pattern in v22-v34).
- `crates/argo-store/src/migrations/v50.rs` (NEW; OR v51) — versioned migration matching the raw SQL.
- `crates/argo-store/src/migrations/mod.rs` — narrowly scoped: ONE `mod v50;` line + ONE ladder entry. NO other edits.
- `crates/argo-store/src/lib_tests.rs` — narrowly scoped: schema-tip assert bump.
- `crates/argo-store/src/migration_reconciliation_tests.rs` — narrowly scoped: schema-tip assert bumps (4-5 occurrences mirror existing pattern).
- `crates/argo-store/src/approvals.rs` — narrowly scoped: ADD `pub async fn list_pending_near_deadline(tenant: &TenantId, window: Duration, now: DateTime<Utc>) -> ArgoResult<Vec<ApprovalRequest>>`. ADD `pub async fn mark_escalation_sent(request_id: &Uuid) -> ArgoResult<()>`. NO change to existing fns.
- `crates/argo-cli/src/handlers/gateway/approval_reminder.rs` (NEW, ≤ 200 LOC) — periodic spawn, scan-and-notify loop, `Notification::ApprovalReminder` builder.
- `crates/argo-cli/src/handlers/gateway/boot.rs` — narrowly scoped: ONE block adding `let _approval_reminder = spawn_approval_reminder(...);` next to existing `_sweeper` / `_alerts_loop` handles.
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly scoped: ONE `pub(super) mod approval_reminder;` line.
- `crates/argo-cli/src/handlers/gateway/approval_reminder_tests.rs` (NEW, sibling) — 4 unit tests: scan-finds-near-deadline; scan-skips-already-escalated; scan-skips-future-beyond-window; mark-escalation-is-idempotent.
- `crates/argo-cli/tests/approval_reminder_smoke.rs` (NEW) — integration smoke: seed 3 approvals (one near-deadline + un-escalated, one near-deadline + already-escalated, one far-future); run reminder loop; assert exactly 1 notification fired.

## Files you must NOT touch

- `crates/argo-channels/**` — NotifyChannel trait stable; reuse only
- `crates/argo-store/migrations/48__approvals.sql` — original approvals migration is upstream
- Any tool, any non-store/non-cli crate

## Acceptance criteria

- [ ] Migration adds `escalation_sent_at INTEGER NULL` column idempotently (re-run is no-op)
- [ ] `list_pending_near_deadline(window)` returns rows where status=Pending AND escalation_sent_at IS NULL AND (deadline_at - now) <= window
- [ ] `mark_escalation_sent(request_id)` is idempotent (second call is no-op)
- [ ] Reminder task runs at `ARGO_APPROVAL_REMINDER_INTERVAL_SECS` (default 300s)
- [ ] When reminder fires, `Notification::ApprovalReminder` is built + dispatched to the originally-requested channels (reuse the channel list stored on the approval row)
- [ ] After successful dispatch, `mark_escalation_sent` stamps the row (so subsequent scans skip it)
- [ ] Failed dispatch does NOT mark escalation_sent (operator gets a retry on next scan)
- [ ] env-vars `ARGO_APPROVAL_REMINDER_WINDOW_MINUTES` (30 default) + `ARGO_APPROVAL_REMINDER_INTERVAL_SECS` (300 default) override
- [ ] 4 unit tests + 1 integration smoke
- [ ] No new public API on `argo-channels` or `argo-store` outside the listed exports
- [ ] `cargo build -p argo-store -p argo-cli` clean
- [ ] `cargo clippy -p argo-store -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-store -p argo-cli` passes (skip pre-existing flakes)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-escalation-on-no-reply`
- PR title: `approvals: escalate on no-reply via NEW approval_reminder background task (mission-escalation-on-no-reply)`

## Notes for the implementer

- **Migration slot collision check first**: `grep -E "v49|v50|v51" docs/build-plan/BUILD_STATUS.md | head` — if v49/v50 are currently CLAIMED/IN_REVIEW, bump to next free slot. Cite the slot decision in PR body.
- The `Notification::ApprovalReminder` variant: extend the existing `Notification` enum in `argo-channels`; if it's an exhaustive-match-protected enum, add per HARD RULE 5 exhaustive-match allowance with a one-line variant + handler in each match site (mirror the pattern from `Notification::ApprovalRequest`).
- If the original approval row doesn't store the channel list (only the `recipient` and channel-router fan-out logic decides), the reminder uses the same fan-out logic as the original `ApprovalRequest` — call into the same dispatcher, just with a different `Notification` variant.
- Don't combine reminder + expiration. The brief explicitly says reminder is BEFORE expiration; `expire_overdue` is a separate concern that fires when deadline passes (ETA: brief #118 multi-step-approval).
- The 30-min window is the brief's default. Operators may want shorter (e.g. 15 min for time-critical actions) — the env-var knob covers this.
