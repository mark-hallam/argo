# Stream P2-12-4 — kill switch: gateway HTTP routes

**Owner**: codex.
**Depends on**: mission-pm-kill-switch-1-store-audit MERGED, mission-pm-kill-switch-3-cli MERGED (for the `Capability::WorkerFreeze` declaration).
**Authored against**: origin/main `6bcb9aeb` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-pm-kill-switch-4-gateway`.
**Source**: skycot-pm-worker REPORT.md §6.2 + §11 P2 #12. Split slice 4/5 of parent `mission-pm-kill-switch` (BLOCKED 2026-05-19 — incomplete may-touch).

## Scope

HTTP surface for freeze/unfreeze, mounted under the gateway's dispatch_post router. Two routes:

1. `POST /api/v1/workers/{id}/freeze` — body `{ "reason": "<text>" }`. Requires `worker.freeze` capability.
2. `POST /api/v1/workers/{id}/unfreeze` — no body. Requires `worker.freeze` capability.

Both routes call into `argo_store::worker_freeze::{freeze, unfreeze}` after the capability check + audit emission (the existing route pattern already emits RbacDeny / RbacAllow audit rows on the capability check).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_worker_freeze.rs` (NEW) — the two route handlers. Pattern after an existing sibling `routes_worker_*.rs` (e.g. `routes_worker_settings.rs` or `routes_worker_trust_grade.rs`). Each route: parse path/body → capability check → call store fn → return JSON response.
- `crates/argo-cli/src/handlers/gateway/mod.rs` — `pub mod routes_worker_freeze;` registration. 1 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs` — mount the two new routes under the existing POST dispatcher. Mirror the pattern used by other `routes_worker_*` routes. ≤ 15 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_worker_freeze_tests.rs` (NEW) — 5 tests: (a) freeze with capability + valid body returns 200; (b) freeze without capability returns 403 + RbacDeny audit row; (c) unfreeze reverses state; (d) freeze with empty `reason` returns 400; (e) freeze for nonexistent worker returns 404.

## Files you must NOT touch

- `crates/argo-store/**` — store primitive is owned by sibling 1.
- `crates/argo-audit/**` — audit kind is owned by sibling 1.
- `crates/argo-agent-runner/**` — runner check is owned by sibling 2.
- `crates/argo-cli/src/cli_worker_cmds.rs` + `crates/argo-cli/src/handlers/worker_actions.rs` + `crates/argo-cli/src/handlers/worker_freeze.rs` — CLI surface is owned by sibling 3.
- `crates/argo-capability/**` + role defaults — owned by sibling 3.
- Web UI — owned by sibling 5.

## Acceptance criteria

- [ ] `POST /api/v1/workers/W1/freeze` with `{"reason": "test"}` + `worker.freeze` capability returns 200 and `is_frozen(W1)` is `Some(...)`.
- [ ] Without capability, the route returns 403 and emits an `RbacDeny` audit row.
- [ ] `POST /api/v1/workers/W1/unfreeze` reverses the freeze.
- [ ] Empty `reason` returns 400 with a clear validation message.
- [ ] Nonexistent worker returns 404.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-cli routes_worker_freeze` passes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `codex-mission-pm-kill-switch-4-gateway`
- PR title: `feat(gateway): POST /workers/{id}/freeze + unfreeze routes (mission-pm-kill-switch-4-gateway)`

## Notes for the implementer

- BEFORE adding the routes, read `routes_worker_settings.rs` (or another `routes_worker_*.rs` sibling) end-to-end and mirror its capability-check + response shape exactly. The gateway has strict conventions around audit emission on RBAC checks — don't reinvent them.
- The dispatch_post.rs mount point uses a path-prefix match. The new routes go in the same block as other `/workers/{id}/...` routes — find that block by `rg "workers/" crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs`.
- Response body on success: `{ "worker_id": "W1", "frozen_at": "2026-05-19T...", "frozen_by": "alice@acme", "reason": "..." }`. Mirror the FreezeRecord shape from sibling 1.
- Both routes are POST (not DELETE for unfreeze) to keep the request shape consistent with other state-toggle routes on workers.

## Out of scope

- Store primitive (sibling 1 — must be MERGED first).
- Runner pre-turn / pre-dispatch checks (sibling 2).
- CLI commands + capability declaration (sibling 3 — must be MERGED first for `Capability::WorkerFreeze`).
- Web UI (sibling 5).
- Preemptive SIGKILL — separate brief.
