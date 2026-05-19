# Stream P2-12 — kill switch / hard stop per worker

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `80ffa124` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-pm-kill-switch`.
**Source**: skycot-pm-worker REPORT.md §6.2 + §11 P2 #12.

## Scope

Today an operator cannot hard-stop a runaway worker from the UI. They can rate-limit (MCP rate limiter), they can archive (post-hoc), but they cannot say "freeze worker W1 mid-run, no more turns, no more tool dispatch, until I unfreeze." This brief lands that primitive.

Mechanic:

1. New `worker_freeze` row in DB: `(worker_id, frozen_at, frozen_by, reason, unfrozen_at)`.
2. Agent-runner checks `is_worker_frozen(worker_id)` before each turn and before each tool dispatch. If frozen, the turn aborts with `WorkerFrozen` and the runner emits a `WorkerFreezeBlocked` audit row.
3. CLI: `argo worker freeze <id> --reason "<text>"` / `argo worker unfreeze <id>`.
4. UI: a red "Freeze" button on the worker profile page; confirms with a prompt.

This is a cooperative primitive — it does NOT kill a running tool subprocess (e.g. an in-flight HTTP request). It stops the **next** dispatch.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/migrations/<next>__worker_freeze.sql` (NEW) — `worker_freeze` table.
- `crates/argo-store/src/migrations/v<next>.rs` (NEW).
- `crates/argo-store/src/worker_freeze.rs` (NEW) — `freeze(worker_id, frozen_by, reason)`, `unfreeze(worker_id, unfrozen_by)`, `is_frozen(worker_id) -> Option<FreezeRecord>`, `list_frozen() -> Vec<FreezeRecord>`.
- `crates/argo-agent-runner/src/turn.rs` — narrowly scoped: before the turn loop starts, check `is_frozen(worker_id)`; return `ArgoError::Provider("worker frozen: {reason}")`. ≤ 15 LOC of additions.
- `crates/argo-agent-runner/src/dispatch.rs` — narrowly scoped: before each tool dispatch, same check. Emit `WorkerFreezeBlocked` audit row on block.
- `crates/argo-audit/src/event.rs` — add `WorkerFreezeBlocked` audit kind.
- `crates/argo-cli/src/cli_worker_cmds.rs` (or existing worker CLI module) — add `Freeze` + `Unfreeze` variants.
- `crates/argo-cli/src/handlers/worker_freeze.rs` (NEW) — handlers for the 2 new CLI subcommands.
- `web/app/(app)/workers/[workerId]/page.tsx` — narrowly scoped: add a "Freeze worker" button + confirm modal. Posts to a NEW route.
- `web/app/api/workers/[id]/freeze/route.ts` (NEW) — server route that calls the gateway.
- `crates/argo-cli/src/handlers/gateway/routes_worker_freeze.rs` (NEW) — `POST /api/v1/workers/{id}/freeze` + `POST /api/v1/workers/{id}/unfreeze`.

## Files you must NOT touch

- Any MCP / tool subprocess code — this is cooperative, not preemptive.
- `crates/argo-tool/src/registry.rs` dispatch — the check lives in the agent-runner's pre-dispatch hook, not in registry.
- Any other audit-emitting site.

## Acceptance criteria

- [ ] `worker_freeze` table + CRUD work; `is_frozen` returns `Some(record)` when frozen, `None` after `unfreeze`.
- [ ] Agent-runner refuses to start a new turn for a frozen worker; returns clear error message including the freeze reason.
- [ ] Pre-dispatch check refuses tool calls and emits `WorkerFreezeBlocked` audit row. Existing in-flight tool calls are NOT cancelled (cooperative — document this in the user-facing message).
- [ ] CLI: `argo worker freeze W1 --reason "investigating prompt injection"` succeeds; `argo worker unfreeze W1` reverses it.
- [ ] Capability check: `worker.freeze` capability required on both routes + CLI. Default-role assignment: owner + admin only.
- [ ] UI button on `/workers/<id>` triggers a confirm modal; on confirm, POST + reload shows the frozen state. Unfreeze button appears when frozen.
- [ ] Tests: (a) frozen worker cannot start a turn; (b) frozen worker cannot dispatch a tool mid-turn (drives this via the dispatch_one_with_tenant path); (c) unfreeze allows turn to resume; (d) capability deny path; (e) audit row count matches blocked dispatches.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `pnpm lint && pnpm test && pnpm build` clean in `web/`.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `codex-mission-pm-kill-switch`
- PR title: `worker: freeze/unfreeze primitive + CLI + UI button (mission-pm-kill-switch)`

## Notes for the implementer

- The check is `is_frozen(worker_id)` synchronously before the turn loop and before each dispatch. Cache the result for the duration of one turn — don't query the DB on every tool call within a single turn (a freeze applied mid-turn will land at the next turn boundary, which is acceptable v1 behaviour).
- The "Freeze" button is **destructive UX**. Use a red button + explicit confirm modal that requires typing the worker name to confirm. Don't make this a one-click action.
- `frozen_by` is a `Principal` serialised the same way `mission-pm-capability-audit-events` (P0 #4) serialises actor strings. Match the convention.
- Document the cooperative semantics in the error message: "Worker W1 is frozen by alice@acme: investigating prompt injection. New turns and tool calls are blocked. In-flight tool calls continue to completion."
- Coordinate `AuditKind` additions with parallel briefs that also add kinds (P0 #4, P1 #6, P1 #8).

## Out of scope

- Preemptive kill (SIGKILL on an in-flight subprocess) — separate brief.
- Time-bounded auto-unfreeze ("freeze for 1 hour") — separate brief.
- Tenant-wide kill switch (freeze all workers at tenant T) — separate brief.
- Audit-trail summary on the profile page ("frozen 3 times in last 30 days") — separate brief.
