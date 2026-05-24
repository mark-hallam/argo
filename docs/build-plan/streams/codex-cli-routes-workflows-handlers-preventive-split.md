# Stream codex-cli-routes-workflows-handlers-preventive-split — preventively extract helpers from routes_workflows_handlers.rs (498 LOC)

**Owner**: codex.
**Depends on**: none (parallelizable with the two other LOC-budget split briefs).
**Authored against**: origin/main `885a761d2` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-cli-routes-workflows-handlers-preventive-split`.
🟡 **P1 PREVENTIVE.** `crates/argo-cli/src/handlers/gateway/routes_workflows_handlers.rs` is 498 LOC — two lines from tripping the conductor pre-merge gate. Same prevention play as the other splits this wave.

## Scope

Mechanical extraction: pull the conflict-response helpers + `definition_json` + `decode_index_row` projection out of `routes_workflows_handlers.rs` into a new sibling `routes_workflows_handlers_helpers.rs`. The 7 public route handlers (`list_workflows`, `fetch_head`, `list_versions`, `fetch_version`, `create_version`, `set_head`, `archive_version`, `rollback`) stay in the main file.

Target: main file drops from 498 → ~410-430 LOC; new helpers file lands at ~70-90 LOC. No behavioral change.

This is a **pure mechanical split**. No new logic. Treat as fast-mode candidate.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_workflows_handlers.rs` — remove the extracted helpers; add `use super::routes_workflows_handlers_helpers::*;`. Keep: the doc comment, the 8 `pub(super) async fn` route handlers (`list_workflows` L21, `fetch_head` L61, `list_versions` L76, `fetch_version` L100, `create_version` L125, `set_head` L228, `archive_version` L286, `rollback` L343), and any inline `#[cfg(test)] mod tests` block.
- `NEW: crates/argo-cli/src/handlers/gateway/routes_workflows_handlers_helpers.rs` — destination for the extracted helpers. Move byte-equivalent:
  - `fn collision_conflict_response(vertical: &str, name: &str) -> (&'static str, String)` (L203)
  - `fn conflict_response(...)` (L212)
  - `fn definition_json(def: &WorkflowDefinition) -> Value` (L435)
  - `fn decode_index_row(row: sqlx::sqlite::SqliteRow) -> ArgoResult<WorkflowDefinition>` (L450)
  - Promote visibility to `pub(super)` so the main file's `use super::*` keeps them gateway-scoped.
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly scoped: add `mod routes_workflows_handlers_helpers;` line. No other edits.

## Files you must NOT touch

- Any `routes_*.rs` other than the two named above
- `Cargo.toml` (no new deps)
- Any callsite outside the gateway module

## Acceptance criteria

- [ ] `crates/argo-cli/src/handlers/gateway/routes_workflows_handlers.rs` is < 450 LOC (target ~420)
- [ ] `crates/argo-cli/src/handlers/gateway/routes_workflows_handlers_helpers.rs` exists and is < 150 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli routes_workflows_handlers` passes
- [ ] `bash scripts/codex-pre-push.sh` clean
- [ ] `bash scripts/conductor-pre-merge-gate.sh` clean on the resulting branch
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `codex-cli-routes-workflows-handlers-preventive-split`
- PR title: `cli(gateway): preventively split routes_workflows_handlers helpers to sibling (codex-cli-routes-workflows-handlers-preventive-split)`

## Notes for the implementer

- Same template + shape as the other LOC-budget split briefs this wave.
- If `definition_json` or `decode_index_row` is called from outside the gateway module (cross-`super::`), keep it in the main file and pick a different helper. Acceptance is "< 450 LOC", not "move these specific 4 functions."
