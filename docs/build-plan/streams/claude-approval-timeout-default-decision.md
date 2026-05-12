# V1.4 — Approval timeout + default decision (sweeper wire-up)

**Owner**: claude. **Depends on**: `argo-store/migrations/48__approvals.sql` (MERGED — `approval_request` table with `deadline_at` + `default_decision` columns); `argo-store::approvals::expire_overdue` (MERGED — store primitive at `crates/argo-store/src/approvals.rs:311`).
**Authored against**: origin/main `e274161f` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-approval-timeout-default-decision`.

> **REWRITE NOTE 2026-05-12 (conductor)**: an earlier version of this brief was BLOCKED for premise drift — it assumed approval state lived in `worker_action` memory rows and asked for a brand-new `session_approval::apply_decision` path that didn't exist. The rewrite below targets the actual approval model that shipped via migration 48: the `approval_request` table already has `deadline_at` + `default_decision` columns, and `argo_store::approvals::expire_overdue` already promotes stale rows to `decision = default_decision` + `status = expired`. The brief now reduces to **wiring** that primitive to a periodic sweeper.

## Why

The store primitive `expire_overdue` exists but has no caller in the gateway boot loop. Pending approvals that hit their `deadline_at` stay in `pending` state forever; the timeout column is dead weight without a sweeper. Wire a 30s-tick sweeper that mirrors the existing `spawn_handoff_sweeper` shape and the bug closes.

## Scope

NEW sibling `crates/argo-cli/src/handlers/gateway/approval_expiry_sweeper.rs` modeled after `sweepers.rs::spawn_handoff_sweeper` (verify shape at `crates/argo-cli/src/handlers/gateway/sweepers.rs:31`). The new file exports:

```rust
pub fn spawn_approval_expiry_sweeper(store: Arc<SessionStore>) -> Option<JoinHandle<()>>;
```

Implementation: spawns a tokio task that, every 30 seconds (configurable via `ARGO_APPROVAL_EXPIRY_TICK_SECS`, default `30`), calls `argo_store::approvals::expire_overdue(pool, Utc::now().timestamp_micros(), "system:approval-expiry-sweeper")` and emits an audit row per non-zero rows-affected count (kind: `approval.expired_by_timeout`, payload: `{ count, swept_at }`).

Wire it into `crates/argo-cli/src/handlers/gateway/boot.rs` alongside the existing sweeper handles (search for `_sweeper` around line 115):

```rust
let _approval_expiry = store
    .as_ref()
    .map(|s| approval_expiry_sweeper::spawn_approval_expiry_sweeper(Arc::clone(s)));
```

CLI surface (additive):
- `argo doctor approvals --since 24h` — lists `approval_request` rows with `status = 'expired'` and `decided_by = 'system:approval-expiry-sweeper'` over the window. Uses the existing `argo doctor` handler pattern.

This is NOT in scope: the `Escalate` timeout-decision variant (`approval_request.default_decision` is a free-form TEXT today; only `approve` / `refuse` are wired through the action-resolution path — `escalate` would need a separate notify-channel re-emit brief). New columns on `approval_request`. Changing the `expire_overdue` store primitive. Touching the JSON `/api/v1/approvals` endpoints. Wiring an opt-out flag (operators who don't want timeouts already get the legacy "wait forever" behavior by setting `deadline_at` to a far-future micros value at request time).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/approval_expiry_sweeper.rs` (NEW, ≤ 180 LOC) — sweeper + tokio task
- `crates/argo-cli/src/handlers/gateway/approval_expiry_sweeper_tests.rs` (NEW, sibling, ≤ 200 LOC) — 5 unit tests: empty-pool no-op, single-overdue-row promoted to expired, two-rows-different-tenants both swept, deadline-in-future not touched, audit row emitted with correct kind+count
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly scoped: `pub mod approval_expiry_sweeper;`
- `crates/argo-cli/src/handlers/gateway/boot.rs` — narrowly scoped: 3-line wiring under the existing `_sweeper` block (~line 115)
- `crates/argo-cli/src/handlers/doctor_approvals.rs` (NEW, ≤ 200 LOC) — `argo doctor approvals` handler
- `crates/argo-cli/src/handlers/doctor_approvals_tests.rs` (NEW, sibling, ≤ 150 LOC) — 3 unit tests: no-expired-rows / multiple-expired-rows-rendered / window-filter
- `crates/argo-cli/src/cli_doctor_cmds.rs` or sibling — 1-line `Approvals { since: humantime::Duration }` variant
- `docs/skycot/RUNBOOK.md` — append "Approval-expiry sweeper" subsection documenting the env knob + audit-row shape

## Files you must NOT touch

- `crates/argo-store/src/approvals.rs` — the store primitive is correct; don't modify the signature
- `crates/argo-store/migrations/**` — no schema change
- `crates/argo-cli/src/handlers/gateway/sweepers.rs` — pattern reference only; keep the new sweeper in a sibling file (sweepers.rs is at 31+ LOC handling handoff-specific shape)
- The `worker_action` memory-row code path — orthogonal

## Acceptance criteria

- [ ] NEW `approval_expiry_sweeper.rs` compiles and is wired in `mod.rs`
- [ ] Sweeper task spawns from `boot.rs` under the same conditional as other sweepers (only when `store.is_some()`)
- [ ] Tick interval reads `ARGO_APPROVAL_EXPIRY_TICK_SECS` (default 30); minimum 5s clamp to prevent thrash; 0/negative ignored with `tracing::warn!`
- [ ] Calls `argo_store::approvals::expire_overdue(pool, now_micros, "system:approval-expiry-sweeper")` per tick
- [ ] On rows-affected > 0, emits audit row via `AuditSink::write(None, AuditKind::HttpRequest, payload, None)` with payload `{ kind: "approval.expired_by_timeout", count, swept_at_iso }`. (Use existing AuditKind variant; adding a new kind is out of scope.)
- [ ] `argo doctor approvals --since 24h` lists expired rows with id / action_id / decided_at / decision
- [ ] 5 sweeper unit tests + 3 doctor-handler unit tests
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] All touched files < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-approval-timeout-default-decision`
- PR title: `gateway: approval-expiry sweeper + doctor surface (claude-approval-timeout-default-decision)`

## Notes for the implementer

- Read `crates/argo-cli/src/handlers/gateway/sweepers.rs` BEFORE writing — match `spawn_handoff_sweeper`'s structure exactly. The `ARGO_HANDOFF_SWEEP_TICK_SECS` env-var convention is the pattern to mirror for `ARGO_APPROVAL_EXPIRY_TICK_SECS`.
- `expire_overdue` already runs in a single SQL UPDATE — there's no row-by-row iteration. The audit row is the LOG of the sweep, not per-row.
- The tick interval can be ≥30s in production but tests should use `Duration::from_millis(50)` to drive multiple ticks deterministically.
- Don't add a new `AuditKind` variant — use `AuditKind::HttpRequest` (or whatever the existing "system-internal sweep" pattern uses; verify via `rg 'AuditKind::' crates/argo-cli/src/handlers/gateway/sweepers.rs`) with a `kind: "approval.expired_by_timeout"` field inside `payload_json`. Adding new AuditKind variants requires workspace-wide consideration.
- Per-tenant accounting could be a follow-up brief (`mission-approval-expiry-per-tenant-metrics`) — out of scope here. v1 emits one row per non-zero sweep across all tenants.
- The `Escalate` variant is intentionally out of scope. v1 maps `default_decision` directly to `decision` (the existing `expire_overdue` logic). Operators wanting escalation behavior set their `default_decision` to `refuse` and rely on the notify-on-refuse path that ships independently.
