# Mission load-bearing wire — memory tool: worker-namespace + shared-team addressing actions

**Owner**: claude. **Depends on**: `Memory` tool with `list`/`read`/`write` SHIPPED (verified at `crates/argo-tool/src/tools/memory.rs:43`); `argo-memory::shared_team` module SHIPPED (verified at `crates/argo-memory/src/shared_team.rs`).
**Authored against**: origin/main `b66b1924` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-memory-tool-namespace-action`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 1 item #43 ("Add worker-namespace and shared-team addressing from the memory tool"). The store-side `shared_team` module exists; what's missing is **action plumbing in the per-worker memory tool** — today a worker can `list`/`read`/`write` only its own private memory. Mission Claim 1 ("real teammate") needs workers to read teammates' shared notes and write into team-scoped buckets.

## Scope

Extend the `Memory` tool's action set from `[list, read, write]` to `[list, read, write, list_team, read_team, write_team]`:

- **list_team** — return entries from the worker's shared-team namespace (per `argo-memory::shared_team`). Caller specifies `team_id` (defaults to the worker's primary team from `org_member`).
- **read_team** — read a specific entry from the shared-team namespace.
- **write_team** — write into the shared-team namespace. Capability-gated: the worker MUST have `memory:write_team` capability (NEW capability key — see HARD RULE 3 capability-based security); without it, returns `error_text("missing capability memory:write_team")`.

The existing `list`/`read`/`write` continue to address the worker's private namespace and remain capability-default.

Cross-worker reads (read another worker's private memory) are NOT in scope here — they'd need a separate `read_worker_id` parameter and tighter capability check; file a follow-up brief if requested.

This is NOT in scope: changing the underlying `shared_team` store API; per-tenant cross-team reads; addressing memory by date-range; capability-engine internals.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-tool/src/tools/memory.rs` — narrowly scoped: extend `action` enum from 3 to 6 variants; ADD `team_id: Option<String>` to parsed-args struct; ADD `list_team_action`, `read_team_action`, `write_team_action` async helpers (mirror existing `list_action`/`read_action`/`write_action` line 177-213). Capability check for `write_team` before dispatch. NO change to existing action dispatch for list/read/write. File currently ~260 LOC — keep under 500.
- `crates/argo-tool/src/tools/memory_tests.rs` (sibling, NEW or append) — 6 unit tests: list-team-returns-team-entries; read-team-reads-by-key; write-team-requires-capability; write-team-with-capability-succeeds; list-team-defaults-to-primary-team; read-team-cross-team-rejected.
- `crates/argo-tool/tests/memory_namespace_smoke.rs` (NEW, ≤ 100 LOC) — integration smoke: seed 2 workers in same team, worker A writes via `write_team`, worker B reads via `read_team`, asserts visible.
- `crates/argo-capability/src/keys.rs` (or wherever capability constants live — verify in first 5 min via `rg -l "memory:read\|memory:write" crates/argo-capability/`) — narrowly scoped: ADD `MEMORY_WRITE_TEAM: &str = "memory:write_team"` constant.

## Files you must NOT touch

- `crates/argo-memory/src/shared_team.rs` — store-side API is upstream; reuse only
- `crates/argo-memory/src/layers.rs` — memory layering is upstream; reuse only
- `crates/argo-capability/src/engine.rs` (or however the capability engine dispatches) — engine surface is stable; only constants get added
- `crates/argo-store/migrations/**` — no schema change (shared_team already wired)
- Any non-tool/non-capability crate

## Acceptance criteria

- [ ] Memory tool action enum extends from `[list, read, write]` to `[list, read, write, list_team, read_team, write_team]`
- [ ] `team_id` parameter is optional; defaults to worker's primary team (resolved via `org_member` lookup)
- [ ] `write_team` checks capability `memory:write_team` BEFORE the write; missing capability returns explicit error
- [ ] `list_team`/`read_team` do NOT require write-team capability (read is default-allowed per existing memory semantics)
- [ ] JSON schema for the tool action enum updated to include 3 new variants
- [ ] Tool description text mentions the new actions
- [ ] Existing `list`/`read`/`write` behavior unchanged (today's private-namespace semantics preserved)
- [ ] 6 unit tests + 1 integration smoke
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-tool -p argo-capability` clean
- [ ] `cargo clippy -p argo-tool -p argo-capability --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-tool -p argo-capability` passes (skip pre-existing flakes)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-memory-tool-namespace-action`
- PR title: `tool/memory: add list_team / read_team / write_team actions for shared-team addressing (mission-memory-tool-namespace-action)`

## Notes for the implementer

- The capability gate is load-bearing — without `memory:write_team`, a worker could pollute team-shared state. The default per-worker capability set should NOT include `memory:write_team`; operators grant it explicitly per the policy doc (verify default policy in `argo-policy` or `argo-capability` after writing).
- `team_id: Option<String>` default-resolution: query `org_member` for the worker's `primary_team_id`. If the worker isn't in any team, the action errors with `error_text("worker has no team membership; supply team_id explicitly")`.
- The existing `MemoryFile` / `WiredMemory` types likely have sibling functions in `argo-memory::shared_team`; reuse them rather than reimplementing serialization.
- Mirror the existing `list_action` / `read_action` / `write_action` helper shape exactly — symmetry helps reviewers.
