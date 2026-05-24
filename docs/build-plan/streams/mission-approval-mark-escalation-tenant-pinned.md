# mission-approval-mark-escalation-tenant-pinned

**Owner**: codex.
**Depends on**: nothing (parallel-safe with sibling fix-briefs).
**Authored against**: origin/main `4312a5ca6` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-approval-mark-escalation-tenant-pinned`.

🟡 **P1 LATENT-LEAK.** `approvals::mark_escalation_sent_at` keys
`UPDATE approval_request SET escalation_sent_at = ? WHERE id = ? AND
status = 'pending' AND escalation_sent_at IS NULL` on `id` alone —
same shape as the `decide_request` bug shipped via #2146, **not**
fixed by that PR.

## Why

PR #2116 §4 finding 1 named both `decide_request` AND
`mark_escalation_sent` as the same SQL-shape leak. PR #2146 only
flipped `decide_request`. The escalation helper still has the open
gap. Found by re-verification pass in
`eval-argo-store-tenant-bind-sweep-remaining-tables` §3.2 row 1b.

## Scope

- Add `tenant_id: TenantId` parameter to
  `approvals::mark_escalation_sent` + `mark_escalation_sent_at`.
- WHERE clause now binds
  `WHERE id = ? AND tenant_id = ? AND status = 'pending' AND
  escalation_sent_at IS NULL`.
- Thread tenant through the sole production caller:
  `crates/argo-cli/src/handlers/gateway/approval_reminder.rs`.
- Add a `cross_tenant_mark_escalation_rejects` pin test in
  `crates/argo-store/tests/`.
- ~80-150 LOC.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/src/approvals.rs`
- `crates/argo-store/tests/approval_mark_escalation_pin.rs` (new)
- `crates/argo-cli/src/handlers/gateway/approval_reminder.rs`
- `crates/argo-cli/tests/approval_reminder_smoke.rs`
- `docs/build-plan/BUILD_STATUS.md`

## Acceptance criteria

- [ ] `mark_escalation_sent_at` rejects cross-tenant calls at SQL.
- [ ] Pin test passes.
- [ ] `cargo clippy -p argo-store -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
