# claude-approval-request-store — approvals table + CRUD model

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep.**

**Authored against**: origin/main c2057ac1 on 2026-05-09 (conductor-authored prereq for claude-approval-timeout-default-decision).

## Why

The current approval flow is implicit: an action proposes itself, the auto-approval gate decides Allow/Ask/Deny inline, and the result is captured in the audit ledger. There's no first-class `approvals` table that tracks pending requests, who can grant/deny them, and when they timeout.

`claude-approval-timeout-default-decision` needs that table to apply a default decision after a configurable timeout. This brief lands the table + CRUD model — no timeout sweep logic yet.

## Approach (concrete)

1. **NEW migration** `crates/argo-store/migrations/42__approvals.sql` (or next free slot — verify first):
   - `approval_request(id BLOB PK, action_id BLOB, tenant_id BLOB, requested_at INTEGER, requested_by TEXT, deadline_at INTEGER, default_decision TEXT, status TEXT, decided_at INTEGER NULL, decided_by TEXT NULL, decision TEXT NULL)`
   - Indexes on `(tenant_id, status)` and `(deadline_at)` for the timeout sweep.
2. **NEW** `crates/argo-store/src/approvals.rs` (~280 LOC):
   - `pub struct ApprovalRequest { ... }`
   - `pub enum ApprovalStatus { Pending, Granted, Denied, Expired }`
   - `pub enum DefaultDecision { Allow, Ask, Deny }`
   - `pub fn create_request(...)`, `pub fn get_request(...)`, `pub fn decide_request(...)`, `pub fn list_pending(...)`, `pub fn expire_overdue(...)`.
3. **NEW** `crates/argo-store/src/approvals_tests.rs` (~150 LOC) — round-trip + state-machine tests.

## Files you may touch

- `crates/argo-store/migrations/42__approvals.sql` (NEW — verify next free raw slot)
- `crates/argo-store/src/migrations/v43.rs` (NEW — versioned; verify next free versioned slot)
- `crates/argo-store/src/migrations/mod.rs` — narrowly: 1 `mod v43` + insertion
- `crates/argo-store/src/approvals.rs` (NEW, ≤ 320 LOC)
- `crates/argo-store/src/approvals_tests.rs` (NEW, ≤ 200 LOC sibling)
- `crates/argo-store/src/lib.rs` — narrowly: pub mod

## Files you must NOT touch

- Auto-approval gate logic (`worker_actions_auto_approval.rs`) — that's the timeout brief's may-touch
- Audit ledger — separate write path

## Acceptance criteria

- [ ] Migration applies UP cleanly + DOWN rolls back cleanly
- [ ] `create_request` inserts a row with status=Pending
- [ ] `decide_request` transitions Pending → Granted/Denied; idempotent
- [ ] `expire_overdue` finds rows where `deadline_at < now` AND status=Pending; transitions to Expired with the configured `default_decision` recorded
- [ ] `list_pending` filters by tenant_id + status correctly
- [ ] State-machine tests pin every legal transition + reject illegal ones
- [ ] `cargo clippy -p argo-store --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-store` clean
- [ ] Every modified file ≤ 500 LOC

## Branch + PR

- Branch: `claude-approval-request-store`
- PR title: `store: approvals table + CRUD model (claude-approval-request-store)`

## Notes for the implementer

- Verify the next free raw migration slot before authoring (40/41 may be taken by skill-share / skill-canary if those land first).
- Verify the next free versioned `vN.rs` slot the same way.
- The `decide_request` idempotency contract is important. Use UPDATE-with-WHERE-status='Pending' so a second call is a no-op.

## Unblocks

- `claude-approval-timeout-default-decision` — needs this table + the `expire_overdue` primitive.
