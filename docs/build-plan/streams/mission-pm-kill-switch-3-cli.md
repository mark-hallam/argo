# Stream P2-12-3 — kill switch: CLI freeze/unfreeze commands

**Owner**: codex.
**Depends on**: mission-pm-kill-switch-1-store-audit MERGED.
**Authored against**: origin/main `6bcb9aeb` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-pm-kill-switch-3-cli`.
**Source**: skycot-pm-worker REPORT.md §6.2 + §11 P2 #12. Split slice 3/5 of parent `mission-pm-kill-switch` (BLOCKED 2026-05-19 — incomplete may-touch).

## Scope

CLI surface for freeze/unfreeze, plus the dispatch hook in the worker_actions handler that routes `WorkerCmd::Freeze` / `WorkerCmd::Unfreeze` variants to the right business logic.

Two commands:

1. `argo worker freeze <id> --reason "<text>"` — freezes the worker. Requires `worker.freeze` capability.
2. `argo worker unfreeze <id>` — reverses the freeze. Requires `worker.freeze` capability.

Capability `worker.freeze` is declared in this brief and assigned to the owner + admin default roles. Other roles can be granted via the existing role-binding flow.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-cli/src/cli_worker_cmds.rs` — add `Freeze { id: String, #[arg(long)] reason: String }` and `Unfreeze { id: String }` variants to the `WorkerCmd` enum. ≤ 15 LOC. (If file is near 500 LOC, split first via a small refactor sibling — but as of 2026-05-19 it has headroom.)
- `crates/argo-cli/src/handlers/worker_actions.rs` — add the dispatch arms for `WorkerCmd::Freeze` and `WorkerCmd::Unfreeze` that call into the new handler module. ≤ 20 LOC.
- `crates/argo-cli/src/handlers/mod.rs` — `pub mod worker_freeze;` registration. 1 LOC.
- `crates/argo-cli/src/handlers/worker_freeze.rs` (NEW) — `pub async fn freeze(deps, id, reason) -> Result<()>`, `pub async fn unfreeze(deps, id) -> Result<()>`. Each handler does the capability check (`Capability::WorkerFreeze`), then calls `argo_store::worker_freeze::{freeze, unfreeze}`. JSON output for `--json` flag if existing pattern requires it.
- `crates/argo-capability/src/lib.rs` (or wherever `Capability` enum lives — verify) — add `WorkerFreeze` variant. ≤ 5 LOC.
- `crates/argo-identity/src/roles/owner.rs` + `roles/admin.rs` (or wherever default-role capability lists live — verify path before editing) — append `Capability::WorkerFreeze` to the owner + admin default role definitions.
- `crates/argo-cli/src/handlers/worker_freeze_tests.rs` (NEW) — 3 tests: (a) freeze with valid capability succeeds; (b) freeze without capability returns RBAC deny; (c) double-freeze updates reason (matches store contract).

## Files you must NOT touch

- `crates/argo-store/**` — store primitive is owned by sibling 1.
- `crates/argo-audit/**` — `WorkerFreezeBlocked` variant is owned by sibling 1.
- `crates/argo-agent-runner/**` — runner check is owned by sibling 2.
- Gateway routes — owned by sibling 4.
- Web UI — owned by sibling 5.

## Acceptance criteria

- [ ] `argo worker freeze W1 --reason "investigating prompt injection"` returns success and a `worker_freeze` row exists for W1.
- [ ] `argo worker unfreeze W1` returns success and `is_frozen(W1)` returns `None`.
- [ ] Without `worker.freeze` capability, both commands return an RBAC deny error.
- [ ] `Capability::WorkerFreeze` is in the owner + admin default role lists.
- [ ] `cargo clippy -p argo-cli -p argo-capability -p argo-identity --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-cli worker_freeze` passes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `codex-mission-pm-kill-switch-3-cli`
- PR title: `feat(cli): argo worker freeze/unfreeze + worker.freeze capability (mission-pm-kill-switch-3-cli)`

## Notes for the implementer

- BEFORE editing the default-role files, run `find crates -type f -name '*.rs' | xargs grep -l "default.*role\|owner.*role\|admin.*role" | head -10` to confirm the actual paths — the may-touch list above gives the expected location but the codebase may organise this differently. If the actual path differs, update the brief's may-touch list in your PR description's "sensible deviations" block.
- Capability check pattern: mirror an existing handler that does an RBAC check (e.g. `worker_archive.rs` if it exists, or any handler that uses `Capability::*` + `check_capability`). Do not invent a new pattern.
- The CLI output for `freeze` should print: `"Worker W1 frozen: investigating prompt injection (by alice@acme at 2026-05-19T...)"`. JSON output matches the FreezeRecord shape from sibling 1.
- This brief does NOT add the gateway route — that's sibling 4. The CLI talks to the store directly via AgentDeps, same as other worker CLI commands.

## Out of scope

- Store primitive (sibling 1 — must be MERGED first).
- Runner pre-turn / pre-dispatch checks (sibling 2).
- Gateway HTTP routes (sibling 4).
- Web UI (sibling 5).
- Preemptive SIGKILL — separate brief.
